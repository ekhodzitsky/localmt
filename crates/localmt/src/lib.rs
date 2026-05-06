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
        OfflineTranslatorPlan, OfflineTranslatorPlanError, OrtEngineError, OrtModelRole,
        TokenGeneratorError, TokenizerError, TranslateRequest, Translator,
    };
    #[cfg(not(feature = "ort-runtime"))]
    use super::{
        LanguagePair, OrtTokenGenerator, TokenGenerator, TokenId, TokenSequence, TokenizerOutput,
    };

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const DECODER_WITH_PAST_SHA256: &str =
        "ef37d12b277fe98960af949b560ded559157bf42d2eea244dcfc64ac08da666c";
    const GENERATION_CONFIG_SHA256: &str =
        "75f35767c145e896ca56dba402cc97c3879445dace669c745ecea5fc0c9552b6";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    const VOCABULARY_SHA256: &str =
        "9e5e90102c699455e9039ff903284e0689394dd345bb11456706f087984d2eb7";
    const CONFIG_SHA256: &str = "f612b89bcdbc401379f644d7e48572e3470f77dcd4c39416405d80952ad7089e";
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
