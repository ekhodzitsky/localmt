//! Benchmark skeleton for localmt.

use core::fmt;
use std::time::Instant;

use localmt_core::{Language, NonEmptyText, TranslateRequest};
use localmt_engine::{TranslationError, TranslatorEngine};
use localmt_models::{ModelPack, Verified};
use localmt_pipeline::{MockTokenGenerator, TranslationPipeline};
use localmt_tokenizer::MockTokenizer;

const MOCK_PIPELINE_RUNTIME: &str = "mock-pipeline";
const SMOKE_TEXT: &str = "Where is the station?";
const SCENARIOS: [(Language, Language); 10] = [
    (Language::English, Language::Russian),
    (Language::Russian, Language::English),
    (Language::English, Language::Thai),
    (Language::Thai, Language::English),
    (Language::English, Language::Vietnamese),
    (Language::Vietnamese, Language::English),
    (Language::English, Language::Japanese),
    (Language::Japanese, Language::English),
    (Language::Russian, Language::Thai),
    (Language::Japanese, Language::Vietnamese),
];

/// Supported benchmark device profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceProfile {
    /// Xiaomi 17 Android arm64-v8a target.
    Xiaomi17,
}

impl DeviceProfile {
    /// { value may be any string }
    /// fn parse(value: &str) -> Result<Self, BenchmarkError>
    /// { ret is Ok only for supported profile ids }
    pub fn parse(value: &str) -> Result<Self, BenchmarkError> {
        match value {
            "xiaomi17" => Ok(Self::Xiaomi17),
            _ => Err(BenchmarkError::UnknownProfile(value.to_owned())),
        }
    }

    /// { true }
    /// fn as_str(self) -> &'static str
    /// { ret is the stable profile id }
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Xiaomi17 => "xiaomi17",
        }
    }

    /// { true }
    /// fn android_abi(self) -> &'static str
    /// { ret is the Android native ABI for profile }
    pub const fn android_abi(self) -> &'static str {
        match self {
            Self::Xiaomi17 => "arm64-v8a",
        }
    }

    /// { true }
    /// fn ram_class_gib(self) -> u16
    /// { ret is the expected RAM class in GiB for profile }
    pub const fn ram_class_gib(self) -> u16 {
        match self {
            Self::Xiaomi17 => 12,
        }
    }

    /// { true }
    /// fn preferred_runtime(self) -> &'static str
    /// { ret is the preferred inference runtime hint for profile }
    pub const fn preferred_runtime(self) -> &'static str {
        match self {
            Self::Xiaomi17 => "llama.cpp",
        }
    }
}

impl fmt::Display for DeviceProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Mock benchmark runner used before real inference exists.
#[derive(Clone, Copy, Debug)]
pub struct MockBenchmarkRunner {
    profile: DeviceProfile,
}

impl MockBenchmarkRunner {
    /// { profile is supported }
    /// fn new(profile: DeviceProfile) -> Self
    /// { ret runs benchmark scenarios for profile }
    pub const fn new(profile: DeviceProfile) -> Self {
        Self { profile }
    }

    /// { pack has verified manifest, files, and checksums }
    /// fn run(&self, pack: &`ModelPack<Verified>`) -> Result<BenchReport, BenchmarkError>
    /// { ret contains timing for all fixed smoke scenarios }
    pub fn run(&self, pack: &ModelPack<Verified>) -> Result<BenchReport, BenchmarkError> {
        let total_start = Instant::now();
        let engine = TranslationPipeline::new(MockTokenizer, MockTokenGenerator);
        let translate_start = Instant::now();
        let mut translations = 0_usize;

        for (source, target) in SCENARIOS {
            let text = NonEmptyText::new(SMOKE_TEXT).map_err(BenchmarkError::InvalidText)?;
            let request =
                TranslateRequest::new(source, target, text).map_err(BenchmarkError::InvalidPair)?;
            let translation = engine.translate(&request)?;
            let _text = translation.text();
            translations += 1;
        }

        let warm_translate_ms = translate_start.elapsed().as_millis();
        let total_ms = total_start.elapsed().as_millis();

        Ok(BenchReport {
            profile: self.profile,
            runtime: MOCK_PIPELINE_RUNTIME,
            model_id: pack.manifest().model_id().as_str().to_owned(),
            scenario_count: SCENARIOS.len(),
            translation_count: translations,
            total_ms,
            warm_translate_ms,
        })
    }
}

/// Benchmark report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchReport {
    profile: DeviceProfile,
    runtime: &'static str,
    model_id: String,
    scenario_count: usize,
    translation_count: usize,
    total_ms: u128,
    warm_translate_ms: u128,
}

impl BenchReport {
    /// { true }
    /// fn profile(&self) -> DeviceProfile
    /// { ret is the benchmark profile }
    pub const fn profile(&self) -> DeviceProfile {
        self.profile
    }

    /// { true }
    /// fn runtime(&self) -> &'static str
    /// { ret is the benchmark runtime label }
    pub const fn runtime(&self) -> &'static str {
        self.runtime
    }

    /// { true }
    /// fn android_abi(&self) -> &'static str
    /// { ret is the benchmark profile Android ABI }
    pub const fn android_abi(&self) -> &'static str {
        self.profile.android_abi()
    }

    /// { true }
    /// fn ram_class_gib(&self) -> u16
    /// { ret is the benchmark profile RAM class }
    pub const fn ram_class_gib(&self) -> u16 {
        self.profile.ram_class_gib()
    }

    /// { true }
    /// fn preferred_runtime(&self) -> &'static str
    /// { ret is the benchmark profile preferred runtime hint }
    pub const fn preferred_runtime(&self) -> &'static str {
        self.profile.preferred_runtime()
    }

    /// { true }
    /// fn model_id(&self) -> &str
    /// { ret is the verified model id }
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// { true }
    /// fn scenario_count(&self) -> usize
    /// { ret is the number of benchmark scenarios }
    pub const fn scenario_count(&self) -> usize {
        self.scenario_count
    }

    /// { true }
    /// fn translation_count(&self) -> usize
    /// { ret is the number of completed translations }
    pub const fn translation_count(&self) -> usize {
        self.translation_count
    }

    /// { true }
    /// fn total_ms(&self) -> u128
    /// { ret is total elapsed benchmark wall time in milliseconds }
    pub const fn total_ms(&self) -> u128 {
        self.total_ms
    }

    /// { true }
    /// fn warm_translate_ms(&self) -> u128
    /// { ret is elapsed translation wall time in milliseconds }
    pub const fn warm_translate_ms(&self) -> u128 {
        self.warm_translate_ms
    }
}

/// Benchmark error.
#[derive(Debug)]
pub enum BenchmarkError {
    /// Device profile is unknown.
    UnknownProfile(String),
    /// Fixed benchmark text violated text invariants.
    InvalidText(localmt_core::TextError),
    /// Fixed benchmark pair violated pair invariants.
    InvalidPair(localmt_core::LanguagePairError),
    /// Translation engine failed.
    Translation(TranslationError),
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProfile(profile) => write!(formatter, "unknown device profile: {profile}"),
            Self::InvalidText(error) => write!(formatter, "{error}"),
            Self::InvalidPair(error) => write!(formatter, "{error}"),
            Self::Translation(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for BenchmarkError {}

impl From<TranslationError> for BenchmarkError {
    fn from(error: TranslationError) -> Self {
        Self::Translation(error)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use localmt_models::{Discovered, ModelPack};

    use crate::{DeviceProfile, MockBenchmarkRunner};

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn mock_benchmark_runs_xiaomi17_scenarios() -> Result<(), Box<dyn std::error::Error>> {
        let pack = ModelPack::<Discovered>::discover(create_pack()?)?.verify()?;
        let report = MockBenchmarkRunner::new(DeviceProfile::Xiaomi17).run(&pack)?;

        assert_eq!(report.profile(), DeviceProfile::Xiaomi17);
        assert_eq!(report.runtime(), "mock-pipeline");
        assert_eq!(report.android_abi(), "arm64-v8a");
        assert_eq!(report.ram_class_gib(), 12);
        assert_eq!(report.preferred_runtime(), "llama.cpp");
        assert_eq!(report.model_id(), "m2m100-418m-int8");
        assert_eq!(report.scenario_count(), 10);
        assert_eq!(report.translation_count(), 10);
        assert!(report.warm_translate_ms() <= report.total_ms());
        Ok(())
    }

    #[test]
    fn device_profile_parses_xiaomi17() {
        assert!(matches!(
            DeviceProfile::parse("xiaomi17"),
            Ok(DeviceProfile::Xiaomi17)
        ));
        assert_eq!(DeviceProfile::Xiaomi17.android_abi(), "arm64-v8a");
        assert_eq!(DeviceProfile::Xiaomi17.ram_class_gib(), 12);
        assert_eq!(DeviceProfile::Xiaomi17.preferred_runtime(), "llama.cpp");
    }

    #[test]
    fn device_profile_rejects_unknown_profile() {
        let error = DeviceProfile::parse("pixel");

        assert!(error.is_err());
    }

    fn create_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-bench-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(
            root.join("manifest.json"),
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
    {{ "path": "encoder.onnx", "kind": "encoder", "sha256": "{ENCODER_SHA256}" }},
    {{ "path": "tokenizer.json", "kind": "tokenizer", "sha256": "{TOKENIZER_SHA256}" }}
  ]
}}"#
            ),
        )?;
        Ok(root)
    }
}
