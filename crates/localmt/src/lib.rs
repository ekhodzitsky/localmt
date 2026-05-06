//! Public facade for the localmt translation library.

use core::fmt;

pub use localmt_bench::{BenchReport, BenchmarkError, DeviceProfile, MockBenchmarkRunner};
pub use localmt_core::{
    Language, LanguageCodeError, LanguagePair, LanguagePairError, MAX_TEXT_CHARS, NonEmptyText,
    TextError, TranslateRequest, Translation,
};
pub use localmt_engine::{MockEngine, TranslationError, TranslatorEngine};
pub use localmt_engine_ort::{
    OrtEngine, OrtEngineError, OrtGeneratorPlan, OrtModelRole, OrtSessionPlan, OrtTokenGenerator,
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
}

/// Prepared no-inference assets for constructing a future offline translator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OfflineTranslatorAssets {
    plan: OfflineTranslatorPlan,
    generation_config: Option<GenerationConfig>,
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
    /// { ret is Ok only when optional generation-config parsing succeeds }
    pub fn from_plan(plan: OfflineTranslatorPlan) -> Result<Self, OfflineTranslatorPlanError> {
        let generation_config = plan.parse_generation_config()?;

        Ok(Self {
            plan,
            generation_config,
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
        Discovered, Language, MockEngine, ModelFileRole, ModelPack, NonEmptyText,
        OfflineTranslatorAssets, OfflineTranslatorPlan, OfflineTranslatorPlanError, OrtEngineError,
        OrtModelRole, TokenGeneratorError, TokenId, TokenizerError, TranslateRequest, Translator,
    };
    #[cfg(not(feature = "ort-runtime"))]
    use super::{LanguagePair, OrtTokenGenerator, TokenGenerator, TokenSequence, TokenizerOutput};

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
}
