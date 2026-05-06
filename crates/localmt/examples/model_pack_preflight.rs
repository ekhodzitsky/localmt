use core::fmt;
use std::process::ExitCode;

use localmt::{
    OfflineTranslatorAssets, OfflineTranslatorAssetsError, OfflineTranslatorAssetsSummary,
};

/// { process args are provided by the host environment }
/// fn main() -> ExitCode
/// { ret is SUCCESS only when the model-pack preflight summary was printed }
fn main() -> ExitCode {
    match run(std::env::args()) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// { args contains program name followed by CLI arguments }
/// fn run(args: impl Iterator<Item = String>) -> Result<String, PreflightError>
/// { ret is Ok only when exactly one model-pack path preflights successfully }
fn run(mut args: impl Iterator<Item = String>) -> Result<String, PreflightError> {
    let _program = args.next();
    let path = args
        .next()
        .ok_or(PreflightError::MissingArgument("MODEL_PACK"))?;

    if args.next().is_some() {
        return Err(PreflightError::TooManyArguments);
    }

    let assets = OfflineTranslatorAssets::from_model_pack_path(path)
        .map_err(PreflightError::OfflineAssets)?;
    Ok(format_summary(&assets.summary()))
}

/// { summary was produced by a successful offline-translator preflight }
/// fn format_summary(summary: &OfflineTranslatorAssetsSummary) -> String
/// { ret is a stable human-readable no-inference preflight summary }
fn format_summary(summary: &OfflineTranslatorAssetsSummary) -> String {
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

    lines.join("\n")
}

#[derive(Debug)]
enum PreflightError {
    MissingArgument(&'static str),
    TooManyArguments,
    OfflineAssets(OfflineTranslatorAssetsError),
}

impl fmt::Display for PreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArgument(name) => write!(formatter, "missing argument: {name}"),
            Self::TooManyArguments => formatter.write_str("too many arguments"),
            Self::OfflineAssets(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for PreflightError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingArgument(_) | Self::TooManyArguments => None,
            Self::OfflineAssets(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;

    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const TOKENIZER_SHA256: &str =
        "38395078aa8c0af1657b8fc788f358d57e5f5fea99c8cdc004198e3c6fffbe71";
    const GENERATION_CONFIG_SHA256: &str =
        "a8a99326d564beb1fc16cb59526dd5ed7b5fd673f969591f47845e17fbed401d";
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
    fn preflight_example_prints_asset_summary() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_pack()?;
        let args = [
            "model_pack_preflight".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("planned: m2m100-418m-int8"));
        assert!(output.contains("tokenizer:"));
        assert!(output.contains("encoder:"));
        assert!(output.contains("decoder:"));
        assert!(output.contains("generation_config: parsed"));
        assert!(output.contains("max_new_tokens: 32"));
        Ok(())
    }

    fn create_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(root.join("generation.json"), GENERATION_CONFIG)?;
        fs::write(root.join("manifest.json"), manifest_json())?;
        Ok(root)
    }

    fn create_temp_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "localmt-preflight-example-test-{}-{nanos}-{counter}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    fn manifest_json() -> String {
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
        )
    }
}
