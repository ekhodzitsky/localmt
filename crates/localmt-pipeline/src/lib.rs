//! Translation pipeline composition for localmt.

use core::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use localmt_core::{Language, LanguagePair, TranslateRequest, Translation};
use localmt_engine::{TranslationError, TranslatorEngine};
use localmt_models::{ModelFileRole, ModelPack, Verified};
use localmt_tokenizer::{
    MAX_TOKENS, TokenId, TokenSequence, TokenizerEngine, TokenizerError, TokenizerInput,
    TokenizerOutput,
};
use serde::Deserialize;

/// Default upper bound for newly generated target tokens.
pub const DEFAULT_MAX_NEW_TOKENS: usize = 128;

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

/// Positive bounded count of tokens a generator may append.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaxNewTokens(usize);

impl MaxNewTokens {
    /// { value may be any usize }
    /// fn new(value: usize) -> Result<Self, GenerationConfigError>
    /// { ret is Ok only when 0 < value <= MAX_TOKENS }
    pub const fn new(value: usize) -> Result<Self, GenerationConfigError> {
        if value == 0 {
            return Err(GenerationConfigError::EmptyMaxNewTokens);
        }

        if value > MAX_TOKENS {
            return Err(GenerationConfigError::MaxNewTokensTooLarge {
                actual_tokens: value,
                max_tokens: MAX_TOKENS,
            });
        }

        Ok(Self(value))
    }

    /// { true }
    /// fn value(self) -> usize
    /// { ret is the positive bounded max-new-token count }
    pub const fn value(self) -> usize {
        self.0
    }
}

impl Default for MaxNewTokens {
    fn default() -> Self {
        Self(DEFAULT_MAX_NEW_TOKENS)
    }
}

/// Required decoder special tokens.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationSpecialTokens {
    bos_token_id: TokenId,
    eos_token_id: TokenId,
}

impl GenerationSpecialTokens {
    /// { bos_token_id and eos_token_id are model vocabulary ids }
    /// fn new(bos_token_id: TokenId, eos_token_id: TokenId) -> Result<Self, GenerationConfigError>
    /// { ret is Ok only when BOS and EOS are distinct }
    pub const fn new(
        bos_token_id: TokenId,
        eos_token_id: TokenId,
    ) -> Result<Self, GenerationConfigError> {
        if bos_token_id.value() == eos_token_id.value() {
            return Err(GenerationConfigError::DuplicateSpecialToken {
                token: bos_token_id,
            });
        }

        Ok(Self {
            bos_token_id,
            eos_token_id,
        })
    }

    /// { true }
    /// fn bos_token_id(self) -> TokenId
    /// { ret is the beginning-of-sequence token id }
    pub const fn bos_token_id(self) -> TokenId {
        self.bos_token_id
    }

    /// { true }
    /// fn eos_token_id(self) -> TokenId
    /// { ret is the end-of-sequence token id }
    pub const fn eos_token_id(self) -> TokenId {
        self.eos_token_id
    }
}

/// Required target-language token ids for the first supported language set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageTokenIds {
    english: TokenId,
    russian: TokenId,
    thai: TokenId,
    vietnamese: TokenId,
    japanese: TokenId,
}

impl LanguageTokenIds {
    /// { token ids may be any model vocabulary ids }
    /// fn new(english: TokenId, russian: TokenId, thai: TokenId, vietnamese: TokenId, japanese: TokenId) -> Result<Self, GenerationConfigError>
    /// { ret is Ok only when language token ids are distinct }
    pub fn new(
        english: TokenId,
        russian: TokenId,
        thai: TokenId,
        vietnamese: TokenId,
        japanese: TokenId,
    ) -> Result<Self, GenerationConfigError> {
        let ids = [
            (Language::English, english),
            (Language::Russian, russian),
            (Language::Thai, thai),
            (Language::Vietnamese, vietnamese),
            (Language::Japanese, japanese),
        ];
        ensure_distinct_language_tokens(&ids)?;

        Ok(Self {
            english,
            russian,
            thai,
            vietnamese,
            japanese,
        })
    }

    /// { true }
    /// fn token_for(self, language: Language) -> TokenId
    /// { ret is the configured token id for language }
    pub const fn token_for(self, language: Language) -> TokenId {
        match language {
            Language::English => self.english,
            Language::Russian => self.russian,
            Language::Thai => self.thai,
            Language::Vietnamese => self.vietnamese,
            Language::Japanese => self.japanese,
        }
    }
}

/// Typed decoder generation configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationConfig {
    max_new_tokens: MaxNewTokens,
    special_tokens: GenerationSpecialTokens,
    language_tokens: LanguageTokenIds,
}

impl GenerationConfig {
    /// { path points to a readable JSON generation config file }
    /// fn from_json_file(path: &Path) -> Result<Self, GenerationConfigParseError>
    /// { ret is Ok only when file contents parse and validate as GenerationConfig }
    pub fn from_json_file(path: &Path) -> Result<Self, GenerationConfigParseError> {
        let contents =
            fs::read_to_string(path).map_err(|source| GenerationConfigParseError::ReadFile {
                path: path.to_path_buf(),
                reason: source.to_string(),
            })?;

        Self::from_json_str(&contents)
    }

    /// { contents is a JSON generation config document }
    /// fn from_json_str(contents: &str) -> Result<Self, GenerationConfigParseError>
    /// { ret is Ok only when contents parse and validate as GenerationConfig }
    pub fn from_json_str(contents: &str) -> Result<Self, GenerationConfigParseError> {
        let raw = serde_json::from_str::<RawGenerationConfig>(contents).map_err(|source| {
            GenerationConfigParseError::ParseJson {
                reason: source.to_string(),
            }
        })?;

        raw.try_into_config()
    }

    /// { special_tokens and language_tokens were validated }
    /// fn with_default_limit(special_tokens: GenerationSpecialTokens, language_tokens: LanguageTokenIds) -> Result<Self, GenerationConfigError>
    /// { ret is Ok only when language tokens do not collide with BOS/EOS }
    pub fn with_default_limit(
        special_tokens: GenerationSpecialTokens,
        language_tokens: LanguageTokenIds,
    ) -> Result<Self, GenerationConfigError> {
        Self::new(MaxNewTokens::default(), special_tokens, language_tokens)
    }

    /// { max_new_tokens, special_tokens, and language_tokens were validated }
    /// fn new(max_new_tokens: MaxNewTokens, special_tokens: GenerationSpecialTokens, language_tokens: LanguageTokenIds) -> Result<Self, GenerationConfigError>
    /// { ret is Ok only when language tokens do not collide with BOS/EOS }
    pub fn new(
        max_new_tokens: MaxNewTokens,
        special_tokens: GenerationSpecialTokens,
        language_tokens: LanguageTokenIds,
    ) -> Result<Self, GenerationConfigError> {
        ensure_language_tokens_do_not_collide_with_specials(special_tokens, language_tokens)?;

        Ok(Self {
            max_new_tokens,
            special_tokens,
            language_tokens,
        })
    }

    /// { true }
    /// fn max_new_tokens(&self) -> MaxNewTokens
    /// { ret is the max-new-token generation limit }
    pub const fn max_new_tokens(&self) -> MaxNewTokens {
        self.max_new_tokens
    }

    /// { true }
    /// fn bos_token_id(&self) -> TokenId
    /// { ret is the configured beginning-of-sequence token id }
    pub const fn bos_token_id(&self) -> TokenId {
        self.special_tokens.bos_token_id()
    }

    /// { true }
    /// fn eos_token_id(&self) -> TokenId
    /// { ret is the configured end-of-sequence token id }
    pub const fn eos_token_id(&self) -> TokenId {
        self.special_tokens.eos_token_id()
    }

    /// { true }
    /// fn target_language_token(&self, language: Language) -> TokenId
    /// { ret is the configured target-language token id }
    pub const fn target_language_token(&self, language: Language) -> TokenId {
        self.language_tokens.token_for(language)
    }
}

/// Generation configuration validation error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationConfigError {
    /// Max-new-token count must be positive.
    EmptyMaxNewTokens,
    /// Max-new-token count exceeds token boundary.
    MaxNewTokensTooLarge {
        /// Requested generated-token count.
        actual_tokens: usize,
        /// Maximum accepted generated-token count.
        max_tokens: usize,
    },
    /// BOS and EOS token ids are equal.
    DuplicateSpecialToken {
        /// Duplicated special token id.
        token: TokenId,
    },
    /// Two language roles use the same token id.
    DuplicateLanguageToken {
        /// First language role.
        first: Language,
        /// Second language role.
        second: Language,
        /// Duplicated language token id.
        token: TokenId,
    },
    /// A language token reuses a special token id.
    LanguageTokenCollidesWithSpecialToken {
        /// Language role with the colliding token.
        language: Language,
        /// Special token role.
        special: GenerationSpecialTokenRole,
        /// Colliding token id.
        token: TokenId,
    },
}

impl fmt::Display for GenerationConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMaxNewTokens => formatter.write_str("max_new_tokens must be positive"),
            Self::MaxNewTokensTooLarge {
                actual_tokens,
                max_tokens,
            } => write!(
                formatter,
                "max_new_tokens has {actual_tokens} tokens, maximum is {max_tokens}"
            ),
            Self::DuplicateSpecialToken { token } => {
                write!(formatter, "BOS and EOS token ids must be distinct: {token}")
            }
            Self::DuplicateLanguageToken {
                first,
                second,
                token,
            } => write!(
                formatter,
                "language token ids must be distinct: {first} and {second} both use {token}"
            ),
            Self::LanguageTokenCollidesWithSpecialToken {
                language,
                special,
                token,
            } => write!(
                formatter,
                "language token id for {language} collides with {special}: {token}"
            ),
        }
    }
}

impl std::error::Error for GenerationConfigError {}

/// Generation config JSON parsing error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationConfigParseError {
    /// Config file could not be read.
    ReadFile {
        /// Path attempted.
        path: PathBuf,
        /// Filesystem error message.
        reason: String,
    },
    /// Config JSON could not be decoded.
    ParseJson {
        /// Parser error message.
        reason: String,
    },
    /// Config JSON decoded but failed semantic validation.
    InvalidConfig(GenerationConfigError),
}

impl fmt::Display for GenerationConfigParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFile { path, reason } => {
                write!(
                    formatter,
                    "failed to read generation config {}: {reason}",
                    path.display()
                )
            }
            Self::ParseJson { reason } => {
                write!(formatter, "invalid generation config JSON: {reason}")
            }
            Self::InvalidConfig(error) => write!(formatter, "invalid generation config: {error}"),
        }
    }
}

impl std::error::Error for GenerationConfigParseError {}

#[derive(Deserialize)]
struct RawGenerationConfig {
    max_new_tokens: Option<usize>,
    bos_token_id: u32,
    eos_token_id: u32,
    language_token_ids: RawLanguageTokenIds,
}

impl RawGenerationConfig {
    fn try_into_config(self) -> Result<GenerationConfig, GenerationConfigParseError> {
        let max_new_tokens = match self.max_new_tokens {
            Some(value) => {
                MaxNewTokens::new(value).map_err(GenerationConfigParseError::InvalidConfig)?
            }
            None => MaxNewTokens::default(),
        };
        let special_tokens = GenerationSpecialTokens::new(
            TokenId::new(self.bos_token_id),
            TokenId::new(self.eos_token_id),
        )
        .map_err(GenerationConfigParseError::InvalidConfig)?;
        let language_tokens = self.language_token_ids.try_into_tokens()?;

        GenerationConfig::new(max_new_tokens, special_tokens, language_tokens)
            .map_err(GenerationConfigParseError::InvalidConfig)
    }
}

#[derive(Deserialize)]
struct RawLanguageTokenIds {
    en: u32,
    ru: u32,
    th: u32,
    vi: u32,
    ja: u32,
}

impl RawLanguageTokenIds {
    fn try_into_tokens(self) -> Result<LanguageTokenIds, GenerationConfigParseError> {
        LanguageTokenIds::new(
            TokenId::new(self.en),
            TokenId::new(self.ru),
            TokenId::new(self.th),
            TokenId::new(self.vi),
            TokenId::new(self.ja),
        )
        .map_err(GenerationConfigParseError::InvalidConfig)
    }
}

/// Special token role used in generation-config validation errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationSpecialTokenRole {
    /// Beginning-of-sequence token role.
    Bos,
    /// End-of-sequence token role.
    Eos,
}

impl fmt::Display for GenerationSpecialTokenRole {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bos => formatter.write_str("BOS"),
            Self::Eos => formatter.write_str("EOS"),
        }
    }
}

fn ensure_distinct_language_tokens(
    ids: &[(Language, TokenId); 5],
) -> Result<(), GenerationConfigError> {
    for (index, (first_language, first_token)) in ids.iter().enumerate() {
        for (second_language, second_token) in &ids[index + 1..] {
            if first_token.value() == second_token.value() {
                return Err(GenerationConfigError::DuplicateLanguageToken {
                    first: *first_language,
                    second: *second_language,
                    token: *first_token,
                });
            }
        }
    }

    Ok(())
}

fn ensure_language_tokens_do_not_collide_with_specials(
    special_tokens: GenerationSpecialTokens,
    language_tokens: LanguageTokenIds,
) -> Result<(), GenerationConfigError> {
    for language in [
        Language::English,
        Language::Russian,
        Language::Thai,
        Language::Vietnamese,
        Language::Japanese,
    ] {
        let token = language_tokens.token_for(language);
        ensure_not_special_token(
            language,
            token,
            GenerationSpecialTokenRole::Bos,
            special_tokens,
        )?;
        ensure_not_special_token(
            language,
            token,
            GenerationSpecialTokenRole::Eos,
            special_tokens,
        )?;
    }

    Ok(())
}

fn ensure_not_special_token(
    language: Language,
    token: TokenId,
    special: GenerationSpecialTokenRole,
    special_tokens: GenerationSpecialTokens,
) -> Result<(), GenerationConfigError> {
    let special_token = match special {
        GenerationSpecialTokenRole::Bos => special_tokens.bos_token_id(),
        GenerationSpecialTokenRole::Eos => special_tokens.eos_token_id(),
    };
    if token.value() == special_token.value() {
        return Err(
            GenerationConfigError::LanguageTokenCollidesWithSpecialToken {
                language,
                special,
                token,
            },
        );
    }

    Ok(())
}

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
    use localmt_tokenizer::{MAX_TOKENS, MockTokenizer, TokenId, TokenSequence, TokenizerOutput};

    use crate::{
        DEFAULT_MAX_NEW_TOKENS, GenerationConfig, GenerationConfigError,
        GenerationConfigParseError, GenerationSpecialTokenRole, GenerationSpecialTokens,
        GeneratorAssetPlan, LanguageTokenIds, MaxNewTokens, MockTokenGenerator, PipelineError,
        TokenGenerator, TokenGeneratorError, TranslationPipeline,
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

    #[test]
    fn generation_config_uses_default_limit_and_language_tokens()
    -> Result<(), Box<dyn std::error::Error>> {
        let special_tokens = GenerationSpecialTokens::new(TokenId::new(0), TokenId::new(1))?;
        let language_tokens = LanguageTokenIds::new(
            TokenId::new(10),
            TokenId::new(11),
            TokenId::new(12),
            TokenId::new(13),
            TokenId::new(14),
        )?;

        let config = GenerationConfig::with_default_limit(special_tokens, language_tokens)?;

        assert_eq!(config.max_new_tokens().value(), DEFAULT_MAX_NEW_TOKENS);
        assert_eq!(config.bos_token_id(), TokenId::new(0));
        assert_eq!(config.eos_token_id(), TokenId::new(1));
        assert_eq!(
            config.target_language_token(Language::Japanese),
            TokenId::new(14)
        );
        Ok(())
    }

    #[test]
    fn generation_config_rejects_empty_and_oversized_limits() {
        let empty = MaxNewTokens::new(0);
        let oversized = MaxNewTokens::new(MAX_TOKENS + 1);

        assert!(matches!(
            empty,
            Err(GenerationConfigError::EmptyMaxNewTokens)
        ));
        assert!(matches!(
            oversized,
            Err(GenerationConfigError::MaxNewTokensTooLarge {
                actual_tokens,
                max_tokens: MAX_TOKENS
            }) if actual_tokens == MAX_TOKENS + 1
        ));
    }

    #[test]
    fn generation_config_rejects_duplicate_special_tokens() {
        let tokens = GenerationSpecialTokens::new(TokenId::new(2), TokenId::new(2));

        assert!(matches!(
            tokens,
            Err(GenerationConfigError::DuplicateSpecialToken {
                token
            }) if token == TokenId::new(2)
        ));
    }

    #[test]
    fn generation_config_rejects_duplicate_language_tokens() {
        let tokens = LanguageTokenIds::new(
            TokenId::new(10),
            TokenId::new(10),
            TokenId::new(12),
            TokenId::new(13),
            TokenId::new(14),
        );

        assert!(matches!(
            tokens,
            Err(GenerationConfigError::DuplicateLanguageToken {
                first: Language::English,
                second: Language::Russian,
                token
            }) if token == TokenId::new(10)
        ));
    }

    #[test]
    fn generation_config_rejects_language_tokens_colliding_with_special_tokens()
    -> Result<(), Box<dyn std::error::Error>> {
        let special_tokens = GenerationSpecialTokens::new(TokenId::new(0), TokenId::new(1))?;
        let language_tokens = LanguageTokenIds::new(
            TokenId::new(10),
            TokenId::new(11),
            TokenId::new(1),
            TokenId::new(13),
            TokenId::new(14),
        )?;

        let config = GenerationConfig::with_default_limit(special_tokens, language_tokens);

        assert!(matches!(
            config,
            Err(GenerationConfigError::LanguageTokenCollidesWithSpecialToken {
                language: Language::Thai,
                special: GenerationSpecialTokenRole::Eos,
                token
            }) if token == TokenId::new(1)
        ));
        Ok(())
    }

    #[test]
    fn generation_config_parses_json_file() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        let path = root.join("generation.json");
        fs::write(
            &path,
            r#"{
  "max_new_tokens": 64,
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}"#,
        )?;

        let config = GenerationConfig::from_json_file(&path)?;

        assert_eq!(config.max_new_tokens().value(), 64);
        assert_eq!(config.bos_token_id(), TokenId::new(0));
        assert_eq!(config.eos_token_id(), TokenId::new(1));
        assert_eq!(
            config.target_language_token(Language::Japanese),
            TokenId::new(14)
        );
        Ok(())
    }

    #[test]
    fn generation_config_parses_default_limit_from_json_str()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::from_json_str(
            r#"{
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}"#,
        )?;

        assert_eq!(config.max_new_tokens().value(), DEFAULT_MAX_NEW_TOKENS);
        Ok(())
    }

    #[test]
    fn generation_config_reports_malformed_json() {
        let config = GenerationConfig::from_json_str("{");

        assert!(matches!(
            config,
            Err(GenerationConfigParseError::ParseJson { .. })
        ));
    }

    #[test]
    fn generation_config_reports_missing_language_token() {
        let config = GenerationConfig::from_json_str(
            r#"{
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13
  }
}"#,
        );

        assert!(matches!(
            config,
            Err(GenerationConfigParseError::ParseJson { ref reason })
                if reason.contains("missing field `ja`")
        ));
    }

    #[test]
    fn generation_config_reports_validation_errors_from_json()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::from_json_str(
            r#"{
  "max_new_tokens": 0,
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}"#,
        );

        assert!(matches!(
            config,
            Err(GenerationConfigParseError::InvalidConfig(
                GenerationConfigError::EmptyMaxNewTokens
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
