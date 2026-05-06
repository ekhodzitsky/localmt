//! ONNX Runtime adapter boundary for localmt.

use core::fmt;
use std::path::{Path, PathBuf};

use localmt_models::{ModelPack, Verified};

const ONNX_RUNTIME: &str = "onnx-runtime";

/// ONNX model role selected from a verified model pack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtModelRole {
    /// Encoder graph file.
    Encoder,
}

impl OrtModelRole {
    /// { true }
    /// fn as_str(self) -> &'static str
    /// { ret is the stable role id }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Encoder => "encoder",
        }
    }
}

impl fmt::Display for OrtModelRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Verified ONNX session load plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtSessionPlan {
    model_id: String,
    role: OrtModelRole,
    model_path: PathBuf,
}

impl OrtSessionPlan {
    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`, role: OrtModelRole) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when pack declares an ONNX Runtime file for role }
    pub fn from_pack(
        pack: &ModelPack<Verified>,
        role: OrtModelRole,
    ) -> Result<Self, OrtEngineError> {
        let runtime = pack.manifest().runtime().as_str();
        if runtime != ONNX_RUNTIME {
            return Err(OrtEngineError::UnsupportedRuntime(runtime.to_owned()));
        }

        let model_file = pack
            .manifest()
            .files()
            .iter()
            .find(|file| file.kind().as_str() == role.as_str())
            .ok_or(OrtEngineError::MissingModelFile(role))?;

        Ok(Self {
            model_id: pack.manifest().model_id().as_str().to_owned(),
            role,
            model_path: pack.root().join(model_file.path().as_path()),
        })
    }

    /// { true }
    /// fn model_id(&self) -> &str
    /// { ret is the verified model-pack id }
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// { true }
    /// fn role(&self) -> OrtModelRole
    /// { ret is the model role selected for this session }
    pub const fn role(&self) -> OrtModelRole {
        self.role
    }

    /// { true }
    /// fn model_path(&self) -> &Path
    /// { ret is the absolute path to the selected ONNX file }
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }
}

/// ONNX Runtime adapter error.
#[derive(Debug)]
pub enum OrtEngineError {
    /// Model pack targets a different runtime.
    UnsupportedRuntime(String),
    /// Model pack has no ONNX file for the requested role.
    MissingModelFile(OrtModelRole),
    /// Crate was compiled without the `ort-runtime` feature.
    OrtRuntimeFeatureDisabled,
    /// ONNX Runtime failed to create a session.
    Ort(String),
}

impl fmt::Display for OrtEngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRuntime(runtime) => {
                write!(formatter, "unsupported model runtime for ORT: {runtime}")
            }
            Self::MissingModelFile(role) => write!(formatter, "missing ORT model file: {role}"),
            Self::OrtRuntimeFeatureDisabled => {
                formatter.write_str("ort-runtime feature is not enabled")
            }
            Self::Ort(error) => write!(formatter, "ONNX Runtime error: {error}"),
        }
    }
}

impl std::error::Error for OrtEngineError {}

/// ONNX Runtime engine handle.
#[cfg(not(feature = "ort-runtime"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrtEngine;

#[cfg(not(feature = "ort-runtime"))]
impl OrtEngine {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtSessionPlan) -> Result<Self, OrtEngineError>
    /// { ret is Err when compiled without ort-runtime }
    pub fn load(_plan: OrtSessionPlan) -> Result<Self, OrtEngineError> {
        Err(OrtEngineError::OrtRuntimeFeatureDisabled)
    }
}

/// ONNX Runtime engine handle.
#[cfg(feature = "ort-runtime")]
#[derive(Debug)]
pub struct OrtEngine {
    plan: OrtSessionPlan,
    session: ort::session::Session,
}

#[cfg(feature = "ort-runtime")]
impl OrtEngine {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtSessionPlan) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when ONNX Runtime creates a session from plan.model_path() }
    pub fn load(plan: OrtSessionPlan) -> Result<Self, OrtEngineError> {
        let session = ort::session::Session::builder()
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?
            .commit_from_file(plan.model_path())
            .map_err(|source| OrtEngineError::Ort(source.to_string()))?;

        Ok(Self { plan, session })
    }

    /// { self was loaded successfully }
    /// fn plan(&self) -> &OrtSessionPlan
    /// { ret is the verified session load plan }
    pub const fn plan(&self) -> &OrtSessionPlan {
        &self.plan
    }

    /// { self was loaded successfully }
    /// fn input_count(&self) -> usize
    /// { ret is the number of ONNX graph inputs }
    pub fn input_count(&self) -> usize {
        self.session.inputs().len()
    }

    /// { self was loaded successfully }
    /// fn output_count(&self) -> usize
    /// { ret is the number of ONNX graph outputs }
    pub fn output_count(&self) -> usize {
        self.session.outputs().len()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_models::{Discovered, ModelPack};

    #[cfg(not(feature = "ort-runtime"))]
    use crate::OrtEngine;
    use crate::{OrtEngineError, OrtModelRole, OrtSessionPlan};

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn session_plan_selects_encoder_file_from_verified_pack()
    -> Result<(), Box<dyn std::error::Error>> {
        let pack =
            ModelPack::<Discovered>::discover(create_pack(true, "onnx-runtime")?)?.verify()?;

        let plan = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder)?;

        assert_eq!(plan.model_id(), "m2m100-418m-int8");
        assert_eq!(plan.role(), OrtModelRole::Encoder);
        assert_eq!(
            plan.model_path(),
            pack.root().join("encoder.onnx").as_path()
        );
        Ok(())
    }

    #[test]
    fn session_plan_rejects_missing_encoder_file() -> Result<(), Box<dyn std::error::Error>> {
        let pack =
            ModelPack::<Discovered>::discover(create_pack(false, "onnx-runtime")?)?.verify()?;

        let error = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder);

        assert!(matches!(
            error,
            Err(OrtEngineError::MissingModelFile(OrtModelRole::Encoder))
        ));
        Ok(())
    }

    #[test]
    fn session_plan_rejects_non_ort_runtime() -> Result<(), Box<dyn std::error::Error>> {
        let pack = ModelPack::<Discovered>::discover(create_pack(true, "candle")?)?.verify()?;

        let error = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder);

        assert!(matches!(
            error,
            Err(OrtEngineError::UnsupportedRuntime(ref runtime)) if runtime == "candle"
        ));
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn load_returns_feature_disabled_without_ort_runtime() -> Result<(), Box<dyn std::error::Error>>
    {
        let pack =
            ModelPack::<Discovered>::discover(create_pack(true, "onnx-runtime")?)?.verify()?;
        let plan = OrtSessionPlan::from_pack(&pack, OrtModelRole::Encoder)?;

        let error = OrtEngine::load(plan);

        assert!(matches!(
            error,
            Err(OrtEngineError::OrtRuntimeFeatureDisabled)
        ));
        Ok(())
    }

    fn create_pack(
        include_encoder: bool,
        runtime: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(
            root.join("manifest.json"),
            manifest_json(include_encoder, runtime),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-engine-ort-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn manifest_json(include_encoder: bool, runtime: &str) -> String {
        let files = if include_encoder {
            format!(
                r#"{{
      "path": "encoder.onnx",
      "kind": "encoder",
      "sha256": "{ENCODER_SHA256}"
    }},
    {{
      "path": "tokenizer.json",
      "kind": "tokenizer",
      "sha256": "{TOKENIZER_SHA256}"
    }}"#
            )
        } else {
            format!(
                r#"{{
      "path": "tokenizer.json",
      "kind": "tokenizer",
      "sha256": "{TOKENIZER_SHA256}"
    }}"#
            )
        };

        format!(
            r#"{{
  "schema_version": 0,
  "model_id": "m2m100-418m-int8",
  "version": "0.1.0",
  "architecture": "m2m100",
  "runtime": "{runtime}",
  "license": "MIT",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
    {files}
  ]
}}"#
        )
    }
}
