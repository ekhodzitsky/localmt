use core::fmt;
use std::path::Path;
use std::process::ExitCode;
use std::ptr;

#[cfg(feature = "hf-tokenizers")]
use localmt::{HfTokenizer, TokenizerEngine, TokenizerInput};
use localmt::{
    Language, LanguagePair, MockTokenGenerator, MockTokenizer, ModelArchitecture, ModelFile,
    ModelFileRole, ModelId, ModelLicense, ModelManifest, ModelPackVersion, ModelRelativePath,
    ModelRuntime, NonEmptyText, OfflineTranslatorAssets, OfflineTranslatorPlan, Sha256Digest,
    TranslateRequest, TranslationPipeline, Translator,
};
use localmt_models::{Discovered, ModelPack};

const HELP_TEXT: &str = "\
localmt development CLI

usage:
  localmt FROM TO TEXT
  localmt model hash FILE
  localmt model inspect PACK
  localmt model verify PACK
  localmt model plan PACK
  localmt model doctor PACK
  localmt model runtime-config PACK
  localmt model tokenize PACK FROM TO TEXT
  localmt model write-manifest PACK MODEL_ID VERSION ARCHITECTURE RUNTIME LICENSE
  localmt ffi smoke PACK FROM TO TEXT
  localmt ffi startup
  localmt ffi header
  localmt ffi runtime-config PACK
  localmt ffi hf-smoke PACK FROM TO TEXT
  localmt ffi ort-smoke PACK
  localmt ffi ort-translate-smoke PACK FROM TO TEXT
  localmt bench --profile xiaomi17 --model-pack PACK
";

const FFI_HELP_TEXT: &str = "\
localmt ffi commands

usage:
  localmt ffi smoke PACK FROM TO TEXT
  localmt ffi startup
  localmt ffi header
  localmt ffi runtime-config PACK
  localmt ffi hf-smoke PACK FROM TO TEXT
  localmt ffi ort-smoke PACK
  localmt ffi ort-translate-smoke PACK FROM TO TEXT
";

const MODEL_HELP_TEXT: &str = "\
localmt model commands

usage:
  localmt model hash FILE
  localmt model inspect PACK
  localmt model verify PACK
  localmt model plan PACK
  localmt model doctor PACK
  localmt model runtime-config PACK
  localmt model tokenize PACK FROM TO TEXT
  localmt model write-manifest PACK MODEL_ID VERSION ARCHITECTURE RUNTIME LICENSE
";

const BENCH_HELP_TEXT: &str = "\
localmt benchmark command

usage:
  localmt bench --profile xiaomi17 --model-pack PACK

notes:
  runtime: mock-pipeline
";

const DOCTOR_SMOKE_TEXT: &str = "hello offline";
const FFI_HEADER_TEXT: &str = include_str!("../../localmt-ffi/include/localmt_ffi.h");
const STANDARD_MODEL_FILES: [StandardModelFile; 7] = [
    StandardModelFile::required("encoder.onnx", ModelFileRole::Encoder),
    StandardModelFile::required("decoder.onnx", ModelFileRole::Decoder),
    StandardModelFile::required("tokenizer.json", ModelFileRole::Tokenizer),
    StandardModelFile::optional("decoder-with-past.onnx", ModelFileRole::DecoderWithPast),
    StandardModelFile::optional("vocab.txt", ModelFileRole::Vocabulary),
    StandardModelFile::optional("config.json", ModelFileRole::Config),
    StandardModelFile::optional("generation.json", ModelFileRole::GenerationConfig),
];

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
    if is_help(&first) {
        if args.next().is_some() {
            return Err(CliError::TooManyArguments);
        }

        return Ok(HELP_TEXT.to_owned());
    }
    if first == "model" {
        return run_model(args);
    }
    if first == "bench" {
        return run_bench(args);
    }
    if first == "ffi" {
        return run_ffi(args);
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
    if is_help(&command) {
        if args.next().is_some() {
            return Err(CliError::TooManyArguments);
        }

        return Ok(MODEL_HELP_TEXT.to_owned());
    }
    match command.as_str() {
        "inspect" => inspect_model(single_model_path(args)?),
        "verify" => verify_model(single_model_path(args)?),
        "plan" => plan_model(single_model_path(args)?),
        "doctor" => doctor_model(single_model_path(args)?),
        "runtime-config" => runtime_config_model(single_model_path(args)?),
        "tokenize" => tokenize_model(args),
        "hash" => hash_model_file(single_model_path(args)?),
        "write-manifest" => write_manifest(args),
        _ => Err(CliError::UnknownModelCommand(command)),
    }
}

/// { args contains FFI subcommand arguments }
/// fn run_ffi(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when the FFI command and arguments are valid }
fn run_ffi(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let command = args
        .next()
        .ok_or(CliError::MissingArgument("FFI_COMMAND"))?;
    if is_help(&command) {
        if args.next().is_some() {
            return Err(CliError::TooManyArguments);
        }

        return Ok(FFI_HELP_TEXT.to_owned());
    }

    match command.as_str() {
        "smoke" => run_ffi_smoke(args),
        "startup" => ffi_startup(args),
        "header" => ffi_header(args),
        "runtime-config" => run_ffi_runtime_config(args),
        "hf-smoke" => run_ffi_hf_smoke(args),
        "ort-smoke" => run_ffi_ort_smoke(args),
        "ort-translate-smoke" => run_ffi_ort_translate_smoke(args),
        _ => Err(CliError::UnknownFfiCommand(command)),
    }
}

/// { args contains no remaining arguments }
/// fn ffi_startup(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is the Android startup contract visible through the FFI buffer ABI }
fn ffi_startup(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let output = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_startup_summary(output_ptr, output_capacity, written_len)
    })?;

    String::from_utf8(output).map_err(CliError::FfiOutputUtf8)
}

/// { args contains no remaining arguments }
/// fn ffi_header(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is the bundled C ABI header text }
fn ffi_header(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    Ok(FFI_HEADER_TEXT.to_owned())
}

/// { args contains one model path argument }
/// fn single_model_path(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when args contains exactly one path }
fn single_model_path(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    Ok(path)
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
    let digest = Sha256Digest::from_file(path).map_err(CliError::ModelPack)?;

    Ok(format!("sha256: {digest}"))
}

/// { args contains write-manifest command arguments }
/// fn write_manifest(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when required standard files exist and manifest JSON serializes }
fn write_manifest(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let pack_path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;
    let model_id = args.next().ok_or(CliError::MissingArgument("MODEL_ID"))?;
    let version = args.next().ok_or(CliError::MissingArgument("VERSION"))?;
    let architecture = args
        .next()
        .ok_or(CliError::MissingArgument("ARCHITECTURE"))?;
    let runtime = args.next().ok_or(CliError::MissingArgument("RUNTIME"))?;
    let license = args.next().ok_or(CliError::MissingArgument("LICENSE"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let manifest = ModelManifest::new_current(
        ModelId::new(model_id).map_err(CliError::ModelPack)?,
        ModelPackVersion::new(version).map_err(CliError::ModelPack)?,
        ModelArchitecture::new(architecture).map_err(CliError::ModelPack)?,
        ModelRuntime::new(runtime).map_err(CliError::ModelPack)?,
        ModelLicense::new(license).map_err(CliError::ModelPack)?,
        default_model_languages(),
        standard_model_files(Path::new(&pack_path))?,
    )
    .map_err(CliError::ModelPack)?;

    manifest
        .to_json_string_pretty()
        .map_err(CliError::ModelPack)
}

/// { true }
/// fn default_model_languages() -> `Vec<Language>`
/// { ret contains the first supported model-pack language set }
fn default_model_languages() -> Vec<Language> {
    vec![
        Language::English,
        Language::Russian,
        Language::Thai,
        Language::Vietnamese,
        Language::Japanese,
    ]
}

/// { root is a local model-pack directory candidate }
/// fn standard_model_files(root: &Path) -> Result<`Vec<ModelFile>`, CliError>
/// { ret contains required and present optional standard model-pack files with digests }
fn standard_model_files(root: &Path) -> Result<Vec<ModelFile>, CliError> {
    let mut files = Vec::new();
    for item in STANDARD_MODEL_FILES {
        let path = root.join(item.path);
        if path.is_file() {
            files.push(ModelFile::new(
                ModelRelativePath::new(item.path.to_owned()).map_err(CliError::ModelPack)?,
                item.role,
                Sha256Digest::from_file(path).map_err(CliError::ModelPack)?,
            ));
        } else if item.required {
            return Err(CliError::MissingStandardModelFile(item.path));
        }
    }

    Ok(files)
}

/// { path is a model-pack root candidate }
/// fn plan_model(path: String) -> Result<String, CliError>
/// { ret summarizes verified SDK asset planning without loading inference sessions }
fn plan_model(path: String) -> Result<String, CliError> {
    let assets =
        OfflineTranslatorAssets::from_model_pack_path(path).map_err(CliError::OfflineAssets)?;

    Ok(assets.summary().to_preflight_text())
}

/// { path is a model-pack root candidate }
/// fn runtime_config_model(path: String) -> Result<String, CliError>
/// { ret summarizes strict ORT runtime config without loading ONNX sessions }
fn runtime_config_model(path: String) -> Result<String, CliError> {
    let pack = ModelPack::<Discovered>::discover(path)
        .and_then(ModelPack::verify)
        .map_err(CliError::ModelPack)?;
    let plan = OfflineTranslatorPlan::from_pack(&pack).map_err(CliError::OfflinePlan)?;
    let config = plan.generator().parse_runtime_config().map_err(|error| {
        CliError::OfflinePlan(localmt::OfflineTranslatorPlanError::Generator(error))
    })?;

    Ok(format!(
        "runtime_config: ok\nmax_new_tokens: {}\nencoder_input_ids: {}\ndecoder_input_ids: {}\ndecoder_logits: {}",
        config.generation_config().max_new_tokens().value(),
        config.ort_io_config().encoder().input_ids(),
        config.ort_io_config().decoder().input_ids(),
        config.ort_io_config().decoder().logits(),
    ))
}

/// { path is a model-pack root candidate }
/// fn doctor_model(path: String) -> Result<String, CliError>
/// { ret reports Ok only when required no-network preflight checks complete }
fn doctor_model(path: String) -> Result<String, CliError> {
    OfflineTranslatorAssets::from_model_pack_path(path.clone()).map_err(CliError::OfflineAssets)?;
    let path_bytes = path.as_bytes();
    let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let source_id = ffi_language_id(Language::English)?;
    let target_id = ffi_language_id(Language::Russian)?;
    let startup_status = doctor_ffi_startup()?;
    let mock_status = doctor_mock_translate(path_bytes, source_id, target_id)?;
    let hf_status = doctor_hf_mock_translate(path_bytes, source_id, target_id)?;
    let ort_status = doctor_ort_generator(path_bytes)?;
    let ort_translate_status = doctor_ort_translate(path_bytes, source_id, target_id)?;

    Ok(format!(
        "doctor: ok\nmodel_plan: ok\nffi_startup: {startup_status}\nffi_model_pack_summary: ok\nffi_mock_translate: {mock_status}\nffi_hf_mock_translate: {hf_status}\nffi_ort_generator: {ort_status}\nffi_ort_translate: {ort_translate_status}"
    ))
}

/// { true }
/// fn doctor_ffi_startup() -> Result<&'static str, CliError>
/// { ret is Ok only when startup summary is readable UTF-8 through FFI }
fn doctor_ffi_startup() -> Result<&'static str, CliError> {
    let output = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_startup_summary(output_ptr, output_capacity, written_len)
    })?;
    let _summary = String::from_utf8(output).map_err(CliError::FfiOutputUtf8)?;

    Ok("ok")
}

/// { path_bytes is a UTF-8 model-pack path and source_id/target_id form a valid FFI pair }
/// fn doctor_mock_translate(path_bytes: &[u8], source_id: u8, target_id: u8) -> Result<&'static str, CliError>
/// { ret is Ok only when the deterministic mock FFI translator opens and translates }
fn doctor_mock_translate(
    path_bytes: &[u8],
    source_id: u8,
    target_id: u8,
) -> Result<&'static str, CliError> {
    let mut translator: *mut localmt_ffi::LocalmtFfiTranslator = ptr::null_mut();
    ffi_ok(localmt_ffi::localmt_ffi_mock_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    ))?;

    let input = DOCTOR_SMOKE_TEXT.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_mock_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_mock_translator_close(translator);
    let _translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok("ok")
}

/// { path_bytes is a UTF-8 model-pack path and source_id/target_id form a valid FFI pair }
/// fn doctor_hf_mock_translate(path_bytes: &[u8], source_id: u8, target_id: u8) -> Result<String, CliError>
/// { ret is Ok only when HF FFI mock translation succeeds or the tokenizer feature is disabled }
fn doctor_hf_mock_translate(
    path_bytes: &[u8],
    source_id: u8,
    target_id: u8,
) -> Result<String, CliError> {
    let mut translator: *mut localmt_ffi::LocalmtFfiHfMockTranslator = ptr::null_mut();
    let status = localmt_ffi::localmt_ffi_hf_mock_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    );
    if status == localmt_ffi::LOCALMT_FFI_TOKENIZER_DISABLED {
        return Ok(ffi_status_text(status));
    }
    ffi_ok(status)?;

    let input = DOCTOR_SMOKE_TEXT.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_hf_mock_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_hf_mock_translator_close(translator);
    let _translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok("ok".to_owned())
}

/// { path_bytes is a UTF-8 model-pack path }
/// fn doctor_ort_generator(path_bytes: &[u8]) -> Result<String, CliError>
/// { ret is Ok only when ORT opens or reports a readiness-only runtime status }
fn doctor_ort_generator(path_bytes: &[u8]) -> Result<String, CliError> {
    let mut generator: *mut localmt_ffi::LocalmtFfiOrtGenerator = ptr::null_mut();
    let status = localmt_ffi::localmt_ffi_ort_generator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut generator,
    );
    if let Some(readiness) = doctor_ort_readiness_status(status) {
        return Ok(readiness);
    }
    ffi_ok(status)?;
    localmt_ffi::localmt_ffi_ort_generator_close(generator);

    Ok("ok".to_owned())
}

/// { path_bytes is a UTF-8 model-pack path and source_id/target_id form a valid FFI pair }
/// fn doctor_ort_translate(path_bytes: &[u8], source_id: u8, target_id: u8) -> Result<String, CliError>
/// { ret is Ok only when ORT FFI translation succeeds or required runtime features are disabled }
fn doctor_ort_translate(
    path_bytes: &[u8],
    source_id: u8,
    target_id: u8,
) -> Result<String, CliError> {
    let mut translator: *mut localmt_ffi::LocalmtFfiOrtTranslator = ptr::null_mut();
    let status = localmt_ffi::localmt_ffi_ort_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    );
    if status == localmt_ffi::LOCALMT_FFI_TOKENIZER_DISABLED {
        return Ok(ffi_status_text(status));
    }
    if let Some(readiness) = doctor_ort_readiness_status(status) {
        return Ok(readiness);
    }
    ffi_ok(status)?;

    let input = DOCTOR_SMOKE_TEXT.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_ort_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_ort_translator_close(translator);
    let _translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok("ok".to_owned())
}

/// { status is a localmt FFI status code }
/// fn doctor_ort_readiness_status(status: i32) -> Option<String>
/// { ret is Some only for ORT runtime readiness statuses that should not fail model doctor }
fn doctor_ort_readiness_status(status: i32) -> Option<String> {
    match status {
        localmt_ffi::LOCALMT_FFI_RUNTIME_DISABLED
        | localmt_ffi::LOCALMT_FFI_RUNTIME_NOT_CONFIGURED => Some(ffi_status_text(status)),
        _ => None,
    }
}

/// { args contains tokenize command arguments }
/// fn tokenize_model(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when a verified tokenizer encodes and decodes the input text }
fn tokenize_model(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;
    let source = parse_language(args.next(), "FROM")?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let text = NonEmptyText::new(text).map_err(CliError::InvalidText)?;
    let _pair = LanguagePair::new(source, target).map_err(CliError::InvalidPair)?;
    let assets =
        OfflineTranslatorAssets::from_model_pack_path(path).map_err(CliError::OfflineAssets)?;

    #[cfg(not(feature = "hf-tokenizers"))]
    {
        let _text = text;
        let _tokenizer_path = assets.plan().tokenizer().tokenizer_path();
        Err(CliError::TokenizerFeatureDisabled)
    }

    #[cfg(feature = "hf-tokenizers")]
    {
        let tokenizer = HfTokenizer::from_file(assets.plan().tokenizer().tokenizer_path())
            .map_err(CliError::Tokenizer)?;
        let input = TokenizerInput::new(source, target, text).map_err(CliError::Tokenizer)?;
        let encoded = tokenizer.encode(&input).map_err(CliError::Tokenizer)?;
        let token_ids = encoded
            .tokens()
            .as_slice()
            .iter()
            .map(|token| token.value().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let decoded = tokenizer
            .decode(target, encoded.tokens())
            .map_err(CliError::Tokenizer)?;

        Ok(format!(
            "tokenizer: loaded\ntokens: {token_ids}\ndecoded: {}",
            decoded.as_str()
        ))
    }
}

/// { args contains FFI smoke command arguments }
/// fn run_ffi_smoke(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when model-pack summary and mock translation succeed through FFI }
fn run_ffi_smoke(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;
    let source = parse_language(args.next(), "FROM")?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let path_bytes = path.as_bytes();
    let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let source_id = ffi_language_id(source)?;
    let target_id = ffi_language_id(target)?;

    let mut translator: *mut localmt_ffi::LocalmtFfiTranslator = ptr::null_mut();
    ffi_ok(localmt_ffi::localmt_ffi_mock_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    ))?;

    let input = text.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_mock_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_mock_translator_close(translator);
    let translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok(format!(
        "ffi_abi: {}\nmodel_pack_summary: ok\nmock_translator_open: ok\nmock_translate: ok\ntranslation: {translation}",
        localmt_ffi::localmt_ffi_abi_version()
    ))
}

/// { args contains FFI HF-tokenizer smoke command arguments }
/// fn run_ffi_hf_smoke(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when tokenizer-backed mock translation succeeds through FFI }
fn run_ffi_hf_smoke(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;
    let source = parse_language(args.next(), "FROM")?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let path_bytes = path.as_bytes();
    let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let source_id = ffi_language_id(source)?;
    let target_id = ffi_language_id(target)?;

    let mut translator: *mut localmt_ffi::LocalmtFfiHfMockTranslator = ptr::null_mut();
    ffi_ok(localmt_ffi::localmt_ffi_hf_mock_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    ))?;

    let input = text.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_hf_mock_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_hf_mock_translator_close(translator);
    let translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok(format!(
        "ffi_abi: {}\nmodel_pack_summary: ok\nhf_mock_translator_open: ok\nhf_mock_translate: ok\ntranslation: {translation}",
        localmt_ffi::localmt_ffi_abi_version()
    ))
}

/// { args contains FFI ORT preflight smoke command arguments }
/// fn run_ffi_ort_smoke(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when ORT generator preflight succeeds through FFI }
fn run_ffi_ort_smoke(args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = single_model_path(args)?;
    let path_bytes = path.as_bytes();
    let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;

    let mut generator: *mut localmt_ffi::LocalmtFfiOrtGenerator = ptr::null_mut();
    ffi_ok(localmt_ffi::localmt_ffi_ort_generator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut generator,
    ))?;
    localmt_ffi::localmt_ffi_ort_generator_close(generator);

    Ok(format!(
        "ffi_abi: {}\nmodel_pack_summary: ok\nort_generator_open: ok",
        localmt_ffi::localmt_ffi_abi_version()
    ))
}

/// { args contains FFI ORT translation smoke command arguments }
/// fn run_ffi_ort_translate_smoke(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when ORT-backed FFI translation succeeds }
fn run_ffi_ort_translate_smoke(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;
    let source = parse_language(args.next(), "FROM")?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let path_bytes = path.as_bytes();
    let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let source_id = ffi_language_id(source)?;
    let target_id = ffi_language_id(target)?;

    let mut translator: *mut localmt_ffi::LocalmtFfiOrtTranslator = ptr::null_mut();
    ffi_ok(localmt_ffi::localmt_ffi_ort_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    ))?;

    let input = text.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_ort_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_ort_translator_close(translator);
    let translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok(format!(
        "ffi_abi: {}\nmodel_pack_summary: ok\nort_translator_open: ok\nort_translate: ok\ntranslation: {translation}",
        localmt_ffi::localmt_ffi_abi_version()
    ))
}

/// { args contains one model path argument }
/// fn run_ffi_runtime_config(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when strict ORT runtime config is summarized through FFI }
fn run_ffi_runtime_config(args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = single_model_path(args)?;
    let path_bytes = path.as_bytes();
    let _model_summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let runtime_summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_runtime_config_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let runtime_summary = String::from_utf8(runtime_summary).map_err(CliError::FfiOutputUtf8)?;

    Ok(format!(
        "ffi_abi: {}\nmodel_pack_summary: ok\nruntime_config_summary: ok\n{runtime_summary}",
        localmt_ffi::localmt_ffi_abi_version()
    ))
}

/// { language is supported by localmt }
/// fn ffi_language_id(language: Language) -> Result<u8, CliError>
/// { ret is the stable FFI language id for language }
fn ffi_language_id(language: Language) -> Result<u8, CliError> {
    let bytes = language.iso_639_1().as_bytes();
    let id = localmt_ffi::localmt_ffi_language_from_iso_639_1(bytes[0], bytes[1]);
    if id < 0 {
        return Err(CliError::FfiStatus {
            status: localmt_ffi::LOCALMT_FFI_INVALID_LANGUAGE,
            message: ffi_status_text(localmt_ffi::LOCALMT_FFI_INVALID_LANGUAGE),
        });
    }

    u8::try_from(id).map_err(|_error| CliError::FfiStatus {
        status: localmt_ffi::LOCALMT_FFI_INVALID_LANGUAGE,
        message: ffi_status_text(localmt_ffi::LOCALMT_FFI_INVALID_LANGUAGE),
    })
}

/// { call writes through the standard FFI byte-buffer contract }
/// fn ffi_bytes(call: impl FnMut(*mut u8, usize, *mut usize) -> i32) -> Result<Vec<u8>, CliError>
/// { ret contains the bytes written by a successful FFI call }
fn ffi_bytes(mut call: impl FnMut(*mut u8, usize, *mut usize) -> i32) -> Result<Vec<u8>, CliError> {
    let mut written_len = 0_usize;
    let status = call(ptr::null_mut(), 0, &mut written_len);
    if status != localmt_ffi::LOCALMT_FFI_BUFFER_TOO_SMALL && status != localmt_ffi::LOCALMT_FFI_OK
    {
        return Err(CliError::FfiStatus {
            status,
            message: ffi_status_text(status),
        });
    }

    let mut output = vec![0_u8; written_len];
    let status = call(output.as_mut_ptr(), output.len(), &mut written_len);
    ffi_ok(status)?;
    output.truncate(written_len);

    Ok(output)
}

/// { status is an FFI status code }
/// fn ffi_ok(status: i32) -> Result<(), CliError>
/// { ret is Ok only when status is LOCALMT_FFI_OK }
fn ffi_ok(status: i32) -> Result<(), CliError> {
    if status == localmt_ffi::LOCALMT_FFI_OK {
        return Ok(());
    }

    Err(CliError::FfiStatus {
        status,
        message: ffi_status_text(status),
    })
}

/// { status may be any FFI status code }
/// fn ffi_status_text(status: i32) -> String
/// { ret is the stable FFI status message when the message lookup succeeds }
fn ffi_status_text(status: i32) -> String {
    let Ok(bytes) = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_status_message(status, output_ptr, output_capacity, written_len)
    }) else {
        return "status message unavailable".to_owned();
    };

    String::from_utf8(bytes).unwrap_or_else(|_error| "status message invalid utf-8".to_owned())
}

/// { args contains benchmark command arguments }
/// fn run_bench(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when profile and model-pack arguments are valid }
fn run_bench(args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let args = args.collect::<Vec<_>>();
    if matches!(args.as_slice(), [value] if is_help(value)) {
        return Ok(BENCH_HELP_TEXT.to_owned());
    }

    let options = BenchOptions::parse(args.into_iter())?;
    let pack = ModelPack::<Discovered>::discover(options.model_pack_path)
        .and_then(ModelPack::verify)
        .map_err(CliError::ModelPack)?;
    let report = localmt_bench::MockBenchmarkRunner::new(options.profile)
        .run(&pack)
        .map_err(CliError::Benchmark)?;

    Ok(format!(
        "profile: {}\nandroid_abi: {}\nram_class_gib: {}\npreferred_runtime: {}\nruntime: {}\nmodel_id: {}\nscenarios: {}\ntranslations: {}\ntotal_ms: {}\nwarm_translate_ms: {}",
        report.profile().as_str(),
        report.android_abi(),
        report.ram_class_gib(),
        report.preferred_runtime(),
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

/// { value may be any CLI argument }
/// fn is_help(value: &str) -> bool
/// { ret is true only for supported help aliases }
fn is_help(value: &str) -> bool {
    matches!(value, "help" | "--help" | "-h")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StandardModelFile {
    path: &'static str,
    role: ModelFileRole,
    required: bool,
}

impl StandardModelFile {
    /// { path is a standard model-pack relative path }
    /// fn required(path: &'static str, role: ModelFileRole) -> Self
    /// { ret marks path as required for write-manifest }
    const fn required(path: &'static str, role: ModelFileRole) -> Self {
        Self {
            path,
            role,
            required: true,
        }
    }

    /// { path is a standard model-pack relative path }
    /// fn optional(path: &'static str, role: ModelFileRole) -> Self
    /// { ret marks path as optional for write-manifest }
    const fn optional(path: &'static str, role: ModelFileRole) -> Self {
        Self {
            path,
            role,
            required: false,
        }
    }
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
    UnknownFfiCommand(String),
    ModelPack(localmt_models::ModelPackError),
    OfflinePlan(localmt::OfflineTranslatorPlanError),
    OfflineAssets(localmt::OfflineTranslatorAssetsError),
    Benchmark(localmt_bench::BenchmarkError),
    InvalidBenchArguments(String),
    MissingStandardModelFile(&'static str),
    FfiStatus {
        status: i32,
        message: String,
    },
    FfiOutputUtf8(std::string::FromUtf8Error),
    #[cfg(not(feature = "hf-tokenizers"))]
    TokenizerFeatureDisabled,
    #[cfg(feature = "hf-tokenizers")]
    Tokenizer(localmt::TokenizerError),
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
            Self::UnknownFfiCommand(command) => write!(formatter, "unknown ffi command: {command}"),
            Self::ModelPack(error) => write!(formatter, "{error}"),
            Self::OfflinePlan(error) => write!(formatter, "{error}"),
            Self::OfflineAssets(error) => write!(formatter, "{error}"),
            Self::Benchmark(error) => write!(formatter, "{error}"),
            Self::InvalidBenchArguments(message) => write!(formatter, "{message}"),
            Self::MissingStandardModelFile(path) => {
                write!(formatter, "missing standard model file: {path}")
            }
            Self::FfiStatus { status, message } => {
                write!(formatter, "ffi status {status}: {message}")
            }
            Self::FfiOutputUtf8(error) => write!(formatter, "ffi output is not UTF-8: {error}"),
            #[cfg(not(feature = "hf-tokenizers"))]
            Self::TokenizerFeatureDisabled => {
                formatter.write_str("hf-tokenizers feature is not enabled")
            }
            #[cfg(feature = "hf-tokenizers")]
            Self::Tokenizer(error) => write!(formatter, "{error}"),
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

    use super::{doctor_ort_readiness_status, run};
    #[cfg(feature = "hf-tokenizers")]
    use localmt::Sha256Digest;

    const ENCODER_SHA256: &str = "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b";
    const DECODER_SHA256: &str = "eacbeef293be61f2a85d929cadb4cbb5248c8b8a1478b3d4b3180ea365d5e687";
    const GENERATION_CONFIG_SHA256: &str =
        "a8a99326d564beb1fc16cb59526dd5ed7b5fd673f969591f47845e17fbed401d";
    const ORT_IO_CONFIG_SHA256: &str =
        "3ed8af0b5a58d51e246f3081e973d8683b89bf3cacf23c26243fec19ab31a721";
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
    const ORT_IO_CONFIG: &str = r#"{
  "ort_io": {
    "encoder": {
      "input_ids": "encoder_input_ids",
      "attention_mask": "encoder_attention_mask",
      "last_hidden_state": "encoder_last_hidden_state"
    },
    "decoder": {
      "input_ids": "decoder_input_ids",
      "encoder_attention_mask": "decoder_encoder_attention_mask",
      "encoder_hidden_states": "decoder_encoder_hidden_states",
      "logits": "decoder_logits"
    },
    "decoder_with_past": {
      "input_ids": "past_input_ids",
      "encoder_attention_mask": "past_encoder_attention_mask",
      "encoder_hidden_states": "past_encoder_hidden_states",
      "logits": "past_logits"
    }
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
        assert!(output.contains("android_abi: arm64-v8a"));
        assert!(output.contains("ram_class_gib: 12"));
        assert!(output.contains("preferred_runtime: onnx-runtime-mobile-xnnpack"));
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
        assert!(output.contains("ort_io_config: absent"));
        Ok(())
    }

    #[test]
    fn cli_runs_ffi_smoke_for_verified_pack() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "ffi".to_owned(),
            "smoke".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("ffi_abi: 11"));
        assert!(output.contains("model_pack_summary: ok"));
        assert!(output.contains("mock_translator_open: ok"));
        assert!(output.contains("mock_translate: ok"));
        assert!(output.contains("translation: hello offline"));
        Ok(())
    }

    #[test]
    #[cfg(all(not(feature = "hf-tokenizers"), not(feature = "ort-runtime")))]
    fn cli_model_doctor_reports_default_readiness_for_verified_pack()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "doctor".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("doctor: ok"));
        assert!(output.contains("model_plan: ok"));
        assert!(output.contains("ffi_startup: ok"));
        assert!(output.contains("ffi_model_pack_summary: ok"));
        assert!(output.contains("ffi_mock_translate: ok"));
        assert!(output.contains("ffi_hf_mock_translate: tokenizer disabled"));
        assert!(output.contains("ffi_ort_generator: runtime disabled"));
        assert!(output.contains("ffi_ort_translate: tokenizer disabled"));
        Ok(())
    }

    #[test]
    fn doctor_ort_readiness_status_reports_runtime_not_configured() {
        assert_eq!(
            doctor_ort_readiness_status(localmt_ffi::LOCALMT_FFI_RUNTIME_NOT_CONFIGURED).as_deref(),
            Some("runtime not configured")
        );
    }

    #[test]
    fn cli_ffi_header_prints_bundled_header() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["localmt".to_owned(), "ffi".to_owned(), "header".to_owned()];

        let output = run(args.into_iter())?;

        assert!(output.contains("#ifndef LOCALMT_FFI_H"));
        assert!(output.contains("#define LOCALMT_FFI_ABI_VERSION 11"));
        assert!(output.contains("int32_t localmt_ffi_ort_runtime_configure("));
        assert!(output.contains("int32_t localmt_ffi_model_pack_summary("));
        assert!(output.contains("int32_t localmt_ffi_runtime_config_summary("));
        assert!(output.contains("int32_t localmt_ffi_mock_translate("));
        assert!(output.contains("int32_t localmt_ffi_ort_generator_open("));
        assert!(output.contains("int32_t localmt_ffi_ort_translator_open("));
        assert!(output.contains("int32_t localmt_ffi_ort_translate("));
        Ok(())
    }

    #[test]
    fn cli_ffi_startup_prints_android_contract() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["localmt".to_owned(), "ffi".to_owned(), "startup".to_owned()];

        let output = run(args.into_iter())?;

        assert!(output.contains("ffi_abi: 11"));
        assert!(output.contains("max_text_chars: 4096"));
        assert!(output.contains("xiaomi17_android_abi: arm64-v8a"));
        assert!(output.contains("languages: en, ru, th, vi, ja"));
        Ok(())
    }

    #[test]
    fn cli_ffi_runtime_config_reports_strict_ort_contract() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_runtime_config_pack()?;
        let args = [
            "localmt".to_owned(),
            "ffi".to_owned(),
            "runtime-config".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("ffi_abi: 11"));
        assert!(output.contains("runtime_config_summary: ok"));
        assert!(output.contains("runtime_config: ok"));
        assert!(output.contains("decoder_logits: decoder_logits"));
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn cli_ffi_hf_smoke_reports_tokenizer_feature_disabled_after_pack_planning()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "ffi".to_owned(),
            "hf-smoke".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let result = run(args.into_iter());

        assert!(
            matches!(result, Err(ref error) if error.to_string().contains("tokenizer disabled"))
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn cli_ffi_hf_smoke_loads_tokenizer_and_translates_through_ffi()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_hf_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "ffi".to_owned(),
            "hf-smoke".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("ffi_abi: 11"));
        assert!(output.contains("model_pack_summary: ok"));
        assert!(output.contains("hf_mock_translator_open: ok"));
        assert!(output.contains("hf_mock_translate: ok"));
        assert!(output.contains("translation: hello offline"));
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn cli_ffi_ort_smoke_reports_runtime_disabled_after_pack_planning()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "ffi".to_owned(),
            "ort-smoke".to_owned(),
            root.display().to_string(),
        ];

        let result = run(args.into_iter());

        assert!(matches!(result, Err(ref error) if error.to_string().contains("runtime disabled")));
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn cli_ffi_ort_translate_smoke_reports_tokenizer_disabled_after_pack_planning()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_runtime_config_pack()?;
        let args = [
            "localmt".to_owned(),
            "ffi".to_owned(),
            "ort-translate-smoke".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let result = run(args.into_iter());

        assert!(
            matches!(result, Err(ref error) if error.to_string().contains("tokenizer disabled"))
        );
        Ok(())
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn cli_model_tokenizer_smoke_reports_feature_disabled_after_pack_planning()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "tokenize".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let result = run(args.into_iter());

        assert!(
            matches!(result, Err(ref error) if error.to_string().contains("hf-tokenizers feature is not enabled"))
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn cli_model_tokenizer_smoke_loads_tokenizer_and_round_trips_text()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_hf_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "tokenize".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let output = run(args.into_iter())?;

        assert_eq!(
            output,
            "tokenizer: loaded\ntokens: 1, 2\ndecoded: hello offline"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn cli_model_tokenizer_smoke_reports_tokenizer_load_errors()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_plannable_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "tokenize".to_owned(),
            root.display().to_string(),
            "en".to_owned(),
            "ru".to_owned(),
            "hello offline".to_owned(),
        ];

        let result = run(args.into_iter());

        assert!(
            matches!(result, Err(ref error) if error.to_string().contains("failed to load tokenizer"))
        );
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

    #[test]
    fn cli_write_manifest_outputs_standard_model_files() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_standard_model_files()?;
        let args = write_manifest_args(&root);

        let output = run(args.into_iter())?;

        assert!(output.contains(r#""model_id": "m2m100-418m-int8""#));
        assert!(output.contains(r#""kind": "encoder""#));
        assert!(output.contains(r#""kind": "decoder""#));
        assert!(output.contains(r#""kind": "tokenizer""#));

        fs::write(root.join("manifest.json"), output)?;
        let pack =
            localmt_models::ModelPack::<localmt_models::Discovered>::discover(&root)?.verify()?;

        assert_eq!(pack.manifest().model_id().as_str(), "m2m100-418m-int8");
        Ok(())
    }

    #[test]
    fn cli_write_manifest_rejects_missing_required_standard_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        let args = write_manifest_args(&root);

        let result = run(args.into_iter());

        assert!(matches!(
            result,
            Err(ref error) if error.to_string().contains("missing standard model file: decoder.onnx")
        ));
        Ok(())
    }

    #[test]
    fn cli_prints_help() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["localmt".to_owned(), "--help".to_owned()];

        let output = run(args.into_iter())?;

        assert!(output.contains("localmt FROM TO TEXT"));
        assert!(output.contains("localmt model hash FILE"));
        assert!(output.contains("localmt model doctor PACK"));
        assert!(output.contains("localmt model runtime-config PACK"));
        assert!(output.contains(
            "localmt model write-manifest PACK MODEL_ID VERSION ARCHITECTURE RUNTIME LICENSE"
        ));
        assert!(output.contains("localmt ffi smoke PACK FROM TO TEXT"));
        assert!(output.contains("localmt ffi startup"));
        assert!(output.contains("localmt ffi header"));
        assert!(output.contains("localmt ffi runtime-config PACK"));
        assert!(output.contains("localmt ffi hf-smoke PACK FROM TO TEXT"));
        assert!(output.contains("localmt ffi ort-smoke PACK"));
        assert!(output.contains("localmt ffi ort-translate-smoke PACK FROM TO TEXT"));
        assert!(output.contains("localmt bench --profile xiaomi17 --model-pack PACK"));
        Ok(())
    }

    #[test]
    fn cli_prints_ffi_help() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["localmt".to_owned(), "ffi".to_owned(), "help".to_owned()];

        let output = run(args.into_iter())?;

        assert!(output.contains("localmt ffi smoke PACK FROM TO TEXT"));
        assert!(output.contains("localmt ffi startup"));
        assert!(output.contains("localmt ffi header"));
        assert!(output.contains("localmt ffi runtime-config PACK"));
        assert!(output.contains("localmt ffi hf-smoke PACK FROM TO TEXT"));
        assert!(output.contains("localmt ffi ort-smoke PACK"));
        assert!(output.contains("localmt ffi ort-translate-smoke PACK FROM TO TEXT"));
        Ok(())
    }

    #[test]
    fn cli_prints_model_help() -> Result<(), Box<dyn std::error::Error>> {
        let args = ["localmt".to_owned(), "model".to_owned(), "help".to_owned()];

        let output = run(args.into_iter())?;

        assert!(output.contains("localmt model inspect PACK"));
        assert!(output.contains("localmt model verify PACK"));
        assert!(output.contains("localmt model plan PACK"));
        assert!(output.contains("localmt model doctor PACK"));
        assert!(output.contains("localmt model runtime-config PACK"));
        assert!(output.contains("localmt model tokenize PACK FROM TO TEXT"));
        assert!(output.contains("localmt model hash FILE"));
        assert!(output.contains(
            "localmt model write-manifest PACK MODEL_ID VERSION ARCHITECTURE RUNTIME LICENSE"
        ));
        Ok(())
    }

    #[test]
    fn cli_reports_ort_runtime_config_for_verified_pack() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_runtime_config_pack()?;
        let args = [
            "localmt".to_owned(),
            "model".to_owned(),
            "runtime-config".to_owned(),
            root.display().to_string(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("runtime_config: ok"));
        assert!(output.contains("max_new_tokens: 32"));
        assert!(output.contains("encoder_input_ids: encoder_input_ids"));
        assert!(output.contains("decoder_logits: decoder_logits"));
        Ok(())
    }

    #[test]
    fn cli_prints_bench_help() -> Result<(), Box<dyn std::error::Error>> {
        let args = [
            "localmt".to_owned(),
            "bench".to_owned(),
            "--help".to_owned(),
        ];

        let output = run(args.into_iter())?;

        assert!(output.contains("localmt bench --profile xiaomi17 --model-pack PACK"));
        assert!(output.contains("runtime: mock-pipeline"));
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

    fn create_runtime_config_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(root.join("generation.json"), GENERATION_CONFIG)?;
        fs::write(root.join("config.json"), ORT_IO_CONFIG)?;
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
    {{ "path": "generation.json", "kind": "generation_config", "sha256": "{GENERATION_CONFIG_SHA256}" }},
    {{ "path": "config.json", "kind": "config", "sha256": "{ORT_IO_CONFIG_SHA256}" }}
  ]
}}"#
            ),
        )?;
        Ok(root)
    }

    #[cfg(feature = "hf-tokenizers")]
    fn create_hf_plannable_pack() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), wordlevel_tokenizer_json())?;
        fs::write(root.join("generation.json"), GENERATION_CONFIG)?;
        let tokenizer_sha256 = Sha256Digest::from_file(root.join("tokenizer.json"))?;
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
    {{ "path": "tokenizer.json", "kind": "tokenizer", "sha256": "{}" }},
    {{ "path": "generation.json", "kind": "generation_config", "sha256": "{GENERATION_CONFIG_SHA256}" }}
  ]
}}"#,
                tokenizer_sha256.as_str()
            ),
        )?;
        Ok(root)
    }

    fn create_standard_model_files() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let root = create_temp_dir()?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(root.join("generation.json"), GENERATION_CONFIG)?;
        Ok(root)
    }

    fn write_manifest_args(root: &std::path::Path) -> [String; 9] {
        [
            "localmt".to_owned(),
            "model".to_owned(),
            "write-manifest".to_owned(),
            root.display().to_string(),
            "m2m100-418m-int8".to_owned(),
            "0.1.0".to_owned(),
            "m2m100".to_owned(),
            "onnx-runtime".to_owned(),
            "MIT".to_owned(),
        ]
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

    #[cfg(feature = "hf-tokenizers")]
    fn wordlevel_tokenizer_json() -> &'static str {
        r#"{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{"type":"WhitespaceSplit"},"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"[UNK]":0,"hello":1,"offline":2},"unk_token":"[UNK]"}}"#
    }
}
