//! Translation pipeline composition for localmt.

use core::fmt;
use std::path::{Path, PathBuf};

use localmt_core::{LanguagePair, TranslateRequest, Translation};
use localmt_engine::{TranslationError, TranslatorEngine};
use localmt_models::{ModelFileRole, ModelPack, Verified};
use localmt_tokenizer::{
    TokenSequence, TokenizerEngine, TokenizerError, TokenizerInput, TokenizerOutput,
};

/// Backend contract for generating target-language tokens from encoded input.
pub trait TokenGenerator {
    /// { input contains validated tokenizer output for a source-target pair }
    /// fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError>
    /// { ret is Ok only when generated target tokens are non-empty and bounded }
    fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError>;
}

/// Deterministic generator that echoes source tokens as target tokens.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MockTokenGenerator;

impl TokenGenerator for MockTokenGenerator {
    fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError> {
        Ok(input.tokens().clone())
    }
}

/// Token generator error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenGeneratorError {
    /// Generator backend is unavailable.
    BackendUnavailable(String),
    /// Verified model pack does not declare a required generator file.
    MissingGeneratorAsset(ModelFileRole),
    /// Generator cannot translate this pair.
    UnsupportedPair(LanguagePair),
    /// Generator rejected produced tokens before decode.
    InvalidGeneratedTokens(String),
}

impl fmt::Display for TokenGeneratorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable(reason) => {
                write!(formatter, "token generator unavailable: {reason}")
            }
            Self::MissingGeneratorAsset(role) => {
                write!(formatter, "model pack is missing generator asset: {role}")
            }
            Self::UnsupportedPair(pair) => write!(
                formatter,
                "unsupported generation pair: {} -> {}",
                pair.source(),
                pair.target()
            ),
            Self::InvalidGeneratedTokens(reason) => {
                write!(formatter, "invalid generated tokens: {reason}")
            }
        }
    }
}

impl std::error::Error for TokenGeneratorError {}

/// Verified model-pack files required to construct a token generator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorAssetPlan {
    encoder_path: PathBuf,
    decoder_path: PathBuf,
    decoder_with_past_path: Option<PathBuf>,
    generation_config_path: Option<PathBuf>,
}

impl GeneratorAssetPlan {
    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`) -> Result<Self, TokenGeneratorError>
    /// { ret is Ok only when pack declares encoder and decoder roles }
    pub fn from_pack(pack: &ModelPack<Verified>) -> Result<Self, TokenGeneratorError> {
        let encoder_path = required_file_path(pack, ModelFileRole::Encoder)?;
        let decoder_path = required_file_path(pack, ModelFileRole::Decoder)?;

        Ok(Self {
            encoder_path,
            decoder_path,
            decoder_with_past_path: pack.file_path(ModelFileRole::DecoderWithPast),
            generation_config_path: pack.file_path(ModelFileRole::GenerationConfig),
        })
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
    /// { ret is Some only when the pack declares a decoder_with_past role }
    pub fn decoder_with_past_path(&self) -> Option<&Path> {
        self.decoder_with_past_path.as_deref()
    }

    /// { true }
    /// fn generation_config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declares a generation_config role }
    pub fn generation_config_path(&self) -> Option<&Path> {
        self.generation_config_path.as_deref()
    }
}

fn required_file_path(
    pack: &ModelPack<Verified>,
    role: ModelFileRole,
) -> Result<PathBuf, TokenGeneratorError> {
    pack.file_path(role)
        .ok_or(TokenGeneratorError::MissingGeneratorAsset(role))
}

/// Translation pipeline over a tokenizer and token generator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationPipeline<T, G> {
    tokenizer: T,
    generator: G,
}

impl<T, G> TranslationPipeline<T, G>
where
    T: TokenizerEngine,
    G: TokenGenerator,
{
    /// { tokenizer and generator implement their backend contracts }
    /// fn new(tokenizer: T, generator: G) -> Self
    /// { ret composes tokenizer and generator for translation }
    pub const fn new(tokenizer: T, generator: G) -> Self {
        Self {
            tokenizer,
            generator,
        }
    }

    /// { request has validated pair and text invariants }
    /// fn translate_pipeline(&self, request: &TranslateRequest) -> Result<Translation, PipelineError>
    /// { ret is Ok only when encode, generate, decode, and Translation construction succeed }
    pub fn translate_pipeline(
        &self,
        request: &TranslateRequest,
    ) -> Result<Translation, PipelineError> {
        let input = TokenizerInput::from_request(request);
        let encoded = self
            .tokenizer
            .encode(&input)
            .map_err(PipelineError::Encode)?;
        let generated = self
            .generator
            .generate(&encoded)
            .map_err(PipelineError::Generate)?;
        let text = self
            .tokenizer
            .decode(request.pair().target(), &generated)
            .map_err(PipelineError::Decode)?;

        Ok(Translation::new(text))
    }

    /// { true }
    /// fn tokenizer(&self) -> &T
    /// { ret is the tokenizer backend used by this pipeline }
    pub const fn tokenizer(&self) -> &T {
        &self.tokenizer
    }

    /// { true }
    /// fn generator(&self) -> &G
    /// { ret is the token generator backend used by this pipeline }
    pub const fn generator(&self) -> &G {
        &self.generator
    }
}

impl<T, G> TranslatorEngine for TranslationPipeline<T, G>
where
    T: TokenizerEngine,
    G: TokenGenerator,
{
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        self.translate_pipeline(request)
            .map_err(PipelineError::into_translation_error)
    }
}

/// Pipeline composition error.
#[derive(Debug)]
pub enum PipelineError {
    /// Tokenization failed before generation.
    Encode(TokenizerError),
    /// Token generation failed before decode.
    Generate(TokenGeneratorError),
    /// Decoding failed after generation.
    Decode(TokenizerError),
}

impl PipelineError {
    /// { self is a pipeline error }
    /// fn into_translation_error(self) -> TranslationError
    /// { ret is the closest public TranslatorEngine error }
    pub fn into_translation_error(self) -> TranslationError {
        match self {
            Self::Generate(TokenGeneratorError::UnsupportedPair(pair)) => {
                TranslationError::UnsupportedPair(pair)
            }
            Self::Decode(TokenizerError::InvalidText(error)) => {
                TranslationError::InvalidOutput(error)
            }
            Self::Encode(error) | Self::Decode(error) => {
                TranslationError::EngineUnavailable(error.to_string())
            }
            Self::Generate(error) => TranslationError::EngineUnavailable(error.to_string()),
        }
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(error) => write!(formatter, "tokenizer encode failed: {error}"),
            Self::Generate(error) => write!(formatter, "token generation failed: {error}"),
            Self::Decode(error) => write!(formatter, "tokenizer decode failed: {error}"),
        }
    }
}

impl std::error::Error for PipelineError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_core::{Language, NonEmptyText, TranslateRequest};
    use localmt_engine::{TranslationError, TranslatorEngine};
    use localmt_models::{Discovered, ModelFileRole, ModelPack};
    use localmt_tokenizer::{MockTokenizer, TokenId, TokenSequence, TokenizerOutput};

    use crate::{
        GeneratorAssetPlan, MockTokenGenerator, PipelineError, TokenGenerator, TokenGeneratorError,
        TranslationPipeline,
    };

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const DECODER_WITH_PAST_SHA256: &str =
        "ef37d12b277fe98960af949b560ded559157bf42d2eea244dcfc64ac08da666c";
    const GENERATION_CONFIG_SHA256: &str =
        "75f35767c145e896ca56dba402cc97c3879445dace669c745ecea5fc0c9552b6";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn pipeline_translates_with_mock_components() -> Result<(), Box<dyn std::error::Error>> {
        let pipeline = TranslationPipeline::new(MockTokenizer, MockTokenGenerator);
        let text = NonEmptyText::new("Where is the station?")?;
        let request = TranslateRequest::new(Language::English, Language::Russian, text)?;

        let translation = pipeline.translate_pipeline(&request)?;

        assert_eq!(translation.text().as_str(), "Where is the station?");
        Ok(())
    }

    #[test]
    fn pipeline_implements_translator_engine() -> Result<(), Box<dyn std::error::Error>> {
        let pipeline = TranslationPipeline::new(MockTokenizer, MockTokenGenerator);
        let text = NonEmptyText::new("hello")?;
        let request = TranslateRequest::new(Language::English, Language::Japanese, text)?;

        let translation = TranslatorEngine::translate(&pipeline, &request)?;

        assert_eq!(translation.text().as_str(), "hello");
        Ok(())
    }

    #[test]
    fn generator_receives_source_and_target_languages() -> Result<(), Box<dyn std::error::Error>> {
        let pipeline = TranslationPipeline::new(MockTokenizer, PairEchoGenerator);
        let text = NonEmptyText::new("ignored")?;
        let request = TranslateRequest::new(Language::Russian, Language::Thai, text)?;

        let translation = pipeline.translate_pipeline(&request)?;

        assert_eq!(translation.text().as_str(), "ru->th");
        Ok(())
    }

    #[test]
    fn pipeline_returns_generator_errors() -> Result<(), Box<dyn std::error::Error>> {
        let pipeline = TranslationPipeline::new(MockTokenizer, FailingGenerator);
        let text = NonEmptyText::new("hello")?;
        let request = TranslateRequest::new(Language::English, Language::Vietnamese, text)?;

        let error = pipeline.translate_pipeline(&request);

        assert!(matches!(
            error,
            Err(PipelineError::Generate(TokenGeneratorError::InvalidGeneratedTokens(ref reason)))
                if reason == "empty generated token sequence"
        ));
        Ok(())
    }

    #[test]
    fn translator_engine_maps_generator_unsupported_pair() -> Result<(), Box<dyn std::error::Error>>
    {
        let pipeline = TranslationPipeline::new(MockTokenizer, UnsupportedPairGenerator);
        let text = NonEmptyText::new("hello")?;
        let request = TranslateRequest::new(Language::English, Language::Japanese, text)?;

        let error = TranslatorEngine::translate(&pipeline, &request);

        assert!(matches!(
            error,
            Err(TranslationError::UnsupportedPair(pair))
                if pair.source() == Language::English && pair.target() == Language::Japanese
        ));
        Ok(())
    }

    #[test]
    fn generator_asset_plan_resolves_required_and_optional_paths()
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
        ])?;
        let decoder_with_past_path = root.join("decoder-with-past.onnx");
        let generation_config_path = root.join("generation.json");
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = GeneratorAssetPlan::from_pack(&pack)?;

        assert_eq!(plan.encoder_path(), root.join("encoder.onnx").as_path());
        assert_eq!(plan.decoder_path(), root.join("decoder.onnx").as_path());
        assert_eq!(
            plan.decoder_with_past_path(),
            Some(decoder_with_past_path.as_path())
        );
        assert_eq!(
            plan.generation_config_path(),
            Some(generation_config_path.as_path())
        );
        Ok(())
    }

    #[test]
    fn generator_asset_plan_rejects_pack_without_encoder_role()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[("decoder.onnx", "decoder", DECODER_SHA256, "decoder\n")])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = GeneratorAssetPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(TokenGeneratorError::MissingGeneratorAsset(
                ModelFileRole::Encoder
            ))
        ));
        Ok(())
    }

    #[test]
    fn generator_asset_plan_rejects_pack_without_decoder_role()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n")])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = GeneratorAssetPlan::from_pack(&pack);

        assert!(matches!(
            plan,
            Err(TokenGeneratorError::MissingGeneratorAsset(
                ModelFileRole::Decoder
            ))
        ));
        Ok(())
    }

    #[derive(Clone, Copy, Debug)]
    struct PairEchoGenerator;

    impl TokenGenerator for PairEchoGenerator {
        fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError> {
            tokens_for(&format!("{}->{}", input.source(), input.target()))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FailingGenerator;

    impl TokenGenerator for FailingGenerator {
        fn generate(&self, _input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError> {
            Err(TokenGeneratorError::InvalidGeneratedTokens(
                "empty generated token sequence".to_owned(),
            ))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct UnsupportedPairGenerator;

    impl TokenGenerator for UnsupportedPairGenerator {
        fn generate(&self, input: &TokenizerOutput) -> Result<TokenSequence, TokenGeneratorError> {
            Err(TokenGeneratorError::UnsupportedPair(input.pair()))
        }
    }

    fn tokens_for(value: &str) -> Result<TokenSequence, TokenGeneratorError> {
        let tokens = value
            .as_bytes()
            .iter()
            .map(|byte| TokenId::new(u32::from(*byte)))
            .collect::<Vec<_>>();
        TokenSequence::new(tokens)
            .map_err(|error| TokenGeneratorError::InvalidGeneratedTokens(error.to_string()))
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
            "localmt-pipeline-test-{}-{nanos}-{counter}",
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
