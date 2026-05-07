//! Public facade for the localmt translation library.

use core::fmt;
use std::path::{Path, PathBuf};

pub use localmt_bench::{BenchReport, BenchmarkError, DeviceProfile, MockBenchmarkRunner};
pub use localmt_core::{
    Language, LanguageCodeError, LanguagePair, LanguagePairError, MAX_TEXT_CHARS, NonEmptyText,
    TextError, TranslateRequest, Translation,
};
pub use localmt_engine::{MockEngine, TranslationError, TranslatorEngine};
#[cfg(feature = "ort-runtime")]
pub use localmt_engine_ort::OrtEngineSlot;
pub use localmt_engine_ort::{
    OrtDecoderLogits, OrtDecoderLogitsError, OrtEngine, OrtEngineError, OrtFloatTensorOutput,
    OrtGenerationInputs, OrtGenerationLoop, OrtGenerationLoopError, OrtGenerationState,
    OrtGenerationStateError, OrtGenerationStep, OrtGenerationStepError, OrtGenerationTensorInputs,
    OrtGeneratorPlan, OrtGeneratorRuntimeConfig, OrtI64TensorInput, OrtIoConfig, OrtIoConfigError,
    OrtIoConfigParseError, OrtModelRole, OrtSessionPlan, OrtTokenGenerator,
};
pub use localmt_models::{
    Discovered, ModelArchitecture, ModelFile, ModelFileKind, ModelFileRole, ModelId, ModelLicense,
    ModelManifest, ModelPack, ModelPackError, ModelPackVersion, ModelRelativePath, ModelRuntime,
    Sha256Digest, Verified,
};
pub use localmt_pipeline::{
    DEFAULT_MAX_NEW_TOKENS, GenerationConfig, GenerationConfigError, GenerationConfigParseError,
    GenerationSpecialTokenRole, GenerationSpecialTokens, GeneratorAssetPlan, LanguageTokenIds,
    MaxNewTokens, MockTokenGenerator, PipelineError, TokenGenerator, TokenGeneratorError,
    TranslationPipeline,
};
#[cfg(feature = "hf-tokenizers")]
pub use localmt_tokenizer::HfTokenizer;
pub use localmt_tokenizer::{
    MAX_TOKENS, MockTokenizer, TokenId, TokenSequence, TokenizerAssetPlan, TokenizerEngine,
    TokenizerError, TokenizerInput, TokenizerOutput,
};

/// Facade-level plan for constructing a future offline translator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineTranslatorPlan {
    tokenizer: TokenizerAssetPlan,
    generator: OrtGeneratorPlan,
}

impl OfflineTranslatorPlan {
    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`) -> Result<Self, OfflineTranslatorPlanError>
    /// { ret is Ok only when tokenizer and ORT generator plans can be built }
    pub fn from_pack(pack: &ModelPack<Verified>) -> Result<Self, OfflineTranslatorPlanError> {
        let tokenizer =
            TokenizerAssetPlan::from_pack(pack).map_err(OfflineTranslatorPlanError::Tokenizer)?;
        let generator =
            OrtGeneratorPlan::from_pack(pack).map_err(OfflineTranslatorPlanError::Generator)?;

        Ok(Self {
            tokenizer,
            generator,
        })
    }

    /// { true }
    /// fn tokenizer(&self) -> &TokenizerAssetPlan
    /// { ret is the verified tokenizer asset plan }
    pub const fn tokenizer(&self) -> &TokenizerAssetPlan {
        &self.tokenizer
    }

    /// { true }
    /// fn generator(&self) -> &OrtGeneratorPlan
    /// { ret is the verified ORT generator plan }
    pub const fn generator(&self) -> &OrtGeneratorPlan {
        &self.generator
    }

    /// { self was built from a verified model pack }
    /// fn parse_generation_config(&self) -> `Result<Option<GenerationConfig>, OfflineTranslatorPlanError>`
    /// { ret is Ok(Some) only when a declared generation_config parses and validates }
    pub fn parse_generation_config(
        &self,
    ) -> Result<Option<GenerationConfig>, OfflineTranslatorPlanError> {
        self.generator
            .parse_generation_config()
            .map_err(OfflineTranslatorPlanError::Generator)
    }

    /// { self was built from a verified model pack }
    /// fn parse_ort_io_config_status(&self) -> Result<OrtIoConfigStatus, OfflineTranslatorPlanError>
    /// { ret distinguishes absent, missing, and parsed ORT I/O config states }
    pub fn parse_ort_io_config_status(
        &self,
    ) -> Result<OrtIoConfigStatus, OfflineTranslatorPlanError> {
        match self.generator.parse_ort_io_config() {
            Ok(Some(config)) => Ok(OrtIoConfigStatus::Parsed(Box::new(config))),
            Ok(None) => Ok(OrtIoConfigStatus::Absent),
            Err(OrtEngineError::IoConfig(OrtIoConfigParseError::MissingOrtIoConfig)) => {
                Ok(OrtIoConfigStatus::Missing)
            }
            Err(error) => Err(OfflineTranslatorPlanError::Generator(error)),
        }
    }
}

/// Facade-level status for optional ORT I/O tensor-name config.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrtIoConfigStatus {
    /// Model pack declares no config file.
    Absent,
    /// Model pack declares config but it has no ort_io object.
    Missing,
    /// Model pack declares valid ORT I/O tensor names.
    Parsed(Box<OrtIoConfig>),
}

/// Prepared no-inference assets for constructing a future offline translator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineTranslatorAssets {
    plan: OfflineTranslatorPlan,
    generation_config: Option<GenerationConfig>,
    ort_io_config_status: OrtIoConfigStatus,
}

impl OfflineTranslatorAssets {
    /// { path points to a model-pack directory candidate }
    /// fn from_model_pack_path(path: impl `AsRef<Path>`) -> Result<Self, OfflineTranslatorAssetsError>
    /// { ret is Ok only when discovery, verification, planning, and config parsing succeed }
    pub fn from_model_pack_path(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, OfflineTranslatorAssetsError> {
        let pack = ModelPack::<Discovered>::discover(path)
            .and_then(ModelPack::verify)
            .map_err(OfflineTranslatorAssetsError::ModelPack)?;

        Self::from_pack(&pack).map_err(OfflineTranslatorAssetsError::Plan)
    }

    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`) -> Result<Self, OfflineTranslatorPlanError>
    /// { ret is Ok only when planning and optional generation-config parsing succeed }
    pub fn from_pack(pack: &ModelPack<Verified>) -> Result<Self, OfflineTranslatorPlanError> {
        Self::from_plan(OfflineTranslatorPlan::from_pack(pack)?)
    }

    /// { plan was built from a verified model pack }
    /// fn from_plan(plan: OfflineTranslatorPlan) -> Result<Self, OfflineTranslatorPlanError>
    /// { ret is Ok only when optional config parsing succeeds }
    pub fn from_plan(plan: OfflineTranslatorPlan) -> Result<Self, OfflineTranslatorPlanError> {
        let generation_config = plan.parse_generation_config()?;
        let ort_io_config_status = plan.parse_ort_io_config_status()?;

        Ok(Self {
            plan,
            generation_config,
            ort_io_config_status,
        })
    }

    /// { true }
    /// fn plan(&self) -> &OfflineTranslatorPlan
    /// { ret is the verified tokenizer and ORT generator path plan }
    pub const fn plan(&self) -> &OfflineTranslatorPlan {
        &self.plan
    }

    /// { true }
    /// fn generation_config(&self) -> `Option<GenerationConfig>`
    /// { ret is Some only when the model pack declared a valid generation_config }
    pub const fn generation_config(&self) -> Option<GenerationConfig> {
        self.generation_config
    }

    /// { true }
    /// fn ort_io_config_status(&self) -> &OrtIoConfigStatus
    /// { ret is the ORT I/O tensor-name readiness status }
    pub const fn ort_io_config_status(&self) -> &OrtIoConfigStatus {
        &self.ort_io_config_status
    }

    /// { self was prepared successfully }
    /// fn summary(&self) -> OfflineTranslatorAssetsSummary
    /// { ret is an owned no-inference preflight summary for adapters }
    pub fn summary(&self) -> OfflineTranslatorAssetsSummary {
        let generator = self.plan.generator();

        OfflineTranslatorAssetsSummary {
            model_id: generator.model_id().to_owned(),
            tokenizer_path: self.plan.tokenizer().tokenizer_path().to_path_buf(),
            encoder_path: generator.encoder().model_path().to_path_buf(),
            decoder_path: generator.decoder().model_path().to_path_buf(),
            decoder_with_past_path: generator
                .decoder_with_past()
                .map(|session| session.model_path().to_path_buf()),
            generation_config: self.generation_config,
            ort_io_config_status: self.ort_io_config_status.clone(),
        }
    }
}

/// Facade-level mock offline translator for adapter integration before real inference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MockOfflineTranslator {
    assets: OfflineTranslatorAssets,
    pipeline: TranslationPipeline<MockTokenizer, MockTokenGenerator>,
}

impl MockOfflineTranslator {
    /// { path points to a model-pack directory candidate }
    /// fn from_model_pack_path(path: impl `AsRef<Path>`) -> Result<Self, OfflineTranslatorAssetsError>
    /// { ret is Ok only when assets prepare successfully and mock pipeline is constructed }
    pub fn from_model_pack_path(
        path: impl AsRef<Path>,
    ) -> Result<Self, OfflineTranslatorAssetsError> {
        let assets = OfflineTranslatorAssets::from_model_pack_path(path)?;

        Ok(Self::from_assets(assets))
    }

    /// { assets were prepared successfully }
    /// fn from_assets(assets: OfflineTranslatorAssets) -> Self
    /// { ret owns assets and a deterministic mock translation pipeline }
    pub const fn from_assets(assets: OfflineTranslatorAssets) -> Self {
        Self {
            assets,
            pipeline: TranslationPipeline::new(MockTokenizer, MockTokenGenerator),
        }
    }

    /// { true }
    /// fn assets(&self) -> &OfflineTranslatorAssets
    /// { ret is the prepared no-inference assets backing this mock translator }
    pub const fn assets(&self) -> &OfflineTranslatorAssets {
        &self.assets
    }

    /// { request has a valid language pair and non-empty text }
    /// fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
    /// { ret is delegated to the deterministic mock pipeline }
    pub fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        <Self as TranslatorEngine>::translate(self, request)
    }
}

impl TranslatorEngine for MockOfflineTranslator {
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        self.pipeline.translate(request)
    }
}

/// Facade-level translator that uses a real HF tokenizer and mock generation.
#[cfg(feature = "hf-tokenizers")]
pub struct HfMockOfflineTranslator {
    assets: OfflineTranslatorAssets,
    pipeline: TranslationPipeline<HfTokenizer, MockTokenGenerator>,
}

#[cfg(feature = "hf-tokenizers")]
impl HfMockOfflineTranslator {
    /// { path points to a model-pack directory candidate }
    /// fn from_model_pack_path(path: impl `AsRef<Path>`) -> Result<Self, HfMockOfflineTranslatorError>
    /// { ret is Ok only when assets prepare and tokenizer JSON loads successfully }
    pub fn from_model_pack_path(
        path: impl AsRef<Path>,
    ) -> Result<Self, HfMockOfflineTranslatorError> {
        let assets = OfflineTranslatorAssets::from_model_pack_path(path)
            .map_err(HfMockOfflineTranslatorError::Assets)?;

        Self::from_assets(assets)
    }

    /// { assets were prepared from a verified model pack }
    /// fn from_assets(assets: OfflineTranslatorAssets) -> Result<Self, HfMockOfflineTranslatorError>
    /// { ret is Ok only when HfTokenizer loads from the verified tokenizer path }
    pub fn from_assets(
        assets: OfflineTranslatorAssets,
    ) -> Result<Self, HfMockOfflineTranslatorError> {
        let tokenizer = HfTokenizer::from_file(assets.plan().tokenizer().tokenizer_path())
            .map_err(HfMockOfflineTranslatorError::Tokenizer)?;

        Ok(Self {
            assets,
            pipeline: TranslationPipeline::new(tokenizer, MockTokenGenerator),
        })
    }

    /// { true }
    /// fn assets(&self) -> &OfflineTranslatorAssets
    /// { ret is the prepared assets backing this tokenizer smoke translator }
    pub const fn assets(&self) -> &OfflineTranslatorAssets {
        &self.assets
    }

    /// { request has a valid language pair and non-empty text }
    /// fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
    /// { ret is delegated to the HF-tokenizer plus mock-generator pipeline }
    pub fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        <Self as TranslatorEngine>::translate(self, request)
    }
}

#[cfg(feature = "hf-tokenizers")]
impl TranslatorEngine for HfMockOfflineTranslator {
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        self.pipeline.translate(request)
    }
}

/// Facade-level HF tokenizer smoke translator loading error.
#[cfg(feature = "hf-tokenizers")]
#[derive(Debug)]
pub enum HfMockOfflineTranslatorError {
    /// Prepared asset loading failed.
    Assets(OfflineTranslatorAssetsError),
    /// Hugging Face tokenizer loading failed.
    Tokenizer(TokenizerError),
}

#[cfg(feature = "hf-tokenizers")]
impl fmt::Display for HfMockOfflineTranslatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assets(error) => write!(formatter, "{error}"),
            Self::Tokenizer(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(feature = "hf-tokenizers")]
impl std::error::Error for HfMockOfflineTranslatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Assets(error) => Some(error),
            Self::Tokenizer(error) => Some(error),
        }
    }
}

/// Facade-level translator that uses a real HF tokenizer and ORT generation.
#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
pub struct OrtOfflineTranslator {
    assets: OfflineTranslatorAssets,
    pipeline: TranslationPipeline<HfTokenizer, OrtTokenGenerator>,
}

#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
impl OrtOfflineTranslator {
    /// { path points to a model-pack directory candidate }
    /// fn from_model_pack_path(path: impl `AsRef<Path>`) -> Result<Self, OrtOfflineTranslatorError>
    /// { ret is Ok only when assets prepare, tokenizer loads, and ORT sessions load }
    pub fn from_model_pack_path(path: impl AsRef<Path>) -> Result<Self, OrtOfflineTranslatorError> {
        let assets = OfflineTranslatorAssets::from_model_pack_path(path)
            .map_err(OrtOfflineTranslatorError::Assets)?;

        Self::from_assets(assets)
    }

    /// { assets were prepared from a verified model pack }
    /// fn from_assets(assets: OfflineTranslatorAssets) -> Result<Self, OrtOfflineTranslatorError>
    /// { ret is Ok only when tokenizer and ORT generator load from verified paths }
    pub fn from_assets(assets: OfflineTranslatorAssets) -> Result<Self, OrtOfflineTranslatorError> {
        let tokenizer = HfTokenizer::from_file(assets.plan().tokenizer().tokenizer_path())
            .map_err(OrtOfflineTranslatorError::Tokenizer)?;
        let generator = OrtTokenGenerator::load(assets.plan().generator().clone())
            .map_err(OrtOfflineTranslatorError::Generator)?;

        Ok(Self {
            assets,
            pipeline: TranslationPipeline::new(tokenizer, generator),
        })
    }

    /// { true }
    /// fn assets(&self) -> &OfflineTranslatorAssets
    /// { ret is the prepared assets backing this ORT translator }
    pub const fn assets(&self) -> &OfflineTranslatorAssets {
        &self.assets
    }

    /// { request has a valid language pair and non-empty text }
    /// fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
    /// { ret is delegated to the HF-tokenizer plus ORT-generator pipeline }
    pub fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        <Self as TranslatorEngine>::translate(self, request)
    }
}

#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
impl TranslatorEngine for OrtOfflineTranslator {
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        self.pipeline.translate(request)
    }
}

/// Facade-level ORT translator loading error.
#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
#[derive(Debug)]
pub enum OrtOfflineTranslatorError {
    /// Prepared asset loading failed.
    Assets(OfflineTranslatorAssetsError),
    /// Hugging Face tokenizer loading failed.
    Tokenizer(TokenizerError),
    /// ORT generator loading failed.
    Generator(OrtEngineError),
}

#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
impl fmt::Display for OrtOfflineTranslatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Assets(error) => write!(formatter, "{error}"),
            Self::Tokenizer(error) => write!(formatter, "{error}"),
            Self::Generator(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
impl std::error::Error for OrtOfflineTranslatorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Assets(error) => Some(error),
            Self::Tokenizer(error) => Some(error),
            Self::Generator(error) => Some(error),
        }
    }
}

/// Owned no-inference summary of prepared offline translator assets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineTranslatorAssetsSummary {
    model_id: String,
    tokenizer_path: PathBuf,
    encoder_path: PathBuf,
    decoder_path: PathBuf,
    decoder_with_past_path: Option<PathBuf>,
    generation_config: Option<GenerationConfig>,
    ort_io_config_status: OrtIoConfigStatus,
}

impl OfflineTranslatorAssetsSummary {
    /// { true }
    /// fn model_id(&self) -> &str
    /// { ret is the verified model id }
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// { true }
    /// fn tokenizer_path(&self) -> &Path
    /// { ret is the verified tokenizer asset path }
    pub fn tokenizer_path(&self) -> &Path {
        &self.tokenizer_path
    }

    /// { true }
    /// fn encoder_path(&self) -> &Path
    /// { ret is the verified encoder graph path }
    pub fn encoder_path(&self) -> &Path {
        &self.encoder_path
    }

    /// { true }
    /// fn decoder_path(&self) -> &Path
    /// { ret is the verified decoder graph path }
    pub fn decoder_path(&self) -> &Path {
        &self.decoder_path
    }

    /// { true }
    /// fn decoder_with_past_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declared a cached decoder graph }
    pub fn decoder_with_past_path(&self) -> Option<&Path> {
        self.decoder_with_past_path.as_deref()
    }

    /// { true }
    /// fn generation_config(&self) -> `Option<GenerationConfig>`
    /// { ret is Some only when the pack declared a valid generation_config }
    pub const fn generation_config(&self) -> Option<GenerationConfig> {
        self.generation_config
    }

    /// { true }
    /// fn ort_io_config_status(&self) -> &OrtIoConfigStatus
    /// { ret is the ORT I/O tensor-name readiness status }
    pub const fn ort_io_config_status(&self) -> &OrtIoConfigStatus {
        &self.ort_io_config_status
    }

    /// { true }
    /// fn to_preflight_text(&self) -> String
    /// { ret is the stable newline summary for CLI and FFI preflight adapters }
    pub fn to_preflight_text(&self) -> String {
        let decoder_with_past = self
            .decoder_with_past_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "absent".to_owned());
        let mut lines = vec![
            format!("planned: {}", self.model_id()),
            format!("tokenizer: {}", self.tokenizer_path().display()),
            format!("encoder: {}", self.encoder_path().display()),
            format!("decoder: {}", self.decoder_path().display()),
            format!("decoder_with_past: {decoder_with_past}"),
        ];

        match self.generation_config() {
            Some(config) => {
                lines.push("generation_config: parsed".to_owned());
                lines.push(format!(
                    "max_new_tokens: {}",
                    config.max_new_tokens().value()
                ));
            }
            None => lines.push("generation_config: absent".to_owned()),
        }
        match self.ort_io_config_status() {
            OrtIoConfigStatus::Absent => lines.push("ort_io_config: absent".to_owned()),
            OrtIoConfigStatus::Missing => lines.push("ort_io_config: missing".to_owned()),
            OrtIoConfigStatus::Parsed(config) => {
                lines.push("ort_io_config: parsed".to_owned());
                lines.push(format!(
                    "encoder_input_ids: {}",
                    config.encoder().input_ids()
                ));
                lines.push(format!("decoder_logits: {}", config.decoder().logits()));
            }
        }

        lines.join("\n")
    }
}

/// Facade-level prepared-asset loading error.
#[derive(Debug)]
pub enum OfflineTranslatorAssetsError {
    /// Model-pack discovery or verification failed.
    ModelPack(ModelPackError),
    /// Offline translator asset planning failed.
    Plan(OfflineTranslatorPlanError),
}

impl fmt::Display for OfflineTranslatorAssetsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelPack(error) => write!(formatter, "model pack failed: {error}"),
            Self::Plan(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for OfflineTranslatorAssetsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModelPack(error) => Some(error),
            Self::Plan(error) => Some(error),
        }
    }
}

/// Facade-level translator planning error.
#[derive(Debug)]
pub enum OfflineTranslatorPlanError {
    /// Tokenizer asset planning failed.
    Tokenizer(TokenizerError),
    /// ORT generator planning failed.
    Generator(OrtEngineError),
}

impl fmt::Display for OfflineTranslatorPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tokenizer(error) => write!(formatter, "tokenizer plan failed: {error}"),
            Self::Generator(error) => write!(formatter, "generator plan failed: {error}"),
        }
    }
}

impl std::error::Error for OfflineTranslatorPlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tokenizer(error) => Some(error),
            Self::Generator(error) => Some(error),
        }
    }
}

/// High-level translator facade over a concrete engine.
#[derive(Clone, Debug)]
pub struct Translator<E> {
    engine: E,
}

impl<E> Translator<E>
where
    E: TranslatorEngine,
{
    /// { engine implements TranslatorEngine }
    /// fn new(engine: E) -> Self
    /// { ret uses engine for future translations }
    pub const fn new(engine: E) -> Self {
        Self { engine }
    }

    /// { request has a valid language pair and non-empty text }
    /// fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
    /// { ret is delegated to the configured engine }
    pub fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        self.engine.translate(request)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        Discovered, GenerationConfig, GenerationSpecialTokens, Language, LanguagePair,
        LanguageTokenIds, MaxNewTokens, MockEngine, MockOfflineTranslator, ModelFileRole,
        ModelPack, NonEmptyText, OfflineTranslatorAssets, OfflineTranslatorPlan,
        OfflineTranslatorPlanError, OrtEngineError, OrtGenerationInputs, OrtGenerationState,
        OrtGenerationStateError, OrtGenerationTensorInputs, OrtI64TensorInput, OrtIoConfigStatus,
        OrtModelRole, TokenGeneratorError, TokenId, TokenSequence, TokenizerError, TokenizerOutput,
        TranslateRequest, Translator,
    };
    #[cfg(feature = "hf-tokenizers")]
    use super::{HfMockOfflineTranslator, HfMockOfflineTranslatorError, Sha256Digest};
    #[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
    use super::{OrtOfflineTranslator, OrtOfflineTranslatorError, Translation, TranslationError};
    #[cfg(not(feature = "ort-runtime"))]
    use super::{OrtTokenGenerator, TokenGenerator};

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const DECODER_WITH_PAST_SHA256: &str =
        "ef37d12b277fe98960af949b560ded559157bf42d2eea244dcfc64ac08da666c";
    const GENERATION_CONFIG_SHA256: &str =
        "75f35767c145e896ca56dba402cc97c3879445dace669c745ecea5fc0c9552b6";
    const VALID_GENERATION_CONFIG_SHA256: &str =
        "a8a99326d564beb1fc16cb59526dd5ed7b5fd673f969591f47845e17fbed401d";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    const VOCABULARY_SHA256: &str =
        "9e5e90102c699455e9039ff903284e0689394dd345bb11456706f087984d2eb7";
    const CONFIG_SHA256: &str = "f612b89bcdbc401379f644d7e48572e3470f77dcd4c39416405d80952ad7089e";
    const MISSING_ORT_IO_CONFIG_SHA256: &str =
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";
    const MISSING_ORT_IO_CONFIG: &str = "{}";
    const VALID_ORT_IO_CONFIG_SHA256: &str =
        "3ed8af0b5a58d51e246f3081e973d8683b89bf3cacf23c26243fec19ab31a721";
    const VALID_ORT_IO_CONFIG: &str = r#"{
  "ort_io": {
    "encoder": {
      "input_ids": "encoder_input_ids",
      "attention_mask": "encoder_attention_mask",
      "last_hidden_state": "encoder_last_hidden_state"
    },
    "decoder": {
      "input_ids": "decoder_input_ids",
      "encoder_attention_mask": "decoder_encoder_attention_mask",
      "encoder_hidden_states": "decoder_encoder_hidden_states",
      "logits": "decoder_logits"
    },
    "decoder_with_past": {
      "input_ids": "past_input_ids",
      "encoder_attention_mask": "past_encoder_attention_mask",
      "encoder_hidden_states": "past_encoder_hidden_states",
      "logits": "past_logits"
    }
  }
}"#;
    const VALID_GENERATION_CONFIG: &str = r#"{
  "max_new_tokens": 32,
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}"#;
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn translator_delegates_to_engine() -> Result<(), Box<dyn std::error::Error>> {
        let text = NonEmptyText::new("hello")?;
        let request = TranslateRequest::new(Language::English, Language::Japanese, text)?;
        let translator = Translator::new(MockEngine);

        let translation = translator.translate(&request);

        assert!(matches!(
            translation,
            Ok(ref item) if item.text().as_str() == "[en->ja] hello"
        ));

        Ok(())
    }

    #[test]
    fn offline_translator_plan_selects_tokenizer_and_generator_assets()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
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
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
            ("vocab.txt", "vocab", VOCABULARY_SHA256, "vocab\n"),
            ("config.json", "config", CONFIG_SHA256, "config\n"),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OfflineTranslatorPlan::from_pack(&pack)?;

        assert_eq!(
            plan.tokenizer().tokenizer_path(),
            root.join("tokenizer.json").as_path()
        );
        assert_eq!(
            plan.tokenizer().vocabulary_path(),
            Some(root.join("vocab.txt").as_path())
        );
        assert_eq!(
            plan.tokenizer().config_path(),
            Some(root.join("config.json").as_path())
        );
        assert_eq!(plan.generator().model_id(), "m2m100-418m-int8");
        assert_eq!(plan.generator().encoder().role(), OrtModelRole::Encoder);
        assert_eq!(
            plan.generator().encoder().model_path(),
            root.join("encoder.onnx").as_path()
        );
        assert_eq!(plan.generator().decoder().role(), OrtModelRole::Decoder);
        assert_eq!(
            plan.generator().decoder().model_path(),
            root.join("decoder.onnx").as_path()
        );
        assert_eq!(
            plan.generator().decoder_with_past().map(|item| item.role()),
            Some(OrtModelRole::DecoderWithPast)
        );
        assert_eq!(
            plan.generator().generation_config_path(),
            Some(root.join("generation.json").as_path())
        );
        Ok(())
    }

    #[test]
    fn offline_translator_plan_maps_missing_tokenizer_asset()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OfflineTranslatorPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(OfflineTranslatorPlanError::Tokenizer(
                TokenizerError::MissingTokenizerAsset
            ))
        ));
        Ok(())
    }

    #[test]
    fn offline_translator_plan_maps_missing_decoder_asset() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = OfflineTranslatorPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(OfflineTranslatorPlanError::Generator(
                OrtEngineError::GeneratorAsset(TokenGeneratorError::MissingGeneratorAsset(
                    ModelFileRole::Decoder
                ))
            ))
        ));
        Ok(())
    }

    #[test]
    fn offline_translator_plan_parses_generation_config() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "generation.json",
                "generation_config",
                VALID_GENERATION_CONFIG_SHA256,
                VALID_GENERATION_CONFIG,
            ),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OfflineTranslatorPlan::from_pack(&pack)?;

        let config = plan.parse_generation_config()?.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "generation config should be present",
            )
        })?;

        assert_eq!(config.max_new_tokens().value(), 32);
        assert_eq!(config.bos_token_id(), TokenId::new(0));
        assert_eq!(config.eos_token_id(), TokenId::new(1));
        assert_eq!(
            config.target_language_token(Language::Japanese),
            TokenId::new(14)
        );
        Ok(())
    }

    #[test]
    fn offline_translator_plan_reports_missing_ort_io_config_object()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
            (
                "config.json",
                "config",
                MISSING_ORT_IO_CONFIG_SHA256,
                MISSING_ORT_IO_CONFIG,
            ),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let plan = OfflineTranslatorPlan::from_pack(&pack)?;

        let status = plan.parse_ort_io_config_status()?;

        assert_eq!(status, OrtIoConfigStatus::Missing);
        Ok(())
    }

    #[test]
    fn offline_translator_assets_summary_reports_ort_io_config()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
            (
                "config.json",
                "config",
                VALID_ORT_IO_CONFIG_SHA256,
                VALID_ORT_IO_CONFIG,
            ),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let summary = OfflineTranslatorAssets::from_pack(&pack)?.summary();

        assert!(matches!(
            summary.ort_io_config_status(),
            OrtIoConfigStatus::Parsed(config)
                if config.encoder().input_ids() == "encoder_input_ids"
                    && config.decoder().logits() == "decoder_logits"
        ));
        let text = summary.to_preflight_text();
        assert!(text.contains("ort_io_config: parsed"));
        assert!(text.contains("encoder_input_ids: encoder_input_ids"));
        assert!(text.contains("decoder_logits: decoder_logits"));
        Ok(())
    }

    #[test]
    fn offline_translator_assets_prepare_plan_and_generation_config()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "generation.json",
                "generation_config",
                VALID_GENERATION_CONFIG_SHA256,
                VALID_GENERATION_CONFIG,
            ),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let assets = OfflineTranslatorAssets::from_pack(&pack)?;

        assert_eq!(
            assets.plan().tokenizer().tokenizer_path(),
            root.join("tokenizer.json").as_path()
        );
        assert_eq!(
            assets
                .generation_config()
                .map(|config| config.max_new_tokens().value()),
            Some(32)
        );
        Ok(())
    }

    #[test]
    fn offline_translator_assets_prepare_from_model_pack_path()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "generation.json",
                "generation_config",
                VALID_GENERATION_CONFIG_SHA256,
                VALID_GENERATION_CONFIG,
            ),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;

        let assets = OfflineTranslatorAssets::from_model_pack_path(&root)?;

        assert_eq!(assets.plan().generator().model_id(), "m2m100-418m-int8");
        assert_eq!(
            assets
                .generation_config()
                .map(|config| config.target_language_token(Language::Russian)),
            Some(TokenId::new(11))
        );
        Ok(())
    }

    #[test]
    fn offline_translator_assets_summary_reports_preflight_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
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
                VALID_GENERATION_CONFIG_SHA256,
                VALID_GENERATION_CONFIG,
            ),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;
        let assets = OfflineTranslatorAssets::from_pack(&pack)?;

        let summary = assets.summary();

        assert_eq!(summary.model_id(), "m2m100-418m-int8");
        assert_eq!(
            summary.tokenizer_path(),
            root.join("tokenizer.json").as_path()
        );
        assert_eq!(summary.encoder_path(), root.join("encoder.onnx").as_path());
        assert_eq!(summary.decoder_path(), root.join("decoder.onnx").as_path());
        assert_eq!(
            summary.decoder_with_past_path(),
            Some(root.join("decoder-with-past.onnx").as_path())
        );
        assert_eq!(
            summary
                .generation_config()
                .map(|config| config.max_new_tokens().value()),
            Some(32)
        );
        let text = summary.to_preflight_text();
        assert!(text.contains("planned: m2m100-418m-int8"));
        assert!(text.contains("tokenizer:"));
        assert!(text.contains("encoder:"));
        assert!(text.contains("decoder:"));
        assert!(text.contains("decoder_with_past:"));
        assert!(text.contains("generation_config: parsed"));
        assert!(text.contains("max_new_tokens: 32"));
        assert!(text.contains("ort_io_config: absent"));
        Ok(())
    }

    #[test]
    fn mock_offline_translator_prepares_assets_before_translation()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "generation.json",
                "generation_config",
                VALID_GENERATION_CONFIG_SHA256,
                VALID_GENERATION_CONFIG,
            ),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;
        let translator = MockOfflineTranslator::from_model_pack_path(&root)?;
        let text = NonEmptyText::new("hello")?;
        let request = TranslateRequest::new(Language::English, Language::Japanese, text)?;

        let translation = translator.translate(&request)?;

        assert_eq!(
            translator
                .assets()
                .generation_config()
                .map(|config| config.max_new_tokens().value()),
            Some(32)
        );
        assert_eq!(translation.text().as_str(), "hello");
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn hf_mock_offline_translator_loads_tokenizer_and_round_trips()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_hf_pack()?;
        let translator = HfMockOfflineTranslator::from_model_pack_path(&root)?;
        let text = NonEmptyText::new("hello offline")?;
        let request = TranslateRequest::new(Language::English, Language::Russian, text)?;

        let translation = translator.translate(&request)?;

        assert_eq!(
            translator.assets().plan().tokenizer().tokenizer_path(),
            root.join("tokenizer.json").as_path()
        );
        assert_eq!(translation.text().as_str(), "hello offline");
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn hf_mock_offline_translator_reports_tokenizer_load_errors()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
        ])?;

        let translator = HfMockOfflineTranslator::from_model_pack_path(&root);

        assert!(matches!(
            translator,
            Err(HfMockOfflineTranslatorError::Tokenizer(
                TokenizerError::TokenizerLoad { .. }
            ))
        ));
        Ok(())
    }

    #[test]
    #[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
    fn ort_offline_translator_maps_missing_pack_to_assets_error()
    -> Result<(), Box<dyn std::error::Error>> {
        let path = create_temp_dir()?.join("missing-pack");

        let translator = OrtOfflineTranslator::from_model_pack_path(&path);

        assert!(matches!(
            translator,
            Err(OrtOfflineTranslatorError::Assets(_))
        ));
        Ok(())
    }

    #[test]
    #[ignore = "compile-only signature guard; construction requires tokenizer and ONNX model assets"]
    #[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
    fn ort_offline_translator_exposes_translate_boundary() {
        fn assert_signature(translator: &OrtOfflineTranslator, request: &TranslateRequest) {
            let result: Result<Translation, TranslationError> = translator.translate(request);
            let _ = result;
        }

        let _signature: fn(&OrtOfflineTranslator, &TranslateRequest) = assert_signature;
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn ort_token_generator_generate_reports_unimplemented_backend()
    -> Result<(), Box<dyn std::error::Error>> {
        let pair = LanguagePair::new(Language::English, Language::Japanese)?;
        let tokens = TokenSequence::new(vec![TokenId::new(7)])?;
        let input = TokenizerOutput::new(pair, tokens);

        let generated = TokenGenerator::generate(&OrtTokenGenerator, &input);

        assert!(matches!(
            generated,
            Err(TokenGeneratorError::BackendUnavailable(ref reason))
                if reason == "ONNX token generation loop is not implemented"
        ));
        Ok(())
    }

    #[test]
    fn generation_inputs_prepare_encoder_and_decoder_seed() -> Result<(), Box<dyn std::error::Error>>
    {
        let pair = LanguagePair::new(Language::English, Language::Japanese)?;
        let tokens = TokenSequence::new(vec![TokenId::new(7), TokenId::new(8)])?;
        let input = TokenizerOutput::new(pair, tokens);
        let config = generation_config_with_limit(3)?;

        let inputs = OrtGenerationInputs::from_tokenizer_output(&input, config);

        assert_eq!(inputs.encoder_input_ids(), &[7, 8]);
        assert_eq!(inputs.encoder_attention_mask(), &[1, 1]);
        assert_eq!(inputs.decoder_input_ids(), &[14]);
        assert_eq!(inputs.max_new_tokens(), 3);
        assert_eq!(inputs.eos_token_id(), 1);
        Ok(())
    }

    #[test]
    fn generation_inputs_preserve_source_token_order() -> Result<(), Box<dyn std::error::Error>> {
        let pair = LanguagePair::new(Language::English, Language::Russian)?;
        let tokens = TokenSequence::new(vec![TokenId::new(42), TokenId::new(7), TokenId::new(42)])?;
        let input = TokenizerOutput::new(pair, tokens);
        let config = generation_config_with_limit(3)?;

        let inputs = OrtGenerationInputs::from_tokenizer_output(&input, config);

        assert_eq!(inputs.encoder_input_ids(), &[42, 7, 42]);
        assert_eq!(inputs.encoder_attention_mask(), &[1, 1, 1]);
        assert_eq!(inputs.decoder_input_ids(), &[11]);
        Ok(())
    }

    #[test]
    fn generation_tensor_inputs_prepare_row_shapes() -> Result<(), Box<dyn std::error::Error>> {
        let pair = LanguagePair::new(Language::English, Language::Japanese)?;
        let tokens = TokenSequence::new(vec![TokenId::new(7), TokenId::new(8)])?;
        let input = TokenizerOutput::new(pair, tokens);
        let config = generation_config_with_limit(3)?;
        let inputs = OrtGenerationInputs::from_tokenizer_output(&input, config);

        let tensor_inputs = OrtGenerationTensorInputs::from_generation_inputs(&inputs);

        assert_eq!(tensor_inputs.encoder_input_ids().shape(), [1, 2]);
        assert_eq!(tensor_inputs.encoder_input_ids().values(), &[7, 8]);
        assert_eq!(tensor_inputs.encoder_attention_mask().shape(), [1, 2]);
        assert_eq!(tensor_inputs.encoder_attention_mask().values(), &[1, 1]);
        assert_eq!(tensor_inputs.decoder_input_ids().shape(), [1, 1]);
        assert_eq!(tensor_inputs.decoder_input_ids().values(), &[14]);
        Ok(())
    }

    #[test]
    fn generation_tensor_input_row_preserves_decoder_growth() {
        let input = OrtI64TensorInput::row(&[14, 21]);

        assert_eq!(input.shape(), [1, 2]);
        assert_eq!(input.values(), &[14, 21]);
    }

    #[test]
    fn generation_state_appends_tokens_until_eos() -> Result<(), Box<dyn std::error::Error>> {
        let pair = LanguagePair::new(Language::English, Language::Japanese)?;
        let tokens = TokenSequence::new(vec![TokenId::new(7), TokenId::new(8)])?;
        let input = TokenizerOutput::new(pair, tokens);
        let inputs =
            OrtGenerationInputs::from_tokenizer_output(&input, generation_config_with_limit(3)?);
        let mut state = OrtGenerationState::new(inputs);

        assert_eq!(state.decoder_input_ids(), &[14]);
        assert!(!state.is_finished());
        assert_eq!(state.accept_next_token(TokenId::new(21)), Ok(()));
        assert_eq!(state.accept_next_token(TokenId::new(1)), Ok(()));

        assert_eq!(state.decoder_input_ids(), &[14, 21, 1]);
        assert_eq!(
            state.generated_token_ids(),
            &[TokenId::new(21), TokenId::new(1)]
        );
        assert!(state.is_finished());
        Ok(())
    }

    #[test]
    fn generation_state_stops_at_max_new_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let pair = LanguagePair::new(Language::English, Language::Russian)?;
        let tokens = TokenSequence::new(vec![TokenId::new(42)])?;
        let input = TokenizerOutput::new(pair, tokens);
        let inputs =
            OrtGenerationInputs::from_tokenizer_output(&input, generation_config_with_limit(2)?);
        let mut state = OrtGenerationState::new(inputs);

        assert_eq!(state.accept_next_token(TokenId::new(21)), Ok(()));
        assert!(!state.is_finished());
        assert_eq!(state.accept_next_token(TokenId::new(22)), Ok(()));

        assert_eq!(state.decoder_input_ids(), &[11, 21, 22]);
        assert_eq!(
            state.generated_token_ids(),
            &[TokenId::new(21), TokenId::new(22)]
        );
        assert!(state.is_finished());
        Ok(())
    }

    #[test]
    fn generation_state_rejects_tokens_after_finish() -> Result<(), Box<dyn std::error::Error>> {
        let pair = LanguagePair::new(Language::English, Language::Russian)?;
        let tokens = TokenSequence::new(vec![TokenId::new(42)])?;
        let input = TokenizerOutput::new(pair, tokens);
        let inputs =
            OrtGenerationInputs::from_tokenizer_output(&input, generation_config_with_limit(1)?);
        let mut state = OrtGenerationState::new(inputs);

        assert_eq!(state.accept_next_token(TokenId::new(21)), Ok(()));
        let before_decoder = state.decoder_input_ids().to_vec();
        let before_generated = state.generated_token_ids().to_vec();

        assert_eq!(
            state.accept_next_token(TokenId::new(22)),
            Err(OrtGenerationStateError::AlreadyFinished)
        );
        assert_eq!(state.decoder_input_ids(), before_decoder);
        assert_eq!(state.generated_token_ids(), before_generated);
        assert!(state.is_finished());
        Ok(())
    }

    fn generation_config_with_limit(
        max_new_tokens: usize,
    ) -> Result<GenerationConfig, Box<dyn std::error::Error>> {
        let max_new_tokens = MaxNewTokens::new(max_new_tokens)?;
        let special_tokens = GenerationSpecialTokens::new(TokenId::new(0), TokenId::new(1))?;
        let language_tokens = LanguageTokenIds::new(
            TokenId::new(10),
            TokenId::new(11),
            TokenId::new(12),
            TokenId::new(13),
            TokenId::new(14),
        )?;

        Ok(GenerationConfig::new(
            max_new_tokens,
            special_tokens,
            language_tokens,
        )?)
    }

    fn create_pack(
        files: &[(&str, &str, &str, &str)],
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        for (path, _role, _sha256, contents) in files {
            fs::write(root.join(path), contents)?;
        }
        fs::write(root.join("manifest.json"), manifest_json(files))?;
        Ok(root)
    }

    #[cfg(feature = "hf-tokenizers")]
    fn create_hf_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), wordlevel_tokenizer_json())?;
        let tokenizer_sha256 = Sha256Digest::from_file(root.join("tokenizer.json"))?;
        fs::write(
            root.join("manifest.json"),
            manifest_json(&[
                ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
                ("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n"),
                (
                    "tokenizer.json",
                    "tokenizer",
                    tokenizer_sha256.as_str(),
                    wordlevel_tokenizer_json(),
                ),
            ]),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-facade-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn manifest_json(files: &[(&str, &str, &str, &str)]) -> String {
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
  "runtime": "onnx-runtime",
  "license": "MIT",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
{file_json}
  ]
}}"#
        )
    }

    #[cfg(feature = "hf-tokenizers")]
    fn wordlevel_tokenizer_json() -> &'static str {
        r#"{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{"type":"WhitespaceSplit"},"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"[UNK]":0,"hello":1,"offline":2},"unk_token":"[UNK]"}}"#
    }
}
