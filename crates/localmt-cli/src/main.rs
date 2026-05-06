use core::fmt;
use std::process::ExitCode;

use localmt::{Language, MockEngine, NonEmptyText, TranslateRequest, Translator};
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

    let source = parse_language_code(first)?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let request_text = NonEmptyText::new(text).map_err(CliError::InvalidText)?;
    let request =
        TranslateRequest::new(source, target, request_text).map_err(CliError::InvalidPair)?;
    let translator = Translator::new(MockEngine);
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
        }
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::run;

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn cli_routes_valid_request_through_mock_engine() {
        let args = [
            "localmt".to_owned(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello".to_owned(),
        ];

        assert!(matches!(
            run(args.into_iter()),
            Ok(ref output) if output == "[en->ru] hello"
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

    fn create_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-cli-test-{}-{nanos}-{counter}",
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
