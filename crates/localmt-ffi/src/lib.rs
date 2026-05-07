//! Pointer-free C ABI for localmt Android adapters.

use std::{ptr, slice, str};

#[cfg(feature = "ort-runtime")]
use localmt::configure_ort_dylib_path;
use localmt::{
    DeviceProfile, Language, LanguagePair, MAX_TEXT_CHARS, MockOfflineTranslator, NonEmptyText,
    OfflineTranslatorAssets, OrtEngineError, OrtTokenGenerator, TranslateRequest,
};
#[cfg(feature = "hf-tokenizers")]
use localmt::{HfMockOfflineTranslator, HfMockOfflineTranslatorError, HfTokenizer};
#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
use localmt::{OrtOfflineTranslator, OrtOfflineTranslatorError};

/// FFI status for successful calls.
pub const LOCALMT_FFI_OK: i32 = 0;
/// FFI status for invalid language ids.
pub const LOCALMT_FFI_INVALID_LANGUAGE: i32 = 1;
/// FFI status for invalid language pairs.
pub const LOCALMT_FFI_INVALID_PAIR: i32 = 2;
/// FFI status for null pointer arguments.
pub const LOCALMT_FFI_NULL_POINTER: i32 = 3;
/// FFI status for byte slices that are not valid UTF-8.
pub const LOCALMT_FFI_INVALID_UTF8: i32 = 4;
/// FFI status for model-pack discovery, verification, or planning failures.
pub const LOCALMT_FFI_MODEL_PACK_ERROR: i32 = 5;
/// FFI status for text input that violates facade invariants.
pub const LOCALMT_FFI_TEXT_ERROR: i32 = 6;
/// FFI status for translation backend failures.
pub const LOCALMT_FFI_TRANSLATION_ERROR: i32 = 7;
/// FFI status when the caller output buffer is too small.
pub const LOCALMT_FFI_BUFFER_TOO_SMALL: i32 = 8;
/// FFI status when the build does not enable the ORT runtime.
pub const LOCALMT_FFI_RUNTIME_DISABLED: i32 = 9;
/// FFI status when ONNX Runtime session loading fails.
pub const LOCALMT_FFI_ORT_ERROR: i32 = 10;
/// FFI status when the build does not enable the HF tokenizer backend.
pub const LOCALMT_FFI_TOKENIZER_DISABLED: i32 = 11;
/// FFI status when tokenizer loading fails.
pub const LOCALMT_FFI_TOKENIZER_ERROR: i32 = 12;
/// FFI status when ORT runtime is enabled but no usable dylib path is configured.
pub const LOCALMT_FFI_RUNTIME_NOT_CONFIGURED: i32 = 13;

/// Pointer-free C ABI version.
pub const LOCALMT_FFI_ABI_VERSION: u32 = 11;
/// FFI code for Android arm64-v8a.
pub const LOCALMT_FFI_ANDROID_ABI_ARM64_V8A: u16 = 1;
/// FFI code for ONNX Runtime Mobile with XNNPACK.
pub const LOCALMT_FFI_RUNTIME_ONNX_MOBILE_XNNPACK: u16 = 1;
const LANGUAGES: [Language; 5] = [
    Language::English,
    Language::Russian,
    Language::Thai,
    Language::Vietnamese,
    Language::Japanese,
];

/// Two-byte ISO 639-1 language code for FFI callers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalmtFfiLanguageCode {
    /// First ASCII byte.
    pub first: u8,
    /// Second ASCII byte.
    pub second: u8,
}

/// Opaque Rust-owned mock translator handle for FFI callers.
pub struct LocalmtFfiTranslator {
    translator: MockOfflineTranslator,
}

/// Opaque Rust-owned HF-tokenizer mock translator handle for FFI callers.
pub struct LocalmtFfiHfMockTranslator {
    #[cfg(feature = "hf-tokenizers")]
    translator: HfMockOfflineTranslator,
}

/// Opaque Rust-owned ORT generator preflight handle for FFI callers.
pub struct LocalmtFfiOrtGenerator {
    _generator: OrtTokenGenerator,
}

/// Opaque Rust-owned ORT translator handle for FFI callers.
pub struct LocalmtFfiOrtTranslator {
    #[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
    translator: OrtOfflineTranslator,
}

/// Opaque Rust-owned HF tokenizer preflight handle for FFI callers.
pub struct LocalmtFfiHfTokenizer {
    #[cfg(feature = "hf-tokenizers")]
    _tokenizer: HfTokenizer,
}

impl LocalmtFfiLanguageCode {
    /// { first and second are raw language-code bytes }
    /// fn new(first: u8, second: u8) -> Self
    /// { ret stores exactly those two bytes }
    pub const fn new(first: u8, second: u8) -> Self {
        Self { first, second }
    }

    /// { true }
    /// fn as_bytes(self) -> [u8; 2]
    /// { ret is the two stored bytes }
    pub const fn as_bytes(self) -> [u8; 2] {
        [self.first, self.second]
    }
}

/// { true }
/// fn localmt_ffi_abi_version() -> u32
/// { ret is the pointer-free C ABI version }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_abi_version() -> u32 {
    LOCALMT_FFI_ABI_VERSION
}

/// { true }
/// fn localmt_ffi_supported_language_count() -> usize
/// { ret is the number of stable language ids exposed by this ABI }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_supported_language_count() -> usize {
    LANGUAGES.len()
}

/// { language_id may be any u8 }
/// fn localmt_ffi_language_code(language_id: u8) -> LocalmtFfiLanguageCode
/// { ret is zeroed only when language_id is not supported }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_language_code(language_id: u8) -> LocalmtFfiLanguageCode {
    language_from_id(language_id)
        .map(language_code)
        .unwrap_or(LocalmtFfiLanguageCode::new(0, 0))
}

/// { first and second may be any bytes }
/// fn localmt_ffi_language_from_iso_639_1(first: u8, second: u8) -> i32
/// { ret is -1 only when bytes do not name a supported language }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_language_from_iso_639_1(first: u8, second: u8) -> i32 {
    LANGUAGES
        .iter()
        .position(|language| language_code(*language).as_bytes() == [first, second])
        .map(|index| index as i32)
        .unwrap_or(-1)
}

/// { source_id and target_id may be any u8 }
/// fn localmt_ffi_validate_language_pair(source_id: u8, target_id: u8) -> i32
/// { ret is OK only when both language ids are valid and distinct }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_validate_language_pair(source_id: u8, target_id: u8) -> i32 {
    let Some(source) = language_from_id(source_id) else {
        return LOCALMT_FFI_INVALID_LANGUAGE;
    };
    let Some(target) = language_from_id(target_id) else {
        return LOCALMT_FFI_INVALID_LANGUAGE;
    };

    match LanguagePair::new(source, target) {
        Ok(_pair) => LOCALMT_FFI_OK,
        Err(_error) => LOCALMT_FFI_INVALID_PAIR,
    }
}

/// { true }
/// fn localmt_ffi_max_text_chars() -> usize
/// { ret is the Rust facade max input text length }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_max_text_chars() -> usize {
    MAX_TEXT_CHARS
}

/// { status may be any i32 }
/// fn ffi_status_message(status: i32) -> &'static str
/// { ret is a stable short message for known status codes, otherwise unknown status }
const fn ffi_status_message(status: i32) -> &'static str {
    match status {
        LOCALMT_FFI_OK => "ok",
        LOCALMT_FFI_INVALID_LANGUAGE => "invalid language",
        LOCALMT_FFI_INVALID_PAIR => "invalid language pair",
        LOCALMT_FFI_NULL_POINTER => "null pointer",
        LOCALMT_FFI_INVALID_UTF8 => "invalid utf-8",
        LOCALMT_FFI_MODEL_PACK_ERROR => "model pack error",
        LOCALMT_FFI_TEXT_ERROR => "text error",
        LOCALMT_FFI_TRANSLATION_ERROR => "translation error",
        LOCALMT_FFI_BUFFER_TOO_SMALL => "buffer too small",
        LOCALMT_FFI_RUNTIME_DISABLED => "runtime disabled",
        LOCALMT_FFI_ORT_ERROR => "onnx runtime error",
        LOCALMT_FFI_TOKENIZER_DISABLED => "tokenizer disabled",
        LOCALMT_FFI_TOKENIZER_ERROR => "tokenizer error",
        LOCALMT_FFI_RUNTIME_NOT_CONFIGURED => "runtime not configured",
        _ => "unknown status",
    }
}

/// { output/written pointers follow the header contract }
/// fn localmt_ffi_status_message(status: i32, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 message bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_status_message(
    status: i32,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    let output = ffi_status_message(status).as_bytes();
    unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

    if output_capacity < output.len() {
        return LOCALMT_FFI_BUFFER_TOO_SMALL;
    }
    if output_ptr.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

    LOCALMT_FFI_OK
}

/// { output/written pointers follow the header contract }
/// fn localmt_ffi_startup_summary(output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 startup summary bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_startup_summary(
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    let summary = ffi_startup_summary();
    let output = summary.as_bytes();
    unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

    if output_capacity < output.len() {
        return LOCALMT_FFI_BUFFER_TOO_SMALL;
    }
    if output_ptr.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

    LOCALMT_FFI_OK
}

/// { true }
/// fn ffi_startup_summary() -> String
/// { ret is the stable Android startup contract summary for this build }
fn ffi_startup_summary() -> String {
    let languages = LANGUAGES
        .iter()
        .map(|language| language.iso_639_1())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "ffi_abi: {}\nmax_text_chars: {}\nxiaomi17_android_abi: {}\nxiaomi17_ram_class_gib: {}\nxiaomi17_preferred_runtime: {}\nhf_tokenizers: {}\nort_runtime: {}\nlanguages: {languages}",
        LOCALMT_FFI_ABI_VERSION,
        MAX_TEXT_CHARS,
        DeviceProfile::Xiaomi17.android_abi(),
        DeviceProfile::Xiaomi17.ram_class_gib(),
        DeviceProfile::Xiaomi17.preferred_runtime(),
        feature_status(cfg!(feature = "hf-tokenizers")),
        feature_status(cfg!(feature = "ort-runtime")),
    )
}

/// { true }
/// fn feature_status(enabled: bool) -> &'static str
/// { ret is enabled only when enabled is true }
const fn feature_status(enabled: bool) -> &'static str {
    if enabled { "enabled" } else { "disabled" }
}

/// { true }
/// fn localmt_ffi_xiaomi17_android_abi_code() -> u16
/// { ret is the stable FFI code for Xiaomi 17 Android ABI }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_xiaomi17_android_abi_code() -> u16 {
    match DeviceProfile::Xiaomi17.android_abi() {
        "arm64-v8a" => LOCALMT_FFI_ANDROID_ABI_ARM64_V8A,
        _ => 0,
    }
}

/// { true }
/// fn localmt_ffi_xiaomi17_ram_class_gib() -> u16
/// { ret is the Xiaomi 17 RAM class in GiB }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_xiaomi17_ram_class_gib() -> u16 {
    DeviceProfile::Xiaomi17.ram_class_gib()
}

/// { true }
/// fn localmt_ffi_xiaomi17_preferred_runtime_code() -> u16
/// { ret is the stable FFI code for Xiaomi 17 preferred runtime }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_xiaomi17_preferred_runtime_code() -> u16 {
    match DeviceProfile::Xiaomi17.preferred_runtime() {
        "onnx-runtime-mobile-xnnpack" => LOCALMT_FFI_RUNTIME_ONNX_MOBILE_XNNPACK,
        _ => 0,
    }
}

/// { path_ptr points to path_len readable bytes and output/written pointers follow the header contract }
/// fn localmt_ffi_model_pack_summary(path_ptr: *const u8, path_len: usize, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 summary bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_model_pack_summary(
    path_ptr: *const u8,
    path_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *written_len = 0 }; // SAFETY: non-null writable length pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let assets = match OfflineTranslatorAssets::from_model_pack_path(path) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_MODEL_PACK_ERROR,
    };
    let summary = assets.summary().to_preflight_text();
    let output = summary.as_bytes();

    unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

    if output_capacity < output.len() {
        return LOCALMT_FFI_BUFFER_TOO_SMALL;
    }
    if output_ptr.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

    LOCALMT_FFI_OK
}

/// { path_ptr points to path_len readable bytes and output/written pointers follow the header contract }
/// fn localmt_ffi_runtime_config_summary(path_ptr: *const u8, path_len: usize, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 strict runtime config summary bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_runtime_config_summary(
    path_ptr: *const u8,
    path_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *written_len = 0 }; // SAFETY: non-null writable length pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let summary = match ffi_runtime_config_summary(path) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let output = summary.as_bytes();

    unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

    if output_capacity < output.len() {
        return LOCALMT_FFI_BUFFER_TOO_SMALL;
    }
    if output_ptr.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

    LOCALMT_FFI_OK
}

/// { path names a model-pack root candidate }
/// fn ffi_runtime_config_summary(path: &str) -> Result<String, i32>
/// { ret is Ok only when the strict ORT runtime config can be summarized }
fn ffi_runtime_config_summary(path: &str) -> Result<String, i32> {
    let assets = OfflineTranslatorAssets::from_model_pack_path(path)
        .map_err(|_error| LOCALMT_FFI_MODEL_PACK_ERROR)?;
    let config = assets
        .plan()
        .generator()
        .parse_runtime_config()
        .map_err(|_error| LOCALMT_FFI_MODEL_PACK_ERROR)?;

    Ok(format!(
        "runtime_config: ok\nmax_new_tokens: {}\nencoder_input_ids: {}\ndecoder_input_ids: {}\ndecoder_logits: {}",
        config.generation_config().max_new_tokens().value(),
        config.ort_io_config().encoder().input_ids(),
        config.ort_io_config().decoder().input_ids(),
        config.ort_io_config().decoder().logits(),
    ))
}

/// { path_ptr points to path_len readable bytes and out_translator is writable }
/// fn localmt_ffi_mock_translator_open(path_ptr: *const u8, path_len: usize, out_translator: *mut *mut LocalmtFfiTranslator) -> i32
/// { ret is OK only when out_translator receives an owned non-null handle }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_mock_translator_open(
    path_ptr: *const u8,
    path_len: usize,
    out_translator: *mut *mut LocalmtFfiTranslator,
) -> i32 {
    if out_translator.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *out_translator = ptr::null_mut() }; // SAFETY: non-null writable out pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };

    match MockOfflineTranslator::from_model_pack_path(path) {
        Ok(translator) => {
            let handle = Box::new(LocalmtFfiTranslator { translator });
            unsafe { *out_translator = Box::into_raw(handle) }; // SAFETY: non-null writable out pointer.
            LOCALMT_FFI_OK
        }
        Err(_error) => LOCALMT_FFI_MODEL_PACK_ERROR,
    }
}

/// { translator is null or was returned by localmt_ffi_mock_translator_open }
/// fn localmt_ffi_mock_translator_close(translator: *mut LocalmtFfiTranslator)
/// { translator is consumed when non-null }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_mock_translator_close(translator: *mut LocalmtFfiTranslator) {
    if translator.is_null() {
        return;
    }

    unsafe { drop(Box::from_raw(translator)) }; // SAFETY: handle came from open and closes once.
}

/// { translator is a valid handle, input/output/written pointers follow the header contract }
/// fn localmt_ffi_mock_translate(translator: *const LocalmtFfiTranslator, source_id: u8, target_id: u8, input_ptr: *const u8, input_len: usize, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_mock_translate(
    translator: *const LocalmtFfiTranslator,
    source_id: u8,
    target_id: u8,
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if translator.is_null() || written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *written_len = 0 }; // SAFETY: non-null writable length pointer.

    let Some(source) = language_from_id(source_id) else {
        return LOCALMT_FFI_INVALID_LANGUAGE;
    };
    let Some(target) = language_from_id(target_id) else {
        return LOCALMT_FFI_INVALID_LANGUAGE;
    };

    let input = match read_ffi_utf8(input_ptr, input_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let text = match NonEmptyText::new(input.to_owned()) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_TEXT_ERROR,
    };
    let request = match TranslateRequest::new(source, target, text) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_INVALID_PAIR,
    };
    let translator = unsafe { &*translator }; // SAFETY: non-null live handle pointer.
    let translation = match translator.translator.translate(&request) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_TRANSLATION_ERROR,
    };
    let output = translation.text().as_str().as_bytes();

    unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

    if output_capacity < output.len() {
        return LOCALMT_FFI_BUFFER_TOO_SMALL;
    }
    if output_ptr.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

    LOCALMT_FFI_OK
}

/// { path_ptr points to path_len readable bytes and out_translator is writable }
/// fn localmt_ffi_hf_mock_translator_open(path_ptr: *const u8, path_len: usize, out_translator: *mut *mut LocalmtFfiHfMockTranslator) -> i32
/// { ret is OK only when out_translator receives an owned non-null HF-tokenizer mock handle }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_hf_mock_translator_open(
    path_ptr: *const u8,
    path_len: usize,
    out_translator: *mut *mut LocalmtFfiHfMockTranslator,
) -> i32 {
    if out_translator.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *out_translator = ptr::null_mut() }; // SAFETY: non-null writable out pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let assets = match OfflineTranslatorAssets::from_model_pack_path(path) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_MODEL_PACK_ERROR,
    };

    #[cfg(not(feature = "hf-tokenizers"))]
    {
        let _tokenizer_path = assets.plan().tokenizer().tokenizer_path();
        LOCALMT_FFI_TOKENIZER_DISABLED
    }

    #[cfg(feature = "hf-tokenizers")]
    {
        match HfMockOfflineTranslator::from_assets(assets) {
            Ok(translator) => {
                let handle = Box::new(LocalmtFfiHfMockTranslator { translator });
                unsafe { *out_translator = Box::into_raw(handle) }; // SAFETY: non-null writable out pointer.
                LOCALMT_FFI_OK
            }
            Err(HfMockOfflineTranslatorError::Assets(_error)) => LOCALMT_FFI_MODEL_PACK_ERROR,
            Err(HfMockOfflineTranslatorError::Tokenizer(_error)) => LOCALMT_FFI_TOKENIZER_ERROR,
        }
    }
}

/// { translator is null or was returned by localmt_ffi_hf_mock_translator_open }
/// fn localmt_ffi_hf_mock_translator_close(translator: *mut LocalmtFfiHfMockTranslator)
/// { translator is consumed when non-null }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_hf_mock_translator_close(
    translator: *mut LocalmtFfiHfMockTranslator,
) {
    if translator.is_null() {
        return;
    }

    unsafe { drop(Box::from_raw(translator)) }; // SAFETY: handle came from open and closes once.
}

/// { translator is a valid HF-tokenizer mock handle, input/output/written pointers follow the header contract }
/// fn localmt_ffi_hf_mock_translate(translator: *const LocalmtFfiHfMockTranslator, source_id: u8, target_id: u8, input_ptr: *const u8, input_len: usize, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_hf_mock_translate(
    translator: *const LocalmtFfiHfMockTranslator,
    source_id: u8,
    target_id: u8,
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if translator.is_null() || written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *written_len = 0 }; // SAFETY: non-null writable length pointer.

    #[cfg(not(feature = "hf-tokenizers"))]
    {
        let _ = (
            source_id,
            target_id,
            input_ptr,
            input_len,
            output_ptr,
            output_capacity,
        );
        LOCALMT_FFI_TOKENIZER_DISABLED
    }

    #[cfg(feature = "hf-tokenizers")]
    {
        let Some(source) = language_from_id(source_id) else {
            return LOCALMT_FFI_INVALID_LANGUAGE;
        };
        let Some(target) = language_from_id(target_id) else {
            return LOCALMT_FFI_INVALID_LANGUAGE;
        };

        let input = match read_ffi_utf8(input_ptr, input_len) {
            Ok(value) => value,
            Err(status) => return status,
        };
        let text = match NonEmptyText::new(input.to_owned()) {
            Ok(value) => value,
            Err(_error) => return LOCALMT_FFI_TEXT_ERROR,
        };
        let request = match TranslateRequest::new(source, target, text) {
            Ok(value) => value,
            Err(_error) => return LOCALMT_FFI_INVALID_PAIR,
        };
        let translator = unsafe { &*translator }; // SAFETY: non-null live handle pointer.
        let translation = match translator.translator.translate(&request) {
            Ok(value) => value,
            Err(_error) => return LOCALMT_FFI_TRANSLATION_ERROR,
        };
        let output = translation.text().as_str().as_bytes();

        unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

        if output_capacity < output.len() {
            return LOCALMT_FFI_BUFFER_TOO_SMALL;
        }
        if output_ptr.is_null() {
            return LOCALMT_FFI_NULL_POINTER;
        }

        unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

        LOCALMT_FFI_OK
    }
}

/// { true }
/// fn localmt_ffi_ort_runtime_enabled() -> u8
/// { ret is 1 only when localmt-ffi was built with ort-runtime }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_ort_runtime_enabled() -> u8 {
    u8::from(cfg!(feature = "ort-runtime"))
}

/// { path_ptr points to path_len readable bytes }
/// fn localmt_ffi_ort_runtime_configure(path_ptr: *const u8, path_len: usize) -> i32
/// { ret is OK only when ORT runtime can use path as its explicit dylib path }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
pub extern "C" fn localmt_ffi_ort_runtime_configure(path_ptr: *const u8, path_len: usize) -> i32 {
    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };

    #[cfg(not(feature = "ort-runtime"))]
    {
        let _ = path;
        LOCALMT_FFI_RUNTIME_DISABLED
    }

    #[cfg(feature = "ort-runtime")]
    {
        configure_ort_dylib_path(path)
            .map(|()| LOCALMT_FFI_OK)
            .unwrap_or_else(ffi_ort_engine_error_status)
    }
}

/// { true }
/// fn localmt_ffi_hf_tokenizer_enabled() -> u8
/// { ret is 1 only when localmt-ffi was built with hf-tokenizers }
#[unsafe(no_mangle)] // SAFETY: pointer-free C export.
pub extern "C" fn localmt_ffi_hf_tokenizer_enabled() -> u8 {
    u8::from(cfg!(feature = "hf-tokenizers"))
}

/// { path_ptr points to path_len readable bytes and out_tokenizer is writable }
/// fn localmt_ffi_hf_tokenizer_open(path_ptr: *const u8, path_len: usize, out_tokenizer: *mut *mut LocalmtFfiHfTokenizer) -> i32
/// { ret is OK only when out_tokenizer receives an owned non-null HF tokenizer handle }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_hf_tokenizer_open(
    path_ptr: *const u8,
    path_len: usize,
    out_tokenizer: *mut *mut LocalmtFfiHfTokenizer,
) -> i32 {
    if out_tokenizer.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *out_tokenizer = ptr::null_mut() }; // SAFETY: non-null writable out pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let assets = match OfflineTranslatorAssets::from_model_pack_path(path) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_MODEL_PACK_ERROR,
    };

    #[cfg(not(feature = "hf-tokenizers"))]
    {
        let _tokenizer_path = assets.plan().tokenizer().tokenizer_path();
        LOCALMT_FFI_TOKENIZER_DISABLED
    }

    #[cfg(feature = "hf-tokenizers")]
    {
        match HfTokenizer::from_file(assets.plan().tokenizer().tokenizer_path()) {
            Ok(tokenizer) => {
                let handle = Box::new(LocalmtFfiHfTokenizer {
                    _tokenizer: tokenizer,
                });
                unsafe { *out_tokenizer = Box::into_raw(handle) }; // SAFETY: non-null writable out pointer.
                LOCALMT_FFI_OK
            }
            Err(_error) => LOCALMT_FFI_TOKENIZER_ERROR,
        }
    }
}

/// { tokenizer is null or was returned by localmt_ffi_hf_tokenizer_open }
/// fn localmt_ffi_hf_tokenizer_close(tokenizer: *mut LocalmtFfiHfTokenizer)
/// { tokenizer is consumed when non-null }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_hf_tokenizer_close(tokenizer: *mut LocalmtFfiHfTokenizer) {
    if tokenizer.is_null() {
        return;
    }

    unsafe { drop(Box::from_raw(tokenizer)) }; // SAFETY: handle came from open and closes once.
}

/// { path_ptr points to path_len readable bytes and out_generator is writable }
/// fn localmt_ffi_ort_generator_open(path_ptr: *const u8, path_len: usize, out_generator: *mut *mut LocalmtFfiOrtGenerator) -> i32
/// { ret is OK only when out_generator receives an owned non-null ORT generator handle }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_ort_generator_open(
    path_ptr: *const u8,
    path_len: usize,
    out_generator: *mut *mut LocalmtFfiOrtGenerator,
) -> i32 {
    if out_generator.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *out_generator = ptr::null_mut() }; // SAFETY: non-null writable out pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let assets = match OfflineTranslatorAssets::from_model_pack_path(path) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_MODEL_PACK_ERROR,
    };
    let plan = assets.plan().generator().clone();

    match OrtTokenGenerator::load(plan) {
        Ok(generator) => {
            let handle = Box::new(LocalmtFfiOrtGenerator {
                _generator: generator,
            });
            unsafe { *out_generator = Box::into_raw(handle) }; // SAFETY: non-null writable out pointer.
            LOCALMT_FFI_OK
        }
        Err(error) => ffi_ort_engine_error_status(error),
    }
}

/// { generator is null or was returned by localmt_ffi_ort_generator_open }
/// fn localmt_ffi_ort_generator_close(generator: *mut LocalmtFfiOrtGenerator)
/// { generator is consumed when non-null }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_ort_generator_close(generator: *mut LocalmtFfiOrtGenerator) {
    if generator.is_null() {
        return;
    }

    unsafe { drop(Box::from_raw(generator)) }; // SAFETY: handle came from open and closes once.
}

/// { path_ptr points to path_len readable bytes and out_translator is writable }
/// fn localmt_ffi_ort_translator_open(path_ptr: *const u8, path_len: usize, out_translator: *mut *mut LocalmtFfiOrtTranslator) -> i32
/// { ret is OK only when out_translator receives an owned non-null ORT translator handle }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_ort_translator_open(
    path_ptr: *const u8,
    path_len: usize,
    out_translator: *mut *mut LocalmtFfiOrtTranslator,
) -> i32 {
    if out_translator.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *out_translator = ptr::null_mut() }; // SAFETY: non-null writable out pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let assets = match OfflineTranslatorAssets::from_model_pack_path(path) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_MODEL_PACK_ERROR,
    };

    #[cfg(not(feature = "hf-tokenizers"))]
    {
        let _tokenizer_path = assets.plan().tokenizer().tokenizer_path();
        LOCALMT_FFI_TOKENIZER_DISABLED
    }

    #[cfg(all(feature = "hf-tokenizers", not(feature = "ort-runtime")))]
    {
        let _generator = assets.plan().generator();
        LOCALMT_FFI_RUNTIME_DISABLED
    }

    #[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
    {
        match OrtOfflineTranslator::from_assets(assets) {
            Ok(translator) => {
                let handle = Box::new(LocalmtFfiOrtTranslator { translator });
                unsafe { *out_translator = Box::into_raw(handle) }; // SAFETY: non-null writable out pointer.
                LOCALMT_FFI_OK
            }
            Err(OrtOfflineTranslatorError::Assets(_error)) => LOCALMT_FFI_MODEL_PACK_ERROR,
            Err(OrtOfflineTranslatorError::Tokenizer(_error)) => LOCALMT_FFI_TOKENIZER_ERROR,
            Err(OrtOfflineTranslatorError::Generator(error)) => ffi_ort_engine_error_status(error),
        }
    }
}

/// { error came from the ORT engine boundary }
/// fn ffi_ort_engine_error_status(error: OrtEngineError) -> i32
/// { ret is the stable FFI status for the engine error class }
fn ffi_ort_engine_error_status(error: OrtEngineError) -> i32 {
    match error {
        OrtEngineError::OrtRuntimeFeatureDisabled => LOCALMT_FFI_RUNTIME_DISABLED,
        OrtEngineError::MissingOrtDylibPath | OrtEngineError::InvalidOrtDylibPath { .. } => {
            LOCALMT_FFI_RUNTIME_NOT_CONFIGURED
        }
        _ => LOCALMT_FFI_ORT_ERROR,
    }
}

/// { translator is null or was returned by localmt_ffi_ort_translator_open }
/// fn localmt_ffi_ort_translator_close(translator: *mut LocalmtFfiOrtTranslator)
/// { translator is consumed when non-null }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_ort_translator_close(translator: *mut LocalmtFfiOrtTranslator) {
    if translator.is_null() {
        return;
    }

    unsafe { drop(Box::from_raw(translator)) }; // SAFETY: handle came from open and closes once.
}

/// { translator is a valid ORT translator handle, input/output/written pointers follow the header contract }
/// fn localmt_ffi_ort_translate(translator: *const LocalmtFfiOrtTranslator, source_id: u8, target_id: u8, input_ptr: *const u8, input_len: usize, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_ort_translate(
    translator: *const LocalmtFfiOrtTranslator,
    source_id: u8,
    target_id: u8,
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if translator.is_null() || written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *written_len = 0 }; // SAFETY: non-null writable length pointer.

    #[cfg(not(feature = "hf-tokenizers"))]
    {
        let _ = (
            source_id,
            target_id,
            input_ptr,
            input_len,
            output_ptr,
            output_capacity,
        );
        LOCALMT_FFI_TOKENIZER_DISABLED
    }

    #[cfg(all(feature = "hf-tokenizers", not(feature = "ort-runtime")))]
    {
        let _ = (
            source_id,
            target_id,
            input_ptr,
            input_len,
            output_ptr,
            output_capacity,
        );
        LOCALMT_FFI_RUNTIME_DISABLED
    }

    #[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
    {
        let Some(source) = language_from_id(source_id) else {
            return LOCALMT_FFI_INVALID_LANGUAGE;
        };
        let Some(target) = language_from_id(target_id) else {
            return LOCALMT_FFI_INVALID_LANGUAGE;
        };

        let input = match read_ffi_utf8(input_ptr, input_len) {
            Ok(value) => value,
            Err(status) => return status,
        };
        let text = match NonEmptyText::new(input.to_owned()) {
            Ok(value) => value,
            Err(_error) => return LOCALMT_FFI_TEXT_ERROR,
        };
        let request = match TranslateRequest::new(source, target, text) {
            Ok(value) => value,
            Err(_error) => return LOCALMT_FFI_INVALID_PAIR,
        };
        let translator = unsafe { &*translator }; // SAFETY: non-null live handle pointer.
        let translation = match translator.translator.translate(&request) {
            Ok(value) => value,
            Err(_error) => return LOCALMT_FFI_TRANSLATION_ERROR,
        };
        let output = translation.text().as_str().as_bytes();

        unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

        if output_capacity < output.len() {
            return LOCALMT_FFI_BUFFER_TOO_SMALL;
        }
        if output_ptr.is_null() {
            return LOCALMT_FFI_NULL_POINTER;
        }

        unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

        LOCALMT_FFI_OK
    }
}

/// { language_id may be any u8 }
/// fn language_from_id(language_id: u8) -> Option<Language>
/// { ret is Some only when language_id is in the stable FFI language table }
fn language_from_id(language_id: u8) -> Option<Language> {
    LANGUAGES.get(usize::from(language_id)).copied()
}

/// { language is supported by localmt }
/// fn language_code(language: Language) -> LocalmtFfiLanguageCode
/// { ret contains the two ASCII ISO 639-1 bytes for language }
fn language_code(language: Language) -> LocalmtFfiLanguageCode {
    let bytes = language.iso_639_1().as_bytes();
    LocalmtFfiLanguageCode::new(bytes[0], bytes[1])
}

/// { ptr points to len readable bytes }
/// fn read_ffi_utf8(ptr: *const u8, len: usize) -> Result<&str, i32>
/// { ret is Ok only when ptr is non-null and bytes are valid UTF-8 }
fn read_ffi_utf8<'a>(ptr: *const u8, len: usize) -> Result<&'a str, i32> {
    if ptr.is_null() {
        return Err(LOCALMT_FFI_NULL_POINTER);
    }

    let bytes = unsafe { slice::from_raw_parts(ptr, len) }; // SAFETY: non-null readable byte slice.

    str::from_utf8(bytes).map_err(|_error| LOCALMT_FFI_INVALID_UTF8)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::ptr;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(not(feature = "hf-tokenizers"))]
    use super::LOCALMT_FFI_TOKENIZER_DISABLED;
    #[cfg(feature = "hf-tokenizers")]
    use super::LOCALMT_FFI_TOKENIZER_ERROR;
    use super::{
        LOCALMT_FFI_BUFFER_TOO_SMALL, LOCALMT_FFI_INVALID_LANGUAGE, LOCALMT_FFI_INVALID_PAIR,
        LOCALMT_FFI_INVALID_UTF8, LOCALMT_FFI_MODEL_PACK_ERROR, LOCALMT_FFI_NULL_POINTER,
        LOCALMT_FFI_OK, LOCALMT_FFI_RUNTIME_NOT_CONFIGURED, LOCALMT_FFI_TEXT_ERROR,
        LocalmtFfiHfMockTranslator, LocalmtFfiHfTokenizer, LocalmtFfiOrtGenerator,
        LocalmtFfiOrtTranslator, LocalmtFfiTranslator, OrtEngineError, ffi_ort_engine_error_status,
        localmt_ffi_abi_version, localmt_ffi_hf_mock_translate,
        localmt_ffi_hf_mock_translator_close, localmt_ffi_hf_mock_translator_open,
        localmt_ffi_hf_tokenizer_close, localmt_ffi_hf_tokenizer_enabled,
        localmt_ffi_hf_tokenizer_open, localmt_ffi_language_code,
        localmt_ffi_language_from_iso_639_1, localmt_ffi_max_text_chars,
        localmt_ffi_mock_translate, localmt_ffi_mock_translator_close,
        localmt_ffi_mock_translator_open, localmt_ffi_model_pack_summary,
        localmt_ffi_ort_generator_open, localmt_ffi_ort_runtime_configure,
        localmt_ffi_ort_runtime_enabled, localmt_ffi_ort_translate,
        localmt_ffi_ort_translator_close, localmt_ffi_ort_translator_open,
        localmt_ffi_runtime_config_summary, localmt_ffi_startup_summary,
        localmt_ffi_status_message, localmt_ffi_supported_language_count,
        localmt_ffi_validate_language_pair, localmt_ffi_xiaomi17_android_abi_code,
        localmt_ffi_xiaomi17_preferred_runtime_code, localmt_ffi_xiaomi17_ram_class_gib,
    };
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
    static PACK_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn ffi_reports_supported_language_codes() {
        assert_eq!(localmt_ffi_supported_language_count(), 5);
        assert_eq!(localmt_ffi_language_code(0).as_bytes(), [b'e', b'n']);
        assert_eq!(localmt_ffi_language_code(4).as_bytes(), [b'j', b'a']);
        assert_eq!(localmt_ffi_language_code(99).as_bytes(), [0, 0]);
        assert_eq!(localmt_ffi_language_from_iso_639_1(b'r', b'u'), 1);
        assert_eq!(localmt_ffi_language_from_iso_639_1(b'z', b'z'), -1);
    }

    #[test]
    fn ffi_validates_language_pairs_with_status_codes() {
        assert_eq!(localmt_ffi_validate_language_pair(0, 4), LOCALMT_FFI_OK);
        assert_eq!(
            localmt_ffi_validate_language_pair(0, 0),
            LOCALMT_FFI_INVALID_PAIR
        );
        assert_eq!(
            localmt_ffi_validate_language_pair(0, 99),
            LOCALMT_FFI_INVALID_LANGUAGE
        );
    }

    #[test]
    fn ffi_reports_abi_and_xiaomi17_contract() {
        assert_eq!(localmt_ffi_abi_version(), 11);
        assert_eq!(localmt_ffi_max_text_chars(), 4096);
        assert_eq!(localmt_ffi_xiaomi17_android_abi_code(), 1);
        assert_eq!(localmt_ffi_xiaomi17_ram_class_gib(), 12);
        assert_eq!(localmt_ffi_xiaomi17_preferred_runtime_code(), 1);
    }

    #[test]
    fn ffi_startup_summary_reports_android_contract() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_startup_summary(output.as_mut_ptr(), output.len(), &mut written_len),
            LOCALMT_FFI_OK
        );
        let summary = std::str::from_utf8(&output[..written_len])?;

        assert!(summary.contains("ffi_abi: 11"));
        assert!(summary.contains("max_text_chars: 4096"));
        assert!(summary.contains("xiaomi17_android_abi: arm64-v8a"));
        assert!(summary.contains("xiaomi17_ram_class_gib: 12"));
        assert!(summary.contains("xiaomi17_preferred_runtime: onnx-runtime-mobile-xnnpack"));
        assert!(summary.contains("languages: en, ru, th, vi, ja"));

        #[cfg(not(feature = "hf-tokenizers"))]
        assert!(summary.contains("hf_tokenizers: disabled"));
        #[cfg(feature = "hf-tokenizers")]
        assert!(summary.contains("hf_tokenizers: enabled"));
        #[cfg(not(feature = "ort-runtime"))]
        assert!(summary.contains("ort_runtime: disabled"));
        #[cfg(feature = "ort-runtime")]
        assert!(summary.contains("ort_runtime: enabled"));

        Ok(())
    }

    #[test]
    fn ffi_startup_summary_reports_required_buffer_len() {
        let mut output = [0_u8; 3];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_startup_summary(output.as_mut_ptr(), output.len(), &mut written_len),
            LOCALMT_FFI_BUFFER_TOO_SMALL
        );
        assert!(written_len > output.len());
    }

    #[test]
    fn ffi_startup_summary_rejects_nulls() {
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_startup_summary(output.as_mut_ptr(), output.len(), ptr::null_mut()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_startup_summary(ptr::null_mut(), output.len(), &mut written_len),
            LOCALMT_FFI_NULL_POINTER
        );
        assert!(written_len > 0);
    }

    #[test]
    fn ffi_status_message_reports_known_status_text() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = [0_u8; 64];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_status_message(
                LOCALMT_FFI_MODEL_PACK_ERROR,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_OK
        );
        assert_eq!(
            std::str::from_utf8(&output[..written_len])?,
            "model pack error"
        );

        Ok(())
    }

    #[test]
    fn ffi_status_message_reports_runtime_not_configured_text()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut output = [0_u8; 64];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_status_message(
                LOCALMT_FFI_RUNTIME_NOT_CONFIGURED,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_OK
        );
        assert_eq!(
            std::str::from_utf8(&output[..written_len])?,
            "runtime not configured"
        );

        Ok(())
    }

    #[test]
    fn ffi_maps_ort_runtime_configuration_errors_to_stable_status() {
        assert_eq!(
            ffi_ort_engine_error_status(OrtEngineError::MissingOrtDylibPath),
            LOCALMT_FFI_RUNTIME_NOT_CONFIGURED
        );
        assert_eq!(
            ffi_ort_engine_error_status(OrtEngineError::InvalidOrtDylibPath {
                path: "/tmp/missing-libonnxruntime.dylib".into(),
                reason: "path does not point to a file".to_owned(),
            }),
            LOCALMT_FFI_RUNTIME_NOT_CONFIGURED
        );
    }

    #[test]
    fn ffi_status_message_reports_unknown_status_text() -> Result<(), Box<dyn std::error::Error>> {
        let mut output = [0_u8; 64];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_status_message(99, output.as_mut_ptr(), output.len(), &mut written_len),
            LOCALMT_FFI_OK
        );
        assert_eq!(
            std::str::from_utf8(&output[..written_len])?,
            "unknown status"
        );

        Ok(())
    }

    #[test]
    fn ffi_status_message_reports_required_buffer_len() {
        let mut output = [0_u8; 3];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_status_message(
                LOCALMT_FFI_MODEL_PACK_ERROR,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_BUFFER_TOO_SMALL
        );
        assert_eq!(written_len, "model pack error".len());
    }

    #[test]
    fn ffi_status_message_rejects_nulls() {
        let mut output = [0_u8; 64];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_status_message(
                LOCALMT_FFI_OK,
                output.as_mut_ptr(),
                output.len(),
                ptr::null_mut(),
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_status_message(
                LOCALMT_FFI_OK,
                ptr::null_mut(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(written_len, "ok".len());
    }

    #[test]
    fn ffi_model_pack_summary_reports_verified_assets() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_model_pack_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_OK
        );

        let summary = std::str::from_utf8(&output[..written_len])?;
        assert!(summary.contains("planned: m2m100-418m-int8"));
        assert!(summary.contains("tokenizer:"));
        assert!(summary.contains("encoder:"));
        assert!(summary.contains("decoder:"));
        assert!(summary.contains("decoder_with_past: absent"));
        assert!(summary.contains("generation_config: absent"));
        assert!(summary.contains("ort_io_config: absent"));
        Ok(())
    }

    #[test]
    fn ffi_model_pack_summary_reports_required_buffer_len() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut output = [0_u8; 3];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_model_pack_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_BUFFER_TOO_SMALL
        );
        assert!(written_len > output.len());

        Ok(())
    }

    #[test]
    fn ffi_model_pack_summary_rejects_nulls_and_invalid_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_model_pack_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                ptr::null_mut(),
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_model_pack_summary(
                ptr::null(),
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_model_pack_summary(
                [0xff].as_ptr(),
                1,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert_eq!(
            localmt_ffi_model_pack_summary(
                path.as_ptr(),
                path.len(),
                ptr::null_mut(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_NULL_POINTER
        );

        Ok(())
    }

    #[test]
    fn ffi_model_pack_summary_maps_model_pack_errors() -> Result<(), Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = PACK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let missing_root = std::env::temp_dir().join(format!(
            "localmt-ffi-missing-summary-test-{nanos}-{counter}",
        ));
        let path = path_bytes(&missing_root)?;
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_model_pack_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_MODEL_PACK_ERROR
        );
        assert_eq!(written_len, 0);

        Ok(())
    }

    #[test]
    fn ffi_runtime_config_summary_reports_strict_ort_contract()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_runtime_config_pack()?;
        let path = path_bytes(&root)?;
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_runtime_config_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_OK
        );

        let summary = std::str::from_utf8(&output[..written_len])?;
        assert!(summary.contains("runtime_config: ok"));
        assert!(summary.contains("max_new_tokens: 32"));
        assert!(summary.contains("encoder_input_ids: encoder_input_ids"));
        assert!(summary.contains("decoder_input_ids: decoder_input_ids"));
        assert!(summary.contains("decoder_logits: decoder_logits"));
        Ok(())
    }

    #[test]
    fn ffi_runtime_config_summary_reports_required_buffer_len()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_runtime_config_pack()?;
        let path = path_bytes(&root)?;
        let mut output = [0_u8; 3];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_runtime_config_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_BUFFER_TOO_SMALL
        );
        assert!(written_len > output.len());

        Ok(())
    }

    #[test]
    fn ffi_runtime_config_summary_rejects_nulls_and_invalid_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_runtime_config_pack()?;
        let path = path_bytes(&root)?;
        let mut output = [0_u8; 512];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_runtime_config_summary(
                path.as_ptr(),
                path.len(),
                output.as_mut_ptr(),
                output.len(),
                ptr::null_mut(),
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_runtime_config_summary(
                ptr::null(),
                0,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(written_len, 0);
        assert_eq!(
            localmt_ffi_runtime_config_summary(
                [0xff].as_ptr(),
                1,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert_eq!(written_len, 0);
        assert_eq!(
            localmt_ffi_runtime_config_summary(
                path.as_ptr(),
                path.len(),
                ptr::null_mut(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_NULL_POINTER
        );
        assert!(written_len > 0);

        Ok(())
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn ffi_ort_runtime_enabled_reports_default_build() {
        assert_eq!(localmt_ffi_ort_runtime_enabled(), 0);
    }

    #[test]
    #[cfg(feature = "ort-runtime")]
    fn ffi_ort_runtime_enabled_reports_feature_build() {
        assert_eq!(localmt_ffi_ort_runtime_enabled(), 1);
    }

    #[test]
    fn ffi_ort_runtime_configure_rejects_nulls_and_invalid_utf8() {
        let path = "/tmp/libonnxruntime.so";

        assert_eq!(
            localmt_ffi_ort_runtime_configure(ptr::null(), path.len()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_ort_runtime_configure([0xff].as_ptr(), 1),
            LOCALMT_FFI_INVALID_UTF8
        );
    }

    #[test]
    fn ffi_ort_runtime_configure_reports_runtime_status_for_missing_path() {
        let path = "/tmp/localmt-missing-libonnxruntime.so";
        let status = localmt_ffi_ort_runtime_configure(path.as_ptr(), path.len());

        #[cfg(feature = "ort-runtime")]
        assert_eq!(status, LOCALMT_FFI_RUNTIME_NOT_CONFIGURED);
        #[cfg(not(feature = "ort-runtime"))]
        assert_eq!(status, super::LOCALMT_FFI_RUNTIME_DISABLED);
    }

    #[test]
    #[cfg(not(feature = "ort-runtime"))]
    fn ffi_ort_generator_open_reports_runtime_disabled() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut generator: *mut LocalmtFfiOrtGenerator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_ort_generator_open(path.as_ptr(), path.len(), &mut generator),
            super::LOCALMT_FFI_RUNTIME_DISABLED
        );
        assert!(generator.is_null());

        super::localmt_ffi_ort_generator_close(generator);
        Ok(())
    }

    #[test]
    fn ffi_ort_generator_open_rejects_nulls_and_invalid_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut generator: *mut LocalmtFfiOrtGenerator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_ort_generator_open(path.as_ptr(), path.len(), ptr::null_mut()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_ort_generator_open([0xff].as_ptr(), 1, &mut generator),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert!(generator.is_null());

        Ok(())
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn ffi_ort_translator_open_reports_tokenizer_disabled() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_runtime_config_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiOrtTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_ort_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_TOKENIZER_DISABLED
        );
        assert!(translator.is_null());

        localmt_ffi_ort_translator_close(translator);
        Ok(())
    }

    #[test]
    fn ffi_ort_translator_open_rejects_nulls_and_invalid_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_runtime_config_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiOrtTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_ort_translator_open(path.as_ptr(), path.len(), ptr::null_mut()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_ort_translator_open([0xff].as_ptr(), 1, &mut translator),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert!(translator.is_null());
        Ok(())
    }

    #[test]
    fn ffi_ort_translator_close_accepts_null_handle() {
        localmt_ffi_ort_translator_close(ptr::null_mut());
    }

    #[test]
    fn ffi_ort_translate_rejects_null_handle() {
        let mut output = [0_u8; 32];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_ort_translate(
                ptr::null(),
                0,
                1,
                b"hello".as_ptr(),
                5,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_NULL_POINTER
        );
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn ffi_hf_tokenizer_enabled_reports_default_build() {
        assert_eq!(localmt_ffi_hf_tokenizer_enabled(), 0);
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn ffi_hf_tokenizer_enabled_reports_feature_build() {
        assert_eq!(localmt_ffi_hf_tokenizer_enabled(), 1);
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn ffi_hf_tokenizer_open_reports_feature_disabled() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut tokenizer: *mut LocalmtFfiHfTokenizer = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_tokenizer_open(path.as_ptr(), path.len(), &mut tokenizer),
            LOCALMT_FFI_TOKENIZER_DISABLED
        );
        assert!(tokenizer.is_null());

        localmt_ffi_hf_tokenizer_close(tokenizer);
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn ffi_hf_tokenizer_open_loads_verified_tokenizer() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_hf_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut tokenizer: *mut LocalmtFfiHfTokenizer = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_tokenizer_open(path.as_ptr(), path.len(), &mut tokenizer),
            LOCALMT_FFI_OK
        );
        assert!(!tokenizer.is_null());

        localmt_ffi_hf_tokenizer_close(tokenizer);
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn ffi_hf_tokenizer_open_maps_tokenizer_load_errors() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut tokenizer: *mut LocalmtFfiHfTokenizer = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_tokenizer_open(path.as_ptr(), path.len(), &mut tokenizer),
            LOCALMT_FFI_TOKENIZER_ERROR
        );
        assert!(tokenizer.is_null());

        Ok(())
    }

    #[test]
    fn ffi_hf_tokenizer_open_rejects_nulls_and_invalid_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut tokenizer: *mut LocalmtFfiHfTokenizer = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_tokenizer_open(path.as_ptr(), path.len(), ptr::null_mut()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_hf_tokenizer_open([0xff].as_ptr(), 1, &mut tokenizer),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert!(tokenizer.is_null());

        Ok(())
    }

    #[test]
    #[cfg(not(feature = "hf-tokenizers"))]
    fn ffi_hf_mock_translator_open_reports_feature_disabled()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiHfMockTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_mock_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_TOKENIZER_DISABLED
        );
        assert!(translator.is_null());

        localmt_ffi_hf_mock_translator_close(translator);
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn ffi_hf_mock_translator_opens_translates_and_closes() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_hf_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiHfMockTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_mock_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_OK
        );
        assert!(!translator.is_null());

        let input = "hello offline";
        let mut output = [0_u8; 32];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_hf_mock_translate(
                translator,
                0,
                1,
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_OK
        );
        assert_eq!(written_len, input.len());
        assert_eq!(&output[..written_len], input.as_bytes());

        localmt_ffi_hf_mock_translator_close(translator);
        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn ffi_hf_mock_translator_open_maps_tokenizer_errors() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiHfMockTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_mock_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_TOKENIZER_ERROR
        );
        assert!(translator.is_null());

        Ok(())
    }

    #[test]
    fn ffi_hf_mock_translator_rejects_nulls_and_invalid_utf8()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiHfMockTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_hf_mock_translator_open(path.as_ptr(), path.len(), ptr::null_mut()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_hf_mock_translator_open([0xff].as_ptr(), 1, &mut translator),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert!(translator.is_null());

        assert_eq!(
            localmt_ffi_hf_mock_translate(
                ptr::null(),
                0,
                1,
                b"hello".as_ptr(),
                5,
                [0_u8; 8].as_mut_ptr(),
                8,
                &mut 0_usize,
            ),
            LOCALMT_FFI_NULL_POINTER
        );

        Ok(())
    }

    #[test]
    #[cfg(feature = "hf-tokenizers")]
    fn ffi_hf_mock_translate_reports_required_buffer_len() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_hf_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiHfMockTranslator = ptr::null_mut();
        assert_eq!(
            localmt_ffi_hf_mock_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_OK
        );
        let input = "hello offline";
        let mut output = [0_u8; 3];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_hf_mock_translate(
                translator,
                0,
                1,
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_BUFFER_TOO_SMALL
        );
        assert_eq!(written_len, input.len());

        localmt_ffi_hf_mock_translator_close(translator);
        Ok(())
    }

    #[test]
    fn ffi_mock_translator_opens_translates_and_closes() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_mock_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_OK
        );
        assert!(!translator.is_null());

        let input = "hello offline";
        let mut output = [0_u8; 32];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_mock_translate(
                translator,
                0,
                1,
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_OK
        );
        assert_eq!(written_len, input.len());
        assert_eq!(&output[..written_len], input.as_bytes());

        localmt_ffi_mock_translator_close(translator);
        Ok(())
    }

    #[test]
    fn ffi_mock_translate_reports_required_buffer_len() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let translator = open_translator(&root)?;
        let input = "too wide";
        let mut output = [0_u8; 3];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_mock_translate(
                translator,
                0,
                1,
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_BUFFER_TOO_SMALL
        );
        assert_eq!(written_len, input.len());

        localmt_ffi_mock_translator_close(translator);
        Ok(())
    }

    #[test]
    fn ffi_mock_translator_rejects_nulls_and_invalid_utf8() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = create_verified_pack()?;
        let path = path_bytes(&root)?;
        let mut translator: *mut LocalmtFfiTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_mock_translator_open(path.as_ptr(), path.len(), ptr::null_mut()),
            LOCALMT_FFI_NULL_POINTER
        );
        assert_eq!(
            localmt_ffi_mock_translator_open([0xff].as_ptr(), 1, &mut translator),
            LOCALMT_FFI_INVALID_UTF8
        );
        assert!(translator.is_null());

        assert_eq!(
            localmt_ffi_mock_translate(
                ptr::null(),
                0,
                1,
                b"hello".as_ptr(),
                5,
                [0_u8; 8].as_mut_ptr(),
                8,
                &mut 0_usize,
            ),
            LOCALMT_FFI_NULL_POINTER
        );

        Ok(())
    }

    #[test]
    fn ffi_mock_translate_maps_validation_errors() -> Result<(), Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;
        let translator = open_translator(&root)?;
        let mut output = [0_u8; 32];
        let mut written_len = 0_usize;

        assert_eq!(
            localmt_ffi_mock_translate(
                translator,
                0,
                0,
                b"hello".as_ptr(),
                5,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_INVALID_PAIR
        );
        assert_eq!(
            localmt_ffi_mock_translate(
                translator,
                99,
                0,
                b"hello".as_ptr(),
                5,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_INVALID_LANGUAGE
        );
        assert_eq!(
            localmt_ffi_mock_translate(
                translator,
                0,
                1,
                b"   ".as_ptr(),
                3,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_TEXT_ERROR
        );
        assert_eq!(
            localmt_ffi_mock_translate(
                translator,
                0,
                1,
                [0xff].as_ptr(),
                1,
                output.as_mut_ptr(),
                output.len(),
                &mut written_len,
            ),
            LOCALMT_FFI_INVALID_UTF8
        );

        localmt_ffi_mock_translator_close(translator);
        Ok(())
    }

    fn open_translator(
        root: &std::path::Path,
    ) -> Result<*mut LocalmtFfiTranslator, Box<dyn std::error::Error>> {
        let path = path_bytes(root)?;
        let mut translator: *mut LocalmtFfiTranslator = ptr::null_mut();

        assert_eq!(
            localmt_ffi_mock_translator_open(path.as_ptr(), path.len(), &mut translator),
            LOCALMT_FFI_OK
        );

        Ok(translator)
    }

    fn path_bytes(path: &std::path::Path) -> Result<&str, Box<dyn std::error::Error>> {
        path.to_str().ok_or_else(|| {
            Box::<dyn std::error::Error>::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "test path is not UTF-8",
            ))
        })
    }

    fn create_verified_pack() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = PACK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("localmt-ffi-mock-test-{nanos}-{counter}",));

        fs::create_dir_all(&root)?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), "tokenizer\n")?;
        fs::write(root.join("manifest.json"), manifest_json())?;

        Ok(root)
    }

    fn create_runtime_config_pack() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let root = create_verified_pack()?;

        fs::write(root.join("generation.json"), GENERATION_CONFIG)?;
        fs::write(root.join("config.json"), ORT_IO_CONFIG)?;
        fs::write(root.join("manifest.json"), runtime_config_manifest_json())?;

        Ok(root)
    }

    #[cfg(feature = "hf-tokenizers")]
    fn create_hf_verified_pack() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let counter = PACK_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("localmt-ffi-hf-test-{nanos}-{counter}",));

        fs::create_dir_all(&root)?;
        fs::write(root.join("encoder.onnx"), "encoder\n")?;
        fs::write(root.join("decoder.onnx"), "decoder\n")?;
        fs::write(root.join("tokenizer.json"), wordlevel_tokenizer_json())?;
        let tokenizer_sha256 = Sha256Digest::from_file(root.join("tokenizer.json"))?;
        fs::write(
            root.join("manifest.json"),
            manifest_json_with_tokenizer(tokenizer_sha256.as_str()),
        )?;

        Ok(root)
    }

    fn manifest_json() -> String {
        manifest_json_with_tokenizer(TOKENIZER_SHA256)
    }

    fn manifest_json_with_tokenizer(tokenizer_sha256: &str) -> String {
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
    {{ "path": "tokenizer.json", "kind": "tokenizer", "sha256": "{tokenizer_sha256}" }}
  ]
}}"#
        )
    }

    fn runtime_config_manifest_json() -> String {
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
        )
    }

    #[cfg(feature = "hf-tokenizers")]
    fn wordlevel_tokenizer_json() -> &'static str {
        r#"{"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{"type":"WhitespaceSplit"},"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"[UNK]":0,"hello":1,"offline":2},"unk_token":"[UNK]"}}"#
    }
}
