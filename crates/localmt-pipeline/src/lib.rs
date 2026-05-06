//! Translation pipeline composition for localmt.

use core::fmt;

use localmt_core::{LanguagePair, TranslateRequest, Translation};
use localmt_engine::{TranslationError, TranslatorEngine};
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
    use localmt_core::{Language, NonEmptyText, TranslateRequest};
    use localmt_engine::{TranslationError, TranslatorEngine};
    use localmt_tokenizer::{MockTokenizer, TokenId, TokenSequence, TokenizerOutput};

    use crate::{
        MockTokenGenerator, PipelineError, TokenGenerator, TokenGeneratorError, TranslationPipeline,
    };

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
}
