use core::fmt;
use std::process::ExitCode;

use localmt::{Language, MockEngine, NonEmptyText, TranslateRequest, Translator};

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
    let source = parse_language(args.next(), "FROM")?;
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

/// { value is an optional language code }
/// fn parse_language(value: `Option<String>`, name: &'static str) -> Result<Language, CliError>
/// { ret is Ok only when value is a supported ISO 639-1 code }
fn parse_language(value: Option<String>, name: &'static str) -> Result<Language, CliError> {
    let code = value.ok_or(CliError::MissingArgument(name))?;

    Language::from_iso_639_1(&code).map_err(CliError::InvalidLanguage)
}

#[derive(Debug, Eq, PartialEq)]
enum CliError {
    MissingArgument(&'static str),
    TooManyArguments,
    InvalidLanguage(localmt::LanguageCodeError),
    InvalidPair(localmt::LanguagePairError),
    InvalidText(localmt::TextError),
    Translate(localmt::TranslationError),
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
        }
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn cli_routes_valid_request_through_mock_engine() {
        let args = [
            "localmt".to_owned(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello".to_owned(),
        ];

        assert_eq!(run(args.into_iter()), Ok("[en->ru] hello".to_owned()));
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
}
