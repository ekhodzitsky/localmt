use core::fmt;
use std::process::ExitCode;

use localmt::{
    Language, MockTokenGenerator, MockTokenizer, NonEmptyText, OfflineTranslatorAssets,
    TranslateRequest, TranslationPipeline, Translator,
};
use localmt_models::{Discovered, ModelPack};

fn main() -> ExitCode {
    match run(std::env::args()) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

/// { args contains process argv items }
/// fn run(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when FROM TO TEXT are valid }
fn run(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let _program = args.next();
    let first = args.next().ok_or(CliError::MissingArgument("FROM"))?;
    if first == "model" {
        return run_model(args);
    }
    if first == "bench" {
        return run_bench(args);
    }

    let source = parse_language_code(first)?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let request_text = NonEmptyText::new(text).map_err(CliError::InvalidText)?;
    let request =
        TranslateRequest::new(source, target, request_text).map_err(CliError::InvalidPair)?;
    let translator = Translator::new(TranslationPipeline::new(MockTokenizer, MockTokenGenerator));
    let translation = translator
        .translate(&request)
        .map_err(CliError::Translate)?;

    Ok(translation.text().as_str().to_owned())
}

/// { args contains model subcommand arguments }
/// fn run_model(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when the model command and model-pack path are valid }
fn run_model(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let command = args
        .next()
        .ok_or(CliError::MissingArgument("MODEL_COMMAND"))?;
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    match command.as_str() {
        "inspect" => inspect_model(path),
        "verify" => verify_model(path),
        "plan" => plan_model(path),
        "hash" => hash_model_file(path),
        _ => Err(CliError::UnknownModelCommand(command)),
    }
}

/// { path is a model-pack root candidate }
/// fn inspect_model(path: String) -> Result<String, CliError>
/// { ret summarizes the discovered manifest }
fn inspect_model(path: String) -> Result<String, CliError> {
    let pack = ModelPack::<Discovered>::discover(path).map_err(CliError::ModelPack)?;
    let manifest = pack.manifest();
    let languages = manifest
        .languages()
        .iter()
        .map(|language| language.iso_639_1())
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        "model_id: {}\nversion: {}\narchitecture: {}\nruntime: {}\nlicense: {}\nlanguages: {}",
        manifest.model_id(),
        manifest.version(),
        manifest.architecture(),
        manifest.runtime(),
        manifest.license(),
        languages
    ))
}

/// { path is a model-pack root candidate }
/// fn verify_model(path: String) -> Result<String, CliError>
/// { ret is Ok only when the model pack verifies }
fn verify_model(path: String) -> Result<String, CliError> {
    let pack = ModelPack::<Discovered>::discover(path)
        .and_then(ModelPack::verify)
        .map_err(CliError::ModelPack)?;

    Ok(format!("verified: {}", pack.manifest().model_id()))
}

/// { path is a local model-pack file candidate }
/// fn hash_model_file(path: String) -> Result<String, CliError>
/// { ret is the manifest-ready lowercase SHA-256 digest }
fn hash_model_file(path: String) -> Result<String, CliError> {
    let digest = localmt::Sha256Digest::from_file(path).map_err(CliError::ModelPack)?;

    Ok(format!("sha256: {digest}"))
}

/// { path is a model-pack root candidate }
/// fn plan_model(path: String) -> Result<String, CliError>
/// { ret summarizes verified SDK asset planning without loading inference sessions }
fn plan_model(path: String) -> Result<String, CliError> {
    let assets =
        OfflineTranslatorAssets::from_model_pack_path(path).map_err(CliError::OfflineAssets)?;
    let summary = assets.summary();

    let decoder_with_past = summary
        .decoder_with_past_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "absent".to_owned());
    let mut lines = vec![
        format!("planned: {}", summary.model_id()),
        format!("tokenizer: {}", summary.tokenizer_path().display()),
        format!("encoder: {}", summary.encoder_path().display()),
        format!("decoder: {}", summary.decoder_path().display()),
        format!("decoder_with_past: {decoder_with_past}"),
    ];

    match summary.generation_config() {
        Some(config) => {
            lines.push("generation_config: parsed".to_owned());
            lines.push(format!(
                "max_new_tokens: {}",
                config.max_new_tokens().value()
            ));
        }
        None => lines.push("generation_config: absent".to_owned()),
    }

    Ok(lines.join("\n"))
}

/// { args contains benchmark command arguments }
/// fn run_bench(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when profile and model-pack arguments are valid }
fn run_bench(args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let options = BenchOptions::parse(args)?;
    let pack = ModelPack::<Discovered>::discover(options.model_pack_path)
        .and_then(ModelPack::verify)
        .map_err(CliError::ModelPack)?;
    let report = localmt_bench::MockBenchmarkRunner::new(options.profile)
        .run(&pack)
        .map_err(CliError::Benchmark)?;

    Ok(format!(
        "profile: {}\nruntime: {}\nmodel_id: {}\nscenarios: {}\ntranslations: {}\ntotal_ms: {}\nwarm_translate_ms: {}",
        report.profile().as_str(),
        report.runtime(),
        report.model_id(),
        report.scenario_count(),
        report.translation_count(),
        report.total_ms(),
        report.warm_translate_ms()
    ))
}

/// { value is an optional language code }
/// fn parse_language(value: `Option<String>`, name: &'static str) -> Result<Language, CliError>
/// { ret is Ok only when value is a supported ISO 639-1 code }
fn parse_language(value: Option<String>, name: &'static str) -> Result<Language, CliError> {
    let code = value.ok_or(CliError::MissingArgument(name))?;

    parse_language_code(code)
}

/// { code may be any string }
/// fn parse_language_code(code: String) -> Result<Language, CliError>
/// { ret is Ok only when code is a supported ISO 639-1 code }
fn parse_language_code(code: String) -> Result<Language, CliError> {
    Language::from_iso_639_1(&code).map_err(CliError::InvalidLanguage)
}

#[derive(Debug)]
enum CliError {
    MissingArgument(&'static str),
    TooManyArguments,
    InvalidLanguage(localmt::LanguageCodeError),
    InvalidPair(localmt::LanguagePairError),
    InvalidText(localmt::TextError),
    Translate(localmt::TranslationError),
    UnknownModelCommand(String),
    ModelPack(localmt_models::ModelPackError),
    OfflineAssets(localmt::OfflineTranslatorAssetsError),
    Benchmark(localmt_bench::BenchmarkError),
    InvalidBenchArguments(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArgument(name) => {
                write!(
                    formatter,
                    "missing argument: {name}\nusage: localmt FROM TO TEXT"
                )
            }
            Self::TooManyArguments => {
                formatter.write_str("too many arguments\nusage: localmt FROM TO TEXT")
            }
            Self::InvalidLanguage(error) => write!(formatter, "{error}"),
            Self::InvalidPair(error) => write!(formatter, "{error}"),
            Self::InvalidText(error) => write!(formatter, "{error}"),
            Self::Translate(error) => write!(formatter, "{error}"),
            Self::UnknownModelCommand(command) => {
                write!(formatter, "unknown model command: {command}")
            }
            Self::ModelPack(error) => write!(formatter, "{error}"),
            Self::OfflineAssets(error) => write!(formatter, "{error}"),
            Self::Benchmark(error) => write!(formatter, "{error}"),
            Self::InvalidBenchArguments(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for CliError {}

struct BenchOptions {
    profile: localmt_bench::DeviceProfile,
    model_pack_path: String,
}

impl BenchOptions {
    /// { args contains benchmark CLI arguments }
    /// fn parse(args: impl Iterator<Item = String>) -> Result<Self, CliError>
    /// { ret is Ok only when --profile and --model-pack are both present once }
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut profile = None;
        let mut model_pack_path = None;
        let mut args = args;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--profile" => {
                    let value = args.next().ok_or(CliError::MissingArgument("PROFILE"))?;
                    profile = Some(
                        localmt_bench::DeviceProfile::parse(&value).map_err(CliError::Benchmark)?,
                    );
                }
                "--model-pack" => {
                    model_pack_path =
                        Some(args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?);
                }
                _ => {
                    return Err(CliError::InvalidBenchArguments(format!(
                        "unknown bench argument: {flag}"
                    )));
                }
            }
        }

        Ok(Self {
            profile: profile.ok_or(CliError::MissingArgument("PROFILE"))?,
            model_pack_path: model_pack_path.ok_or(CliError::MissingArgument("MODEL_PACK"))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::run;

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const GENERATION_CONFIG_SHA256: &str =
        "a8a99326d564beb1fc16cb59526dd5ed7b5fd673f969591f47845e17fbed401d";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    const GENERATION_CONFIG: &str = r#"{
  "max_new_tokens": 32,
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}"#;
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn cli_routes_valid_request_through_mock_pipeline() {
        let args = [
            "localmt".to_owned(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello".to_owned(),
        ];

        assert!(matches!(
            run(args.into_iter()),
            Ok(ref output) if output == "hello"
        ));
    }

    #[test]
    fn cli_rejects_same_language_pair() {
        let args = [
            "localmt".to_owned(),
            "en".to_owned(),
            "en".to_owned(),
            "hello".to_owned(),
        ];

        let result = run(args.into_iter());

        assert!(matches!(result, Err(ref error) if error.to_string().contains("both English")));
    }

    #[test]
    fn cli_inspects_model_pack_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "inspect".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("model_id: m2m100-418m-int8"));
        assert!(output.contains("runtime: onnx-runtime"));
        assert!(output.contains("languages: en, ru, th, vi, ja"));
        Ok(())
    }

    #[test]
    fn cli_verifies_model_pack_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "verify".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert_eq!(output, "verified: m2m100-418m-int8");
        Ok(())
    }

    #[test]
    fn cli_runs_mock_pipeline_benchmark_for_verified_pack() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_pack()?;
        let args = [
            "localmt".to_owned(),
            "bench".to_owned(),
            "--profile".to_owned(),
            "xiaomi17".to_owned(),
            "--model-pack".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("profile: xiaomi17"));
        assert!(output.contains("runtime: mock-pipeline"));
        assert!(output.contains("model_id: m2m100-418m-int8"));
        assert!(output.contains("scenarios: 10"));
        Ok(())
    }

    #[test]
    fn cli_plans_verified_model_pack_assets() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "plan".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("planned: m2m100-418m-int8"));
        assert!(output.contains("tokenizer:"));
        assert!(output.contains("tokenizer.json"));
        assert!(output.contains("encoder:"));
        assert!(output.contains("encoder.onnx"));
        assert!(output.contains("decoder:"));
        assert!(output.contains("decoder.onnx"));
        assert!(output.contains("generation_config: parsed"));
        assert!(output.contains("max_new_tokens: 32"));
        Ok(())
    }

    #[test]
    fn cli_hashes_model_pack_file_for_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        let path = root.join("encoder.onnx");
        fs::write(&path, "encoder\n")?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "hash".to_owned(),
            path.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert_eq!(output, format!("sha256: {ENCODER_SHA256}"));
        Ok(())
    }

    fn create_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
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

    fn create_plannable_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(root.join("generation.json"), GENERATION_CONFIG)?;
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
    {{ "path": "decoder.onnx", "kind": "decoder", "sha256": "{DECODER_SHA256}" }},
    {{ "path": "tokenizer.json", "kind": "tokenizer", "sha256": "{TOKENIZER_SHA256}" }},
    {{ "path": "generation.json", "kind": "generation_config", "sha256": "{GENERATION_CONFIG_SHA256}" }}
  ]
}}"#
            ),
        )?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-cli-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }
}
