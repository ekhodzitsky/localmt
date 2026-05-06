//! Public facade for the localmt translation library.

pub use localmt_bench::{BenchReport, BenchmarkError, DeviceProfile, MockBenchmarkRunner};
pub use localmt_core::{
    Language, LanguageCodeError, LanguagePair, LanguagePairError, MAX_TEXT_CHARS, NonEmptyText,
    TextError, TranslateRequest, Translation,
};
pub use localmt_engine::{MockEngine, TranslationError, TranslatorEngine};
pub use localmt_engine_ort::{OrtEngine, OrtEngineError, OrtModelRole, OrtSessionPlan};
pub use localmt_models::{
    Discovered, ModelArchitecture, ModelFile, ModelFileKind, ModelFileRole, ModelId, ModelLicense,
    ModelManifest, ModelPack, ModelPackError, ModelPackVersion, ModelRelativePath, ModelRuntime,
    Sha256Digest, Verified,
};
pub use localmt_pipeline::{
    GeneratorAssetPlan, MockTokenGenerator, PipelineError, TokenGenerator, TokenGeneratorError,
    TranslationPipeline,
};
pub use localmt_tokenizer::{
    MAX_TOKENS, MockTokenizer, TokenId, TokenSequence, TokenizerAssetPlan, TokenizerEngine,
    TokenizerError, TokenizerInput, TokenizerOutput,
};

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
    use super::{Language, MockEngine, NonEmptyText, TranslateRequest, Translator};

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
}
