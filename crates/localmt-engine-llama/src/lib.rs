//! llama.cpp / GGUF adapter boundary for localmt.

use core::{fmt, num::NonZeroUsize};
use std::fs;
use std::path::{Path, PathBuf};

use localmt_core::{Language, TranslateRequest, Translation};
use localmt_engine::{TranslationError, TranslatorEngine};
use localmt_models::{GgufModelAssetPlan, ModelId};
use serde::Deserialize;

const DEFAULT_CONTEXT_TOKENS: usize = 2048;
const DEFAULT_CPU_THREADS: usize = 1;
const DEFAULT_TEMPERATURE: f32 = 0.0;
const DISABLED_RUNTIME: &str = "llama.cpp runtime is not enabled";

/// Verified llama.cpp model-load plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlamaModelPlan {
    model_id: ModelId,
    model_path: PathBuf,
    chat_template_path: Option<PathBuf>,
    runtime_config_path: Option<PathBuf>,
}

impl LlamaModelPlan {
    /// { assets were built from a verified GGUF model pack }
    /// fn from_assets(assets: GgufModelAssetPlan) -> Result<Self, LlamaEngineError>
    /// { ret preserves root-qualified GGUF and optional llama metadata paths }
    pub fn from_assets(assets: GgufModelAssetPlan) -> Result<Self, LlamaEngineError> {
        Ok(Self {
            model_id: assets.model_id().clone(),
            model_path: assets.model_path().to_path_buf(),
            chat_template_path: assets.chat_template_path().map(Path::to_path_buf),
            runtime_config_path: assets.runtime_config_path().map(Path::to_path_buf),
        })
    }

    /// { true }
    /// fn model_id(&self) -> &ModelId
    /// { ret is the verified model-pack id }
    pub const fn model_id(&self) -> &ModelId {
        &self.model_id
    }

    /// { true }
    /// fn model_path(&self) -> &Path
    /// { ret is the root-qualified GGUF model path }
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// { true }
    /// fn chat_template_path(&self) -> `Option<&Path>`
    /// { ret is Some only when a verified chat_template file was declared }
    pub fn chat_template_path(&self) -> Option<&Path> {
        self.chat_template_path.as_deref()
    }

    /// { true }
    /// fn runtime_config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when a verified llama_runtime_config file was declared }
    pub fn runtime_config_path(&self) -> Option<&Path> {
        self.runtime_config_path.as_deref()
    }

    /// { self may declare a runtime config path }
    /// fn parse_runtime_config(&self) -> Result<LlamaRuntimeConfig, LlamaEngineError>
    /// { ret is Ok with parsed config or defaults when no config path exists }
    pub fn parse_runtime_config(&self) -> Result<LlamaRuntimeConfig, LlamaEngineError> {
        match self.runtime_config_path() {
            Some(path) => LlamaRuntimeConfig::from_json_file(path),
            None => Ok(LlamaRuntimeConfig::default()),
        }
    }
}

/// llama.cpp runtime configuration owned by localmt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LlamaRuntimeConfig {
    context_tokens: NonZeroUsize,
    cpu_threads: NonZeroUsize,
    temperature: f32,
}

impl LlamaRuntimeConfig {
    /// { contents is a JSON runtime-config document }
    /// fn from_json_str(contents: &str) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when optional numeric fields are valid }
    pub fn from_json_str(contents: &str) -> Result<Self, LlamaEngineError> {
        let raw = serde_json::from_str::<RawLlamaRuntimeConfig>(contents).map_err(|source| {
            LlamaEngineError::ParseRuntimeConfig {
                reason: source.to_string(),
            }
        })?;

        Self::from_raw(raw)
    }

    /// { path points to a readable JSON runtime-config file }
    /// fn from_json_file(path: &Path) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when file contents parse as LlamaRuntimeConfig }
    pub fn from_json_file(path: &Path) -> Result<Self, LlamaEngineError> {
        let contents =
            fs::read_to_string(path).map_err(|source| LlamaEngineError::ReadRuntimeConfig {
                path: path.to_path_buf(),
                reason: source.to_string(),
            })?;

        Self::from_json_str(&contents)
    }

    /// { true }
    /// fn context_tokens(&self) -> NonZeroUsize
    /// { ret is the configured llama context window in tokens }
    pub const fn context_tokens(&self) -> NonZeroUsize {
        self.context_tokens
    }

    /// { true }
    /// fn cpu_threads(&self) -> NonZeroUsize
    /// { ret is the configured CPU thread count }
    pub const fn cpu_threads(&self) -> NonZeroUsize {
        self.cpu_threads
    }

    /// { true }
    /// fn temperature(&self) -> f32
    /// { ret is the configured sampling temperature }
    pub const fn temperature(&self) -> f32 {
        self.temperature
    }

    fn from_raw(raw: RawLlamaRuntimeConfig) -> Result<Self, LlamaEngineError> {
        Ok(Self {
            context_tokens: nonzero_or_default(
                "context_tokens",
                raw.context_tokens,
                DEFAULT_CONTEXT_TOKENS,
            )?,
            cpu_threads: nonzero_or_default("cpu_threads", raw.cpu_threads, DEFAULT_CPU_THREADS)?,
            temperature: valid_temperature(raw.temperature.unwrap_or(DEFAULT_TEMPERATURE))?,
        })
    }
}

impl Default for LlamaRuntimeConfig {
    fn default() -> Self {
        Self {
            context_tokens: nonzero_default(DEFAULT_CONTEXT_TOKENS),
            cpu_threads: nonzero_default(DEFAULT_CPU_THREADS),
            temperature: DEFAULT_TEMPERATURE,
        }
    }
}

/// Translation prompt text passed to the llama.cpp completion boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlamaTranslationPrompt(String);

impl LlamaTranslationPrompt {
    /// { request has a valid language pair and non-empty text }
    /// fn from_request(request: &TranslateRequest) -> Self
    /// { ret is the stable HY-MT segment prompt for request.target() and request.text() }
    pub fn from_request(request: &TranslateRequest) -> Self {
        let target = language_label(request.pair().target());
        Self(format!(
            "Translate the following segment into {target}, without additional explanation.\n\n{}",
            request.text().as_str()
        ))
    }

    /// { true }
    /// fn as_str(&self) -> &str
    /// { ret is the prompt text }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// llama.cpp translator handle.
#[derive(Clone, Debug)]
pub struct LlamaTranslator;

impl LlamaTranslator {
    /// { plan was built from verified GGUF assets }
    /// fn load(plan: LlamaModelPlan) -> Result<Self, LlamaEngineError>
    /// { ret is Ok only when the llama runtime feature can load the model }
    pub fn load(_plan: LlamaModelPlan) -> Result<Self, LlamaEngineError> {
        Err(LlamaEngineError::RuntimeDisabled)
    }

    /// { true }
    /// fn disabled_for_tests() -> Self
    /// { ret is a translator handle that reports disabled runtime on translate }
    #[cfg(test)]
    pub(crate) const fn disabled_for_tests() -> Self {
        Self
    }
}

impl TranslatorEngine for LlamaTranslator {
    fn translate(&self, _request: &TranslateRequest) -> Result<Translation, TranslationError> {
        Err(TranslationError::EngineUnavailable(
            DISABLED_RUNTIME.to_owned(),
        ))
    }
}

/// Error returned by the llama.cpp / GGUF adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlamaEngineError {
    /// Runtime feature or native backend is not available in this build.
    RuntimeDisabled,
    /// Runtime-config JSON could not be read.
    ReadRuntimeConfig {
        /// Config path.
        path: PathBuf,
        /// Stable read error reason.
        reason: String,
    },
    /// Runtime-config JSON could not be parsed.
    ParseRuntimeConfig {
        /// Stable parse error reason.
        reason: String,
    },
    /// Runtime-config numeric field is invalid.
    InvalidRuntimeConfig {
        /// Config field name.
        field: &'static str,
        /// Stable validation reason.
        reason: &'static str,
    },
}

impl fmt::Display for LlamaEngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeDisabled => formatter.write_str(DISABLED_RUNTIME),
            Self::ReadRuntimeConfig { path, reason } => {
                write!(
                    formatter,
                    "failed to read llama runtime config {}: {reason}",
                    path.display()
                )
            }
            Self::ParseRuntimeConfig { reason } => {
                write!(formatter, "failed to parse llama runtime config: {reason}")
            }
            Self::InvalidRuntimeConfig { field, reason } => {
                write!(formatter, "invalid llama runtime config {field}: {reason}")
            }
        }
    }
}

impl std::error::Error for LlamaEngineError {}

#[derive(Deserialize)]
struct RawLlamaRuntimeConfig {
    context_tokens: Option<usize>,
    cpu_threads: Option<usize>,
    temperature: Option<f32>,
}

fn nonzero_or_default(
    field: &'static str,
    value: Option<usize>,
    default: usize,
) -> Result<NonZeroUsize, LlamaEngineError> {
    match value {
        Some(value) => NonZeroUsize::new(value).ok_or(LlamaEngineError::InvalidRuntimeConfig {
            field,
            reason: "must be greater than zero",
        }),
        None => Ok(nonzero_default(default)),
    }
}

fn nonzero_default(value: usize) -> NonZeroUsize {
    if let Some(value) = NonZeroUsize::new(value) {
        return value;
    }

    NonZeroUsize::MIN
}

fn valid_temperature(value: f32) -> Result<f32, LlamaEngineError> {
    if value.is_finite() && value >= 0.0 {
        return Ok(value);
    }

    Err(LlamaEngineError::InvalidRuntimeConfig {
        field: "temperature",
        reason: "must be finite and non-negative",
    })
}

const fn language_label(language: Language) -> &'static str {
    match language {
        Language::English => "English",
        Language::Russian => "Russian",
        Language::Thai => "Thai",
        Language::Vietnamese => "Vietnamese",
        Language::Japanese => "Japanese",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_core::{Language, NonEmptyText, TranslateRequest};
    use localmt_engine::TranslatorEngine;
    use localmt_models::{Discovered, GgufModelAssetPlan, ModelPack};

    use crate::{
        LlamaEngineError, LlamaModelPlan, LlamaRuntimeConfig, LlamaTranslationPrompt,
        LlamaTranslator,
    };

    const GGUF_MODEL_SHA256: &str =
        "a561ab462e9c80d55c2ca56fd846a30001aac1dea00c99d7e13e2b1320b88024";
    const CHAT_TEMPLATE_SHA256: &str =
        "90d69c78fb9ef942c8ce0a0d88a7455572ada06044bc1516473472664ddd013b";
    const LLAMA_RUNTIME_CONFIG_SHA256: &str =
        "4b825731b61ba8b6858381a904d807b1ae1f24bd163cc485d8082cd2198e0d0e";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn model_plan_preserves_verified_gguf_assets() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_gguf_pack()?;
        let assets = verified_gguf_assets(&root)?;

        let plan = LlamaModelPlan::from_assets(assets)?;

        assert_eq!(plan.model_id().as_str(), "hymt-1.25bit");
        assert_eq!(plan.model_path(), root.join("hymt.gguf"));
        assert_eq!(
            plan.chat_template_path(),
            Some(root.join("chat-template.jinja")).as_deref()
        );
        assert_eq!(
            plan.runtime_config_path(),
            Some(root.join("llama-runtime.json")).as_deref()
        );
        Ok(())
    }

    #[test]
    fn runtime_config_parses_optional_json_limits() -> Result<(), Box<dyn std::error::Error>> {
        let config = LlamaRuntimeConfig::from_json_str(
            r#"{"context_tokens": 2048, "cpu_threads": 4, "temperature": 0.0}"#,
        )?;

        assert_eq!(config.context_tokens().get(), 2048);
        assert_eq!(config.cpu_threads().get(), 4);
        assert_eq!(config.temperature(), 0.0);
        Ok(())
    }

    #[test]
    fn runtime_config_rejects_invalid_numeric_limits() {
        let zero_threads =
            LlamaRuntimeConfig::from_json_str(r#"{"context_tokens": 2048, "cpu_threads": 0}"#);
        let negative_temperature = LlamaRuntimeConfig::from_json_str(r#"{"temperature": -0.1}"#);

        assert!(matches!(
            zero_threads,
            Err(LlamaEngineError::InvalidRuntimeConfig {
                field: "cpu_threads",
                ..
            })
        ));
        assert!(matches!(
            negative_temperature,
            Err(LlamaEngineError::InvalidRuntimeConfig {
                field: "temperature",
                ..
            })
        ));
    }

    #[test]
    fn prompt_uses_owned_hymt_template_and_language_label() -> Result<(), Box<dyn std::error::Error>>
    {
        let request = TranslateRequest::new(
            Language::English,
            Language::Russian,
            NonEmptyText::new("hello offline".to_owned())?,
        )?;

        let prompt = LlamaTranslationPrompt::from_request(&request);

        assert_eq!(
            prompt.as_str(),
            "Translate the following segment into Russian, without additional explanation.\n\nhello offline"
        );
        Ok(())
    }

    #[test]
    fn translator_load_reports_disabled_runtime_without_feature()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_gguf_pack()?;
        let plan = LlamaModelPlan::from_assets(verified_gguf_assets(&root)?)?;

        let error = LlamaTranslator::load(plan).err();

        assert!(matches!(error, Some(LlamaEngineError::RuntimeDisabled)));
        Ok(())
    }

    #[test]
    fn disabled_translator_still_implements_engine_trait() -> Result<(), Box<dyn std::error::Error>>
    {
        let request = TranslateRequest::new(
            Language::English,
            Language::Japanese,
            NonEmptyText::new("hello offline".to_owned())?,
        )?;
        let translator = LlamaTranslator::disabled_for_tests();

        let error = translator.translate(&request).err();

        assert!(matches!(
            error,
            Some(localmt_engine::TranslationError::EngineUnavailable(_))
        ));
        Ok(())
    }

    fn verified_gguf_assets(
        root: &std::path::Path,
    ) -> Result<GgufModelAssetPlan, Box<dyn std::error::Error>> {
        let pack = ModelPack::<Discovered>::discover(root)?.verify()?;
        Ok(GgufModelAssetPlan::from_pack(&pack)?)
    }

    fn create_gguf_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("hymt.gguf"), "hymt gguf\n")?;
        fs::write(root.join("chat-template.jinja"), "template\n")?;
        fs::write(root.join("llama-runtime.json"), "llama runtime\n")?;
        fs::write(
            root.join("manifest.json"),
            format!(
                r#"{{
  "schema_version": 0,
  "model_id": "hymt-1.25bit",
  "version": "0.1.0",
  "architecture": "hunyuan-dense",
  "runtime": "llama.cpp",
  "license": "Tencent Hunyuan Community",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
    {{ "path": "hymt.gguf", "kind": "gguf_model", "sha256": "{GGUF_MODEL_SHA256}" }},
    {{ "path": "chat-template.jinja", "kind": "chat_template", "sha256": "{CHAT_TEMPLATE_SHA256}" }},
    {{ "path": "llama-runtime.json", "kind": "llama_runtime_config", "sha256": "{LLAMA_RUNTIME_CONFIG_SHA256}" }}
  ]
}}"#
            ),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-engine-llama-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }
}
