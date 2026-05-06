#ifndef LOCALMT_FFI_H
#define LOCALMT_FFI_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LOCALMT_FFI_OK 0
#define LOCALMT_FFI_INVALID_LANGUAGE 1
#define LOCALMT_FFI_INVALID_PAIR 2

#define LOCALMT_FFI_ABI_VERSION 1
#define LOCALMT_FFI_ANDROID_ABI_ARM64_V8A 1
#define LOCALMT_FFI_RUNTIME_ONNX_MOBILE_XNNPACK 1

/* Two-byte ISO 639-1 language code. Invalid ids return {0, 0}. */
typedef struct LocalmtFfiLanguageCode {
  uint8_t first;
  uint8_t second;
} LocalmtFfiLanguageCode;

/* Returns LOCALMT_FFI_ABI_VERSION. */
uint32_t localmt_ffi_abi_version(void);
/* Returns the number of stable language ids. */
size_t localmt_ffi_supported_language_count(void);
/* Maps a stable language id to a two-byte ISO 639-1 code. */
LocalmtFfiLanguageCode localmt_ffi_language_code(uint8_t language_id);
/* Returns the stable language id, or -1 when the code is unsupported. */
int32_t localmt_ffi_language_from_iso_639_1(uint8_t first, uint8_t second);
/* Returns LOCALMT_FFI_OK, LOCALMT_FFI_INVALID_LANGUAGE, or LOCALMT_FFI_INVALID_PAIR. */
int32_t localmt_ffi_validate_language_pair(uint8_t source_id, uint8_t target_id);
/* Returns the Rust facade max input length in Unicode scalar values. */
size_t localmt_ffi_max_text_chars(void);
/* Xiaomi 17 target metadata encoded as stable small integers. */
uint16_t localmt_ffi_xiaomi17_android_abi_code(void);
uint16_t localmt_ffi_xiaomi17_ram_class_gib(void);
uint16_t localmt_ffi_xiaomi17_preferred_runtime_code(void);

#ifdef __cplusplus
}
#endif

#endif
