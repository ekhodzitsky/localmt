//! Tokenizer boundary for localmt.

use core::fmt;
use std::path::{Path, PathBuf};

use localmt_core::{
    Language, LanguagePair, LanguagePairError, MAX_TEXT_CHARS, NonEmptyText, TextError,
    TranslateRequest,
};
use localmt_models::{ModelFileRole, ModelPack, Verified};

/// Maximum token count accepted by the tokenizer boundary.
pub const MAX_TOKENS: usize = MAX_TEXT_CHARS * 4;

/// Semantic token id.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TokenId(u32);

impl TokenId {
    /// { value may be any model vocabulary id }
    /// fn new(value: u32) -> Self
    /// { ret contains value as a semantic token id }
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// { true }
    /// fn value(self) -> u32
    /// { ret is the raw model vocabulary id }
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for TokenId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

/// Non-empty bounded token sequence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenSequence {
    tokens: Vec<TokenId>,
}

impl TokenSequence {
    /// { tokens may be any vector of token ids }
    /// fn new(tokens: `Vec<TokenId>`) -> Result<Self, TokenizerError>
    /// { ret is Ok only when tokens is non-empty and len <= MAX_TOKENS }
    pub fn new(tokens: Vec<TokenId>) -> Result<Self, TokenizerError> {
        if tokens.is_empty() {
            return Err(TokenizerError::EmptyTokenSequence);
        }

        let actual_tokens = tokens.len();
        if actual_tokens > MAX_TOKENS {
            return Err(TokenizerError::TokenSequenceTooLong {
                actual_tokens,
                max_tokens: MAX_TOKENS,
            });
        }

        Ok(Self { tokens })
    }

    /// { true }
    /// fn as_slice(&self) -> &[TokenId]
    /// { ret contains every token in order }
    pub fn as_slice(&self) -> &[TokenId] {
        &self.tokens
    }

    /// { true }
    /// fn len(&self) -> usize
    /// { ret is the number of tokens }
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// { true }
    /// fn is_empty(&self) -> bool
    /// { ret is always false for a constructed TokenSequence }
    pub const fn is_empty(&self) -> bool {
        false
    }
}

/// Tokenizer input with validated language-pair and text invariants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenizerInput {
    pair: LanguagePair,
    text: NonEmptyText,
}

impl TokenizerInput {
    /// { text is non-empty and bounded }
    /// fn new(source: Language, target: Language, text: NonEmptyText) -> Result<Self, TokenizerError>
    /// { ret is Ok only when source != target }
    pub fn new(
        source: Language,
        target: Language,
        text: NonEmptyText,
    ) -> Result<Self, TokenizerError> {
        let pair = LanguagePair::new(source, target).map_err(TokenizerError::InvalidPair)?;

        Ok(Self { pair, text })
    }

    /// { request has validated pair and text invariants }
    /// fn from_request(request: &TranslateRequest) -> Self
    /// { ret carries request.pair() and a clone of request.text() }
    pub fn from_request(request: &TranslateRequest) -> Self {
        Self {
            pair: request.pair(),
            text: request.text().clone(),
        }
    }

    /// { true }
    /// fn source(&self) -> Language
    /// { ret is the source language }
    pub const fn source(&self) -> Language {
        self.pair.source()
    }

    /// { true }
    /// fn target(&self) -> Language
    /// { ret is the target language }
    pub const fn target(&self) -> Language {
        self.pair.target()
    }

    /// { true }
    /// fn pair(&self) -> LanguagePair
    /// { ret is the validated source-target pair }
    pub const fn pair(&self) -> LanguagePair {
        self.pair
    }

    /// { true }
    /// fn text(&self) -> &NonEmptyText
    /// { ret is the text to tokenize }
    pub const fn text(&self) -> &NonEmptyText {
        &self.text
    }
}

/// Tokenizer output with the language pair used during encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenizerOutput {
    pair: LanguagePair,
    tokens: TokenSequence,
}

impl TokenizerOutput {
    /// { tokens is non-empty and bounded }
    /// fn new(pair: LanguagePair, tokens: TokenSequence) -> Self
    /// { ret carries pair and tokens exactly }
    pub const fn new(pair: LanguagePair, tokens: TokenSequence) -> Self {
        Self { pair, tokens }
    }

    /// { true }
    /// fn source(&self) -> Language
    /// { ret is the source language used for encoding }
    pub const fn source(&self) -> Language {
        self.pair.source()
    }

    /// { true }
    /// fn target(&self) -> Language
    /// { ret is the target language used for encoding }
    pub const fn target(&self) -> Language {
        self.pair.target()
    }

    /// { true }
    /// fn pair(&self) -> LanguagePair
    /// { ret is the validated source-target pair }
    pub const fn pair(&self) -> LanguagePair {
        self.pair
    }

    /// { true }
    /// fn tokens(&self) -> &TokenSequence
    /// { ret is the encoded token sequence }
    pub const fn tokens(&self) -> &TokenSequence {
        &self.tokens
    }
}

/// Tokenizer backend contract.
pub trait TokenizerEngine {
    /// { input has validated text and language-pair invariants }
    /// fn encode(&self, input: &TokenizerInput) -> Result<TokenizerOutput, TokenizerError>
    /// { ret is Ok only when input text is converted to a non-empty token sequence }
    fn encode(&self, input: &TokenizerInput) -> Result<TokenizerOutput, TokenizerError>;

    /// { tokens is non-empty and bounded }
    /// fn decode(&self, target: Language, tokens: &TokenSequence) -> Result<NonEmptyText, TokenizerError>
    /// { ret is Ok only when tokens decode to non-empty bounded text for target }
    fn decode(
        &self,
        target: Language,
        tokens: &TokenSequence,
    ) -> Result<NonEmptyText, TokenizerError>;
}

/// Verified model-pack files required to construct a real tokenizer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenizerAssetPlan {
    tokenizer_path: PathBuf,
    vocabulary_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
}

impl TokenizerAssetPlan {
    /// { pack has verified manifest, files, and checksums }
    /// fn from_pack(pack: &`ModelPack<Verified>`) -> Result<Self, TokenizerError>
    /// { ret is Ok only when pack declares a tokenizer role }
    pub fn from_pack(pack: &ModelPack<Verified>) -> Result<Self, TokenizerError> {
        let tokenizer_path = pack
            .file_path(ModelFileRole::Tokenizer)
            .ok_or(TokenizerError::MissingTokenizerAsset)?;

        Ok(Self {
            tokenizer_path,
            vocabulary_path: pack.file_path(ModelFileRole::Vocabulary),
            config_path: pack.file_path(ModelFileRole::Config),
        })
    }

    /// { true }
    /// fn tokenizer_path(&self) -> &Path
    /// { ret is the verified tokenizer asset path }
    pub fn tokenizer_path(&self) -> &Path {
        &self.tokenizer_path
    }

    /// { true }
    /// fn vocabulary_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declares a vocabulary role }
    pub fn vocabulary_path(&self) -> Option<&Path> {
        self.vocabulary_path.as_deref()
    }

    /// { true }
    /// fn config_path(&self) -> `Option<&Path>`
    /// { ret is Some only when the pack declares a config role }
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }
}

/// Deterministic UTF-8 byte tokenizer used until a real tokenizer is wired.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockTokenizer;

impl TokenizerEngine for MockTokenizer {
    fn encode(&self, input: &TokenizerInput) -> Result<TokenizerOutput, TokenizerError> {
        let tokens = input
            .text()
            .as_str()
            .as_bytes()
            .iter()
            .map(|byte| TokenId::new(u32::from(*byte)))
            .collect::<Vec<_>>();
        let sequence = TokenSequence::new(tokens)?;

        Ok(TokenizerOutput::new(input.pair(), sequence))
    }

    fn decode(
        &self,
        _target: Language,
        tokens: &TokenSequence,
    ) -> Result<NonEmptyText, TokenizerError> {
        let bytes = mock_bytes(tokens)?;
        let text = String::from_utf8(bytes).map_err(TokenizerError::InvalidUtf8)?;

        NonEmptyText::new(text).map_err(TokenizerError::InvalidText)
    }
}

/// Hugging Face tokenizer.json backend.
#[cfg(feature = "hf-tokenizers")]
#[derive(Debug)]
pub struct HfTokenizer {
    tokenizer: tokenizers::Tokenizer,
    nllb_language_tokens: Option<HfNllbLanguageTokens>,
}

#[cfg(feature = "hf-tokenizers")]
impl HfTokenizer {
    /// { path points to a Hugging Face tokenizer JSON candidate }
    /// fn from_file(path: impl `AsRef<Path>`) -> Result<Self, TokenizerError>
    /// { ret is Ok only when tokenizers loads path successfully }
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, TokenizerError> {
        let path = path.as_ref();
        let tokenizer = tokenizers::Tokenizer::from_file(path).map_err(|source| {
            TokenizerError::TokenizerLoad {
                path: path.to_path_buf(),
                reason: source.to_string(),
            }
        })?;

        let nllb_language_tokens = HfNllbLanguageTokens::from_tokenizer(&tokenizer);

        Ok(Self {
            tokenizer,
            nllb_language_tokens,
        })
    }

    /// { input has validated text and language-pair invariants }
    /// fn encode_ids(&self, input: &TokenizerInput) -> Result<Vec<TokenId>, TokenizerError>
    /// { ret contains source-specialized token ids when tokenizer exposes NLLB language tokens }
    fn encode_ids(&self, input: &TokenizerInput) -> Result<Vec<TokenId>, TokenizerError> {
        match self.nllb_language_tokens {
            Some(language_tokens) => self.encode_nllb_ids(input, language_tokens),
            None => self.encode_default_ids(input.text().as_str(), true),
        }
    }

    /// { input has validated text and language-pair invariants }
    /// fn encode_nllb_ids(&self, input: &TokenizerInput, language_tokens: HfNllbLanguageTokens) -> Result<Vec<TokenId>, TokenizerError>
    /// { ret is prefixed with the input source language token and suffixed with EOS }
    fn encode_nllb_ids(
        &self,
        input: &TokenizerInput,
        language_tokens: HfNllbLanguageTokens,
    ) -> Result<Vec<TokenId>, TokenizerError> {
        let encoding = self.encode_default_ids(input.text().as_str(), false)?;
        let mut tokens = Vec::with_capacity(encoding.len() + 2);
        tokens.push(language_tokens.token_for(input.source()));
        tokens.extend(encoding);
        tokens.push(language_tokens.eos);

        Ok(tokens)
    }

    /// { text is non-empty and bounded }
    /// fn encode_default_ids(&self, text: &str, add_special_tokens: bool) -> Result<Vec<TokenId>, TokenizerError>
    /// { ret contains ids emitted by the loaded Hugging Face tokenizer }
    fn encode_default_ids(
        &self,
        text: &str,
        add_special_tokens: bool,
    ) -> Result<Vec<TokenId>, TokenizerError> {
        let encoding = self
            .tokenizer
            .encode(text, add_special_tokens)
            .map_err(|source| TokenizerError::TokenizerEncode(source.to_string()))?;

        Ok(encoding
            .get_ids()
            .iter()
            .copied()
            .map(TokenId::new)
            .collect())
    }
}

#[cfg(feature = "hf-tokenizers")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HfNllbLanguageTokens {
    english: TokenId,
    russian: TokenId,
    thai: TokenId,
    vietnamese: TokenId,
    japanese: TokenId,
    eos: TokenId,
}

#[cfg(feature = "hf-tokenizers")]
impl HfNllbLanguageTokens {
    /// { tokenizer is a loaded Hugging Face tokenizer }
    /// fn from_tokenizer(tokenizer: &tokenizers::Tokenizer) -> Option<Self>
    /// { ret is Some only when the tokenizer exposes all first-set NLLB language tokens and EOS }
    fn from_tokenizer(tokenizer: &tokenizers::Tokenizer) -> Option<Self> {
        Some(Self {
            english: TokenId::new(tokenizer.token_to_id("eng_Latn")?),
            russian: TokenId::new(tokenizer.token_to_id("rus_Cyrl")?),
            thai: TokenId::new(tokenizer.token_to_id("tha_Thai")?),
            vietnamese: TokenId::new(tokenizer.token_to_id("vie_Latn")?),
            japanese: TokenId::new(tokenizer.token_to_id("jpn_Jpan")?),
            eos: TokenId::new(tokenizer.token_to_id("</s>")?),
        })
    }

    /// { true }
    /// fn token_for(self, language: Language) -> TokenId
    /// { ret is the NLLB source language token for language }
    const fn token_for(self, language: Language) -> TokenId {
        match language {
            Language::English => self.english,
            Language::Russian => self.russian,
            Language::Thai => self.thai,
            Language::Vietnamese => self.vietnamese,
            Language::Japanese => self.japanese,
        }
    }
}

#[cfg(feature = "hf-tokenizers")]
impl TokenizerEngine for HfTokenizer {
    fn encode(&self, input: &TokenizerInput) -> Result<TokenizerOutput, TokenizerError> {
        let tokens = self.encode_ids(input)?;
        let sequence = TokenSequence::new(tokens)?;

        Ok(TokenizerOutput::new(input.pair(), sequence))
    }

    fn decode(
        &self,
        _target: Language,
        tokens: &TokenSequence,
    ) -> Result<NonEmptyText, TokenizerError> {
        let ids = tokens
            .as_slice()
            .iter()
            .map(|token| token.value())
            .collect::<Vec<_>>();
        let text = self
            .tokenizer
            .decode(&ids, true)
            .map_err(|source| TokenizerError::TokenizerDecode(source.to_string()))?;

        NonEmptyText::new(text).map_err(TokenizerError::InvalidText)
    }
}

/// Tokenizer boundary error.
#[derive(Debug)]
pub enum TokenizerError {
    /// Source and target languages violate pair invariants.
    InvalidPair(LanguagePairError),
    /// Verified model pack does not declare a tokenizer file.
    MissingTokenizerAsset,
    /// Token sequence is empty.
    EmptyTokenSequence,
    /// Token sequence exceeds the accepted bound.
    TokenSequenceTooLong {
        /// Actual number of tokens.
        actual_tokens: usize,
        /// Maximum accepted tokens.
        max_tokens: usize,
    },
    /// Mock tokenizer cannot convert the token id to a byte.
    InvalidMockToken(TokenId),
    /// Mock tokenizer bytes are not valid UTF-8.
    InvalidUtf8(std::string::FromUtf8Error),
    /// Hugging Face tokenizer file could not be loaded.
    TokenizerLoad {
        /// Path passed to the tokenizer loader.
        path: PathBuf,
        /// Backend error message.
        reason: String,
    },
    /// Hugging Face tokenizer could not encode text.
    TokenizerEncode(String),
    /// Hugging Face tokenizer could not decode ids.
    TokenizerDecode(String),
    /// Decoded text violated text invariants.
    InvalidText(TextError),
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPair(error) => write!(formatter, "{error}"),
            Self::MissingTokenizerAsset => {
                formatter.write_str("model pack is missing tokenizer asset")
            }
            Self::EmptyTokenSequence => formatter.write_str("token sequence must not be empty"),
            Self::TokenSequenceTooLong {
                actual_tokens,
                max_tokens,
            } => write!(
                formatter,
                "token sequence has {actual_tokens} tokens, maximum is {max_tokens}"
            ),
            Self::InvalidMockToken(token) => {
                write!(formatter, "mock token is not a byte: {token}")
            }
            Self::InvalidUtf8(error) => write!(formatter, "invalid UTF-8 token bytes: {error}"),
            Self::TokenizerLoad { path, reason } => {
                write!(
                    formatter,
                    "failed to load tokenizer {}: {reason}",
                    path.display()
                )
            }
            Self::TokenizerEncode(reason) => write!(formatter, "tokenizer encode failed: {reason}"),
            Self::TokenizerDecode(reason) => write!(formatter, "tokenizer decode failed: {reason}"),
            Self::InvalidText(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for TokenizerError {}

fn mock_bytes(tokens: &TokenSequence) -> Result<Vec<u8>, TokenizerError> {
    let mut bytes = Vec::with_capacity(tokens.len());
    for token in tokens.as_slice() {
        let byte = u8::try_from(token.value())
            .map_err(|_source| TokenizerError::InvalidMockToken(*token))?;
        bytes.push(byte);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(feature = "hf-tokenizers")]
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_core::{Language, NonEmptyText, TranslateRequest};
    use localmt_models::{Discovered, ModelPack};

    use crate::{
        MAX_TOKENS, MockTokenizer, TokenId, TokenSequence, TokenizerAssetPlan, TokenizerEngine,
        TokenizerError, TokenizerInput,
    };

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    const VOCABULARY_SHA256: &str =
        "9e5e90102c699455e9039ff903284e0689394dd345bb11456706f087984d2eb7";
    const CONFIG_SHA256: &str = "f612b89bcdbc401379f644d7e48572e3470f77dcd4c39416405d80952ad7089e";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn tokenizer_input_preserves_request_invariants() -> Result<(), Box<dyn std::error::Error>> {
        let text = NonEmptyText::new("hello")?;
        let input = TokenizerInput::new(Language::English, Language::Russian, text)?;

        assert_eq!(input.source(), Language::English);
        assert_eq!(input.target(), Language::Russian);
        assert_eq!(input.text().as_str(), "hello");
        Ok(())
    }

    #[test]
    fn tokenizer_input_can_be_created_from_translate_request()
    -> Result<(), Box<dyn std::error::Error>> {
        let text = NonEmptyText::new("Where is the station?")?;
        let request = TranslateRequest::new(Language::English, Language::Thai, text)?;

        let input = TokenizerInput::from_request(&request);

        assert_eq!(input.source(), Language::English);
        assert_eq!(input.target(), Language::Thai);
        assert_eq!(input.text().as_str(), "Where is the station?");
        Ok(())
    }

    #[test]
    fn token_sequence_rejects_empty_tokens() {
        let sequence = TokenSequence::new(Vec::new());

        assert!(matches!(sequence, Err(TokenizerError::EmptyTokenSequence)));
    }

    #[test]
    fn token_sequence_rejects_oversized_tokens() {
        let tokens = vec![TokenId::new(1); MAX_TOKENS + 1];

        let sequence = TokenSequence::new(tokens);

        assert!(matches!(
            sequence,
            Err(TokenizerError::TokenSequenceTooLong {
                actual_tokens,
                max_tokens: MAX_TOKENS
            }) if actual_tokens == MAX_TOKENS + 1
        ));
    }

    #[test]
    fn mock_tokenizer_round_trips_utf8_text() -> Result<(), Box<dyn std::error::Error>> {
        let text = NonEmptyText::new("Привет Bangkok")?;
        let input = TokenizerInput::new(Language::Russian, Language::Thai, text)?;
        let tokenizer = MockTokenizer;

        let encoded = tokenizer.encode(&input)?;
        let decoded = tokenizer.decode(input.target(), encoded.tokens())?;

        assert_eq!(encoded.source(), Language::Russian);
        assert_eq!(encoded.target(), Language::Thai);
        assert_eq!(decoded.as_str(), "Привет Bangkok");
        Ok(())
    }

    #[test]
    fn mock_tokenizer_rejects_invalid_utf8_tokens() -> Result<(), Box<dyn std::error::Error>> {
        let tokens = TokenSequence::new(vec![TokenId::new(0xd0)])?;
        let tokenizer = MockTokenizer;

        let decoded = tokenizer.decode(Language::Russian, &tokens);

        assert!(matches!(decoded, Err(TokenizerError::InvalidUtf8(_))));
        Ok(())
    }

    #[test]
    fn tokenizer_asset_plan_resolves_required_and_optional_paths()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[
            ("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n"),
            (
                "tokenizer.json",
                "tokenizer",
                TOKENIZER_SHA256,
                "tokenizer\n",
            ),
            ("vocab.txt", "vocab", VOCABULARY_SHA256, "vocab\n"),
            ("config.json", "config", CONFIG_SHA256, "config\n"),
        ])?;
        let vocabulary_path = root.join("vocab.txt");
        let config_path = root.join("config.json");
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = TokenizerAssetPlan::from_pack(&pack)?;

        assert_eq!(plan.tokenizer_path(), root.join("tokenizer.json").as_path());
        assert_eq!(plan.vocabulary_path(), Some(vocabulary_path.as_path()));
        assert_eq!(plan.config_path(), Some(config_path.as_path()));
        Ok(())
    }

    #[test]
    fn tokenizer_asset_plan_rejects_pack_without_tokenizer_role()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack(&[("encoder.onnx", "encoder", ENCODER_SHA256, "encoder\n")])?;
        let pack = ModelPack::<Discovered>::discover(&root)?.verify()?;

        let plan = TokenizerAssetPlan::from_pack(&pack);

        assert!(matches!(plan, Err(TokenizerError::MissingTokenizerAsset)));
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn hf_tokenizer_loads_tokenizer_json_and_round_trips_words()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        let tokenizer_path = root.join("tokenizer.json");
        write_wordlevel_tokenizer(&tokenizer_path)?;

        let tokenizer = super::HfTokenizer::from_file(&tokenizer_path)?;
        let text = NonEmptyText::new("hello offline")?;
        let input = TokenizerInput::new(Language::English, Language::Russian, text)?;
        let encoded = tokenizer.encode(&input)?;

        assert_eq!(
            encoded
                .tokens()
                .as_slice()
                .iter()
                .map(|token| token.value())
                .collect::<Vec<_>>(),
            vec![1, 2]
        );

        let decoded = tokenizer.decode(Language::Russian, encoded.tokens())?;
        assert_eq!(decoded.as_str(), "hello offline");

        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn hf_tokenizer_prefixes_nllb_source_language_token() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_temp_dir()?;
        let tokenizer_path = root.join("tokenizer.json");
        write_nllb_wordlevel_tokenizer(&tokenizer_path)?;

        let tokenizer = super::HfTokenizer::from_file(&tokenizer_path)?;
        let text = NonEmptyText::new("hello offline")?;
        let input = TokenizerInput::new(Language::Russian, Language::English, text)?;
        let encoded = tokenizer.encode(&input)?;

        assert_eq!(
            encoded
                .tokens()
                .as_slice()
                .iter()
                .map(|token| token.value())
                .collect::<Vec<_>>(),
            vec![5, 1, 2, 3]
        );

        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn hf_tokenizer_reports_load_errors() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        let missing_path = root.join("missing-tokenizer.json");

        let tokenizer = super::HfTokenizer::from_file(&missing_path);

        assert!(matches!(
            tokenizer,
            Err(TokenizerError::TokenizerLoad { ref path, .. }) if path == &missing_path
        ));

        Ok(())
    }

    #[cfg(feature = "hf-tokenizers")]
    fn write_wordlevel_tokenizer(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use tokenizers::Tokenizer;
        use tokenizers::models::wordlevel::WordLevel;
        use tokenizers::pre_tokenizers::whitespace::WhitespaceSplit;

        let vocab = [
            ("[UNK]".to_owned(), 0),
            ("hello".to_owned(), 1),
            ("offline".to_owned(), 2),
        ]
        .into_iter()
        .collect();
        let model = WordLevel::builder()
            .vocab(vocab)
            .unk_token("[UNK]".to_owned())
            .build()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let mut tokenizer = Tokenizer::new(model);
        tokenizer.with_pre_tokenizer(Some(WhitespaceSplit));
        tokenizer
            .save(path, false)
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        Ok(())
    }

    #[cfg(feature = "hf-tokenizers")]
    fn write_nllb_wordlevel_tokenizer(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use tokenizers::Tokenizer;
        use tokenizers::models::wordlevel::WordLevel;
        use tokenizers::pre_tokenizers::whitespace::WhitespaceSplit;
        use tokenizers::processors::template::TemplateProcessing;

        let vocab = [
            ("[UNK]".to_owned(), 0),
            ("hello".to_owned(), 1),
            ("offline".to_owned(), 2),
            ("</s>".to_owned(), 3),
            ("eng_Latn".to_owned(), 4),
            ("rus_Cyrl".to_owned(), 5),
            ("tha_Thai".to_owned(), 6),
            ("vie_Latn".to_owned(), 7),
            ("jpn_Jpan".to_owned(), 8),
        ]
        .into_iter()
        .collect();
        let model = WordLevel::builder()
            .vocab(vocab)
            .unk_token("[UNK]".to_owned())
            .build()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let mut tokenizer = Tokenizer::new(model);
        tokenizer.with_pre_tokenizer(Some(WhitespaceSplit));
        let processor = TemplateProcessing::builder()
            .try_single("eng_Latn $A </s>")
            .map_err(|error| std::io::Error::other(error.to_string()))?
            .special_tokens(vec![("eng_Latn", 4), ("</s>", 3)])
            .build()
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        tokenizer.with_post_processor(Some(processor));
        tokenizer
            .save(path, false)
            .map_err(|error| std::io::Error::other(error.to_string()))?;

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
            "localmt-tokenizer-test-{}-{nanos}-{counter}",
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
