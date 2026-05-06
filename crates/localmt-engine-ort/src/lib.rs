//! ONNX Runtime adapter boundary for localmt.

use core::fmt;
use std::path::{Path, PathBuf};

use localmt_models::{ModelFileRole, ModelPack, Verified};
use localmt_pipeline::{GeneratorAssetPlan, TokenGeneratorError};

const ONNX_RUNTIME: &str = "onnx-runtime";

/// ONNX model role selected from a verified model pack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtModelRole {
    /// Encoder graph file.
    Encoder,
    /// Decoder graph file.
    Decoder,
    /// Decoder graph file with cached past-key-values.
    DecoderWithPast,
}

impl OrtModelRole {
    /// { true }
    /// fn as_str(self) -> &'static str
    /// { ret is the stable role id }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Encoder => "encoder",
            Self::Decoder => "decoder",
            Self::DecoderWithPast => "decoder_with_past",
        }
    }

    /// { true }
    /// fn model_file_role(self) -> ModelFileRole
    /// { ret is the model-pack file role required for this ORT model role }
    pub const fn model_file_role(self) -> ModelFileRole {
        match self {
            Self::Encoder => ModelFileRole::Encoder,
            Self::Decoder => ModelFileRole::Decoder,
            Self::DecoderWithPast => ModelFileRole::DecoderWithPast,
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
    fn new(model_id: String, role: OrtModelRole, model_path: PathBuf) -> Self {
        Self {
            model_id,
            role,
            model_path,
        }
    }

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

        let model_path = pack
            .file_path(role.model_file_role())
            .ok_or(OrtEngineError::MissingModelFile(role))?;

        Ok(Self::new(
            pack.manifest().model_id().as_str().to_owned(),
            role,
            model_path,
        ))
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

/// Verified ONNX generator load plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGeneratorPlan {
    model_id: String,
    encoder: OrtSessionPlan,
    decoder: OrtSessionPlan,
    decoder_with_past: Option<OrtSessionPlan>,
    generation_config_path: Option<PathBuf>,
}

impl OrtGeneratorPlan {
    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when pack targets ONNX Runtime and generator assets are present }
    pub fn from_pack(pack: &ModelPack<Verified>) -> Result<Self, OrtEngineError> {
        let runtime = pack.manifest().runtime().as_str();
        if runtime != ONNX_RUNTIME {
            return Err(OrtEngineError::UnsupportedRuntime(runtime.to_owned()));
        }

        let assets = GeneratorAssetPlan::from_pack(pack).map_err(OrtEngineError::GeneratorAsset)?;
        let model_id = pack.manifest().model_id().as_str().to_owned();
        let encoder = OrtSessionPlan::new(
            model_id.clone(),
            OrtModelRole::Encoder,
            assets.encoder_path().to_path_buf(),
        );
        let decoder = OrtSessionPlan::new(
            model_id.clone(),
            OrtModelRole::Decoder,
            assets.decoder_path().to_path_buf(),
        );
        let decoder_with_past = assets.decoder_with_past_path().map(|path| {
            OrtSessionPlan::new(
                model_id.clone(),
                OrtModelRole::DecoderWithPast,
                path.to_path_buf(),
            )
        });

        Ok(Self {
            model_id,
            encoder,
            decoder,
            decoder_with_past,
            generation_config_path: assets.generation_config_path().map(Path::to_path_buf),
        })
    }

    /// { true }
    /// fn model_id(&self) -> &str
    /// { ret is the verified model-pack id }
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// { true }
    /// fn encoder(&self) -> &OrtSessionPlan
    /// { ret is the encoder session plan }
    pub const fn encoder(&self) -> &OrtSessionPlan {
        &self.encoder
    }

    /// { true }
    /// fn decoder(&self) -> &OrtSessionPlan
    /// { ret is the decoder session plan }
    pub const fn decoder(&self) -> &OrtSessionPlan {
        &self.decoder
    }

    /// { true }
    /// fn decoder_with_past(&self) -> `Option<&OrtSessionPlan>`
    /// { ret is Some only when the pack declares decoder_with_past }
    pub const fn decoder_with_past(&self) -> Option<&OrtSessionPlan> {
        self.decoder_with_past.as_ref()
    }

    /// { true }
    /// fn generation_config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declares generation_config }
    pub fn generation_config_path(&self) -> Option<&Path> {
        self.generation_config_path.as_deref()
    }
}

/// ONNX Runtime adapter error.
#[derive(Debug)]
pub enum OrtEngineError {
    /// Model pack targets a different runtime.
    UnsupportedRuntime(String),
    /// Model pack has no ONNX file for the requested role.
    MissingModelFile(OrtModelRole),
    /// Generator asset planning failed before ORT session planning.
    GeneratorAsset(TokenGeneratorError),
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
            Self::GeneratorAsset(error) => write!(formatter, "{error}"),
            Self::OrtRuntimeFeatureDisabled => {
                formatter.write_str("ort-runtime feature is not enabled")
            }
            Self::Ort(error) => write!(formatter, "ONNX Runtime error: {error}"),
        }
    }
}

impl std::error::Error for OrtEngineError {}

/// ONNX Runtime token generator handle.
#[cfg(not(feature = "ort-runtime"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrtTokenGenerator;

#[cfg(not(feature = "ort-runtime"))]
impl OrtTokenGenerator {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError>
    /// { ret is Err when compiled without ort-runtime }
    pub fn load(_plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError> {
        Err(OrtEngineError::OrtRuntimeFeatureDisabled)
    }
}

/// ONNX Runtime token generator handle.
#[cfg(feature = "ort-runtime")]
#[derive(Debug)]
pub struct OrtTokenGenerator {
    plan: OrtGeneratorPlan,
    encoder: OrtEngine,
    decoder: OrtEngine,
    decoder_with_past: Option<OrtEngine>,
}

#[cfg(feature = "ort-runtime")]
impl OrtTokenGenerator {
    /// { plan was built from a verified ONNX Runtime model pack }
    /// fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError>
    /// { ret is Ok only when ONNX Runtime loads required generator sessions }
    pub fn load(plan: OrtGeneratorPlan) -> Result<Self, OrtEngineError> {
        let encoder = OrtEngine::load(plan.encoder().clone())?;
        let decoder = OrtEngine::load(plan.decoder().clone())?;
        let decoder_with_past = plan
            .decoder_with_past()
            .cloned()
            .map(OrtEngine::load)
            .transpose()?;

        Ok(Self {
            plan,
            encoder,
            decoder,
            decoder_with_past,
        })
    }

    /// { self was loaded successfully }
    /// fn plan(&self) -> &OrtGeneratorPlan
    /// { ret is the verified generator plan used for loading }
    pub const fn plan(&self) -> &OrtGeneratorPlan {
        &self.plan
    }

    /// { self was loaded successfully }
    /// fn encoder(&self) -> &OrtEngine
    /// { ret is the loaded encoder session wrapper }
    pub const fn encoder(&self) -> &OrtEngine {
        &self.encoder
    }

    /// { self was loaded successfully }
    /// fn decoder(&self) -> &OrtEngine
    /// { ret is the loaded decoder session wrapper }
    pub const fn decoder(&self) -> &OrtEngine {
        &self.decoder
    }

    /// { self was loaded successfully }
    /// fn decoder_with_past(&self) -> `Option<&OrtEngine>`
    /// { ret is Some only when the cached decoder session was loaded }
    pub const fn decoder_with_past(&self) -> Option<&OrtEngine> {
        self.decoder_with_past.as_ref()
    }
}

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

    use localmt_models::{Discovered, ModelFileRole, ModelPack};
    use localmt_pipeline::TokenGeneratorError;

    #[cfg(not(feature = "ort-runtime"))]
    use crate::{OrtEngine, OrtTokenGenerator};
    use crate::{OrtEngineError, OrtGeneratorPlan, OrtModelRole, OrtSessionPlan};

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const DECODER_WITH_PAST_SHA256: &str =
        "ef37d12b277fe98960af949b560ded559157bf42d2eea244dcfc64ac08da666c";
    const GENERATION_CONFIG_SHA256: &str =
        "75f35767c145e896ca56dba402cc97c3879445dace669c745ecea5fc0c9552b6";
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

    #[test]
    fn generator_plan_selects_required_and_optional_assets()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
                (
                    "decoder-with-past.onnx",
                    "decoder_with_past",
                    DECODER_WITH_PAST_SHA256,
                    "decoder-with-past\n",
                ),
                (
                    "generation.json",
                    "generation_config",
                    GENERATION_CONFIG_SHA256,
                    "generation\n",
                ),
            ],
        )?;
        let decoder_with_past_path = root.join("decoder-with-past.onnx");
        let generation_config_path = root.join("generation.json");
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        assert_eq!(plan.model_id(), "m2m100-418m-int8");
        assert_eq!(plan.encoder().role(), OrtModelRole::Encoder);
        assert_eq!(plan.decoder().role(), OrtModelRole::Decoder);
        assert_eq!(
            plan.decoder_with_past().map(OrtSessionPlan::role),
            Some(OrtModelRole::DecoderWithPast)
        );
        assert_eq!(
            plan.decoder_with_past().map(OrtSessionPlan::model_path),
            Some(decoder_with_past_path.as_path())
        );
        assert_eq!(
            plan.generation_config_path(),
            Some(generation_config_path.as_path())
        );
        Ok(())
    }

    #[test]
    fn generator_plan_rejects_non_ort_runtime() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "candle",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OrtGeneratorPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(OrtEngineError::UnsupportedRuntime(ref runtime)) if runtime == "candle"
        ));
        Ok(())
    }

    #[test]
    fn generator_plan_rejects_missing_decoder_asset() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n")],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OrtGeneratorPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(OrtEngineError::GeneratorAsset(
                TokenGeneratorError::MissingGeneratorAsset(ModelFileRole::Decoder)
            ))
        ));
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn token_generator_load_returns_feature_disabled_without_ort_runtime()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack_with_files(
            "onnx-runtime",
            &[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            ],
        )?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OrtGeneratorPlan::from_pack(&pack)?;

        let error = OrtTokenGenerator::load(plan);

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

    fn create_pack_with_files(
        runtime: &str,
        files: &[(&str, &str, &str, &str)],
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        for (path, _kind, _sha256, contents) in files {
            fs::write(root.join(path), contents)?;
        }
        fs::write(
            root.join("manifest.json"),
            manifest_json_for_files(runtime, files),
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

    fn manifest_json_for_files(runtime: &str, files: &[(&str, &str, &str, &str)]) -> String {
        let file_json = files
            .iter()
            .map(|(path, kind, sha256, _contents)| {
                format!(r#"    {{ "path": "{path}", "kind": "{kind}", "sha256": "{sha256}" }}"#)
            })
            .collect::<Vec<_>>()
            .join(",\n");

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
{file_json}
  ]
}}"#
        )
    }
}
