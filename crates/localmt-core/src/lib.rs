//! Core invariant types for localmt.

use core::fmt;

/// Maximum accepted input text length for the first library slice.
pub const MAX_TEXT_CHARS: usize = 4096;

/// Publicly supported language.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Language {
    /// English.
    English,
    /// Russian.
    Russian,
    /// Thai.
    Thai,
    /// Vietnamese.
    Vietnamese,
    /// Japanese.
    Japanese,
}

impl Language {
    /// { true }
    /// fn iso_639_1(self) -> &'static str
    /// { ret is the two-letter ISO 639-1 code for self }
    pub const fn iso_639_1(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Russian => "ru",
            Self::Thai => "th",
            Self::Vietnamese => "vi",
            Self::Japanese => "ja",
        }
    }

    /// { true }
    /// fn m2m100_code(self) -> &'static str
    /// { ret is the M2M100 language code for self }
    pub const fn m2m100_code(self) -> &'static str {
        self.iso_639_1()
    }

    /// { true }
    /// fn display_name(self) -> &'static str
    /// { ret is a stable English display name for self }
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Russian",
            Self::Thai => "Thai",
            Self::Vietnamese => "Vietnamese",
            Self::Japanese => "Japanese",
        }
    }

    /// { code may be any string }
    /// fn from_iso_639_1(code: &str) -> Result<Self, LanguageCodeError>
    /// { ret is Ok only for en, ru, th, vi, or ja }
    pub fn from_iso_639_1(code: &str) -> Result<Self, LanguageCodeError> {
        match code {
            "en" => Ok(Self::English),
            "ru" => Ok(Self::Russian),
            "th" => Ok(Self::Thai),
            "vi" => Ok(Self::Vietnamese),
            "ja" => Ok(Self::Japanese),
            _ => Err(LanguageCodeError {
                code: code.to_owned(),
            }),
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.iso_639_1())
    }
}

/// Error returned when a language code is not in the supported set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCodeError {
    code: String,
}

impl LanguageCodeError {
    /// { true }
    /// fn code(&self) -> &str
    /// { ret is the unsupported code that failed parsing }
    pub fn code(&self) -> &str {
        &self.code
    }
}

impl fmt::Display for LanguageCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unsupported language code: {}", self.code)
    }
}

impl std::error::Error for LanguageCodeError {}

/// A valid source-target language pair.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LanguagePair {
    source: Language,
    target: Language,
}

impl LanguagePair {
    /// { source and target are supported languages }
    /// fn new(source: Language, target: Language) -> Result<Self, LanguagePairError>
    /// { ret is Ok only when source != target }
    pub const fn new(source: Language, target: Language) -> Result<Self, LanguagePairError> {
        if source as u8 == target as u8 {
            Err(LanguagePairError::SameLanguage(source))
        } else {
            Ok(Self { source, target })
        }
    }

    /// { true }
    /// fn source(self) -> Language
    /// { ret is the pair source language }
    pub const fn source(self) -> Language {
        self.source
    }

    /// { true }
    /// fn target(self) -> Language
    /// { ret is the pair target language }
    pub const fn target(self) -> Language {
        self.target
    }
}

/// Error returned when a requested language pair violates invariants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguagePairError {
    /// Source and target languages are identical.
    SameLanguage(Language),
}

impl fmt::Display for LanguagePairError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameLanguage(language) => write!(
                formatter,
                "source and target language are both {}",
                language.display_name()
            ),
        }
    }
}

impl std::error::Error for LanguagePairError {}

/// Non-empty bounded text.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NonEmptyText {
    value: String,
}

impl NonEmptyText {
    /// { value may be any string }
    /// fn new(value: impl `Into<String>`) -> Result<Self, TextError>
    /// { ret is Ok only when value has non-whitespace text and <= MAX_TEXT_CHARS chars }
    pub fn new(value: impl Into<String>) -> Result<Self, TextError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(TextError::Empty);
        }

        let actual_chars = value.chars().count();
        if actual_chars > MAX_TEXT_CHARS {
            return Err(TextError::TooLong {
                actual_chars,
                max_chars: MAX_TEXT_CHARS,
            });
        }

        Ok(Self { value })
    }

    /// { true }
    /// fn as_str(&self) -> &str
    /// { ret is the original accepted text }
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// { true }
    /// fn char_count(&self) -> usize
    /// { ret is the number of Unicode scalar values in self }
    pub fn char_count(&self) -> usize {
        self.value.chars().count()
    }
}

impl fmt::Display for NonEmptyText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Error returned when text violates localmt text invariants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextError {
    /// Text is empty after whitespace trimming.
    Empty,
    /// Text is longer than the accepted bound.
    TooLong {
        /// Actual number of characters.
        actual_chars: usize,
        /// Maximum accepted number of characters.
        max_chars: usize,
    },
}

impl fmt::Display for TextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("text must not be empty"),
            Self::TooLong {
                actual_chars,
                max_chars,
            } => write!(
                formatter,
                "text has {actual_chars} characters, maximum is {max_chars}"
            ),
        }
    }
}

impl std::error::Error for TextError {}

/// A validated translation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslateRequest {
    pair: LanguagePair,
    text: NonEmptyText,
}

impl TranslateRequest {
    /// { text is non-empty and bounded }
    /// fn new(source: Language, target: Language, text: NonEmptyText) -> Result<Self, LanguagePairError>
    /// { ret is Ok only when source != target }
    pub fn new(
        source: Language,
        target: Language,
        text: NonEmptyText,
    ) -> Result<Self, LanguagePairError> {
        match LanguagePair::new(source, target) {
            Ok(pair) => Ok(Self { pair, text }),
            Err(error) => Err(error),
        }
    }

    /// { true }
    /// fn pair(&self) -> LanguagePair
    /// { ret is the request language pair }
    pub const fn pair(&self) -> LanguagePair {
        self.pair
    }

    /// { true }
    /// fn text(&self) -> &NonEmptyText
    /// { ret is the request text }
    pub const fn text(&self) -> &NonEmptyText {
        &self.text
    }
}

/// Translation output guaranteed to be non-empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Translation {
    text: NonEmptyText,
}

impl Translation {
    /// { text is non-empty and bounded }
    /// fn new(text: NonEmptyText) -> Self
    /// { ret.text() == text.as_str() }
    pub const fn new(text: NonEmptyText) -> Self {
        Self { text }
    }

    /// { true }
    /// fn text(&self) -> &NonEmptyText
    /// { ret is the translated text }
    pub const fn text(&self) -> &NonEmptyText {
        &self.text
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Language, LanguagePair, LanguagePairError, MAX_TEXT_CHARS, NonEmptyText, TextError,
        TranslateRequest,
    };

    #[test]
    fn language_codes_round_trip_for_supported_languages() {
        let languages = [
            Language::English,
            Language::Russian,
            Language::Thai,
            Language::Vietnamese,
            Language::Japanese,
        ];

        for language in languages {
            assert_eq!(Language::from_iso_639_1(language.iso_639_1()), Ok(language));
        }
    }

    #[test]
    fn unknown_language_code_is_rejected() {
        let error = Language::from_iso_639_1("de");
        assert!(matches!(error, Err(ref item) if item.code() == "de"));
    }

    #[test]
    fn same_language_pair_is_rejected() {
        assert_eq!(
            LanguagePair::new(Language::English, Language::English),
            Err(LanguagePairError::SameLanguage(Language::English))
        );
    }

    #[test]
    fn different_language_pair_is_accepted() {
        let pair = LanguagePair::new(Language::English, Language::Russian);

        assert!(matches!(
            pair,
            Ok(item)
                if item.source() == Language::English
                    && item.target() == Language::Russian
        ));
    }

    #[test]
    fn empty_text_is_rejected() {
        assert_eq!(NonEmptyText::new("   "), Err(TextError::Empty));
    }

    #[test]
    fn oversized_text_is_rejected() {
        let oversized = "a".repeat(MAX_TEXT_CHARS + 1);

        assert_eq!(
            NonEmptyText::new(oversized),
            Err(TextError::TooLong {
                actual_chars: MAX_TEXT_CHARS + 1,
                max_chars: MAX_TEXT_CHARS,
            })
        );
    }

    #[test]
    fn request_reuses_pair_invariant() -> Result<(), Box<dyn std::error::Error>> {
        let text = NonEmptyText::new("hello")?;

        assert_eq!(
            TranslateRequest::new(Language::Thai, Language::Thai, text),
            Err(LanguagePairError::SameLanguage(Language::Thai))
        );

        Ok(())
    }
}
