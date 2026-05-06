//! Tokenizer boundary for localmt.

use core::fmt;

use localmt_core::{
    Language, LanguagePair, LanguagePairError, MAX_TEXT_CHARS, NonEmptyText, TextError,
    TranslateRequest,
};

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

/// Tokenizer boundary error.
#[derive(Debug)]
pub enum TokenizerError {
    /// Source and target languages violate pair invariants.
    InvalidPair(LanguagePairError),
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
    /// Decoded text violated text invariants.
    InvalidText(TextError),
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPair(error) => write!(formatter, "{error}"),
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
    use localmt_core::{Language, NonEmptyText, TranslateRequest};

    use crate::{
        MAX_TOKENS, MockTokenizer, TokenId, TokenSequence, TokenizerEngine, TokenizerError,
        TokenizerInput,
    };

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
}
