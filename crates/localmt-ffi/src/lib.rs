//! Pointer-free C ABI for localmt Android adapters.

use localmt::{DeviceProfile, Language, LanguagePair, MAX_TEXT_CHARS};

/// FFI status for successful calls.
pub const LOCALMT_FFI_OK: i32 = 0;
/// FFI status for invalid language ids.
pub const LOCALMT_FFI_INVALID_LANGUAGE: i32 = 1;
/// FFI status for invalid language pairs.
pub const LOCALMT_FFI_INVALID_PAIR: i32 = 2;

/// Pointer-free C ABI version.
pub const LOCALMT_FFI_ABI_VERSION: u32 = 1;
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

#[cfg(test)]
mod tests {
    use super::{
        LOCALMT_FFI_INVALID_LANGUAGE, LOCALMT_FFI_INVALID_PAIR, LOCALMT_FFI_OK,
        localmt_ffi_abi_version, localmt_ffi_language_code, localmt_ffi_language_from_iso_639_1,
        localmt_ffi_max_text_chars, localmt_ffi_supported_language_count,
        localmt_ffi_validate_language_pair, localmt_ffi_xiaomi17_android_abi_code,
        localmt_ffi_xiaomi17_preferred_runtime_code, localmt_ffi_xiaomi17_ram_class_gib,
    };

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
        assert_eq!(localmt_ffi_abi_version(), 1);
        assert_eq!(localmt_ffi_max_text_chars(), 4096);
        assert_eq!(localmt_ffi_xiaomi17_android_abi_code(), 1);
        assert_eq!(localmt_ffi_xiaomi17_ram_class_gib(), 12);
        assert_eq!(localmt_ffi_xiaomi17_preferred_runtime_code(), 1);
    }
}
