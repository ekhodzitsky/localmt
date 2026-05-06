//! Translation engine boundary for localmt.

use core::fmt;

use localmt_core::{LanguagePair, NonEmptyText, TextError, TranslateRequest, Translation};

/// Common translation engine interface.
pub trait TranslatorEngine {
    /// { request has a valid language pair and non-empty text }
    /// fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
    /// { ret is Ok only when translated text is non-empty }
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>;
}

/// Deterministic engine for tests and CLI smoke checks.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockEngine;

impl TranslatorEngine for MockEngine {
    fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError> {
        let pair = request.pair();
        let value = format!(
            "[{}->{}] {}",
            pair.source().m2m100_code(),
            pair.target().m2m100_code(),
            request.text().as_str()
        );
        let text = NonEmptyText::new(value).map_err(TranslationError::InvalidOutput)?;

        Ok(Translation::new(text))
    }
}

/// Error returned by translation engines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TranslationError {
    /// The backend is not ready to translate.
    EngineUnavailable(String),
    /// The backend cannot translate this language pair.
    UnsupportedPair(LanguagePair),
    /// The backend produced text that violates output invariants.
    InvalidOutput(TextError),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EngineUnavailable(reason) => write!(formatter, "engine unavailable: {reason}"),
            Self::UnsupportedPair(pair) => write!(
                formatter,
                "unsupported translation pair: {} -> {}",
                pair.source(),
                pair.target()
            ),
            Self::InvalidOutput(error) => write!(formatter, "invalid translation output: {error}"),
        }
    }
}

impl std::error::Error for TranslationError {}

#[cfg(test)]
mod tests {
    use localmt_core::{Language, NonEmptyText, TranslateRequest};

    use super::{MockEngine, TranslatorEngine};

    #[test]
    fn mock_engine_returns_deterministic_non_empty_text() -> Result<(), Box<dyn std::error::Error>>
    {
        let text = NonEmptyText::new("Where is the station?")?;
        let request = TranslateRequest::new(Language::English, Language::Russian, text)?;

        let translation = MockEngine.translate(&request);

        assert!(matches!(
            translation,
            Ok(ref item) if item.text().as_str() == "[en->ru] Where is the station?"
        ));

        Ok(())
    }
}
