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
#define LOCALMT_FFI_NULL_POINTER 3
#define LOCALMT_FFI_INVALID_UTF8 4
#define LOCALMT_FFI_MODEL_PACK_ERROR 5
#define LOCALMT_FFI_TEXT_ERROR 6
#define LOCALMT_FFI_TRANSLATION_ERROR 7
#define LOCALMT_FFI_BUFFER_TOO_SMALL 8
#define LOCALMT_FFI_RUNTIME_DISABLED 9
#define LOCALMT_FFI_ORT_ERROR 10
#define LOCALMT_FFI_TOKENIZER_DISABLED 11
#define LOCALMT_FFI_TOKENIZER_ERROR 12

#define LOCALMT_FFI_ABI_VERSION 4
#define LOCALMT_FFI_ANDROID_ABI_ARM64_V8A 1
#define LOCALMT_FFI_RUNTIME_ONNX_MOBILE_XNNPACK 1

/* Two-byte ISO 639-1 language code. Invalid ids return {0, 0}. */
typedef struct LocalmtFfiLanguageCode {
  uint8_t first;
  uint8_t second;
} LocalmtFfiLanguageCode;

/*
 * Opaque Rust-owned translator handle.
 *
 * Handles are created by localmt_ffi_mock_translator_open and must be released
 * exactly once with localmt_ffi_mock_translator_close. Passing a non-null
 * pointer not created by open, or closing the same handle twice, is invalid.
 */
typedef struct LocalmtFfiTranslator LocalmtFfiTranslator;

/*
 * Opaque Rust-owned HF-tokenizer mock translator handle.
 *
 * Handles are created by localmt_ffi_hf_mock_translator_open and must be
 * released exactly once with localmt_ffi_hf_mock_translator_close. This path
 * loads the verified tokenizer.json and preserves the translation-shaped ABI,
 * but token generation is still deterministic mock generation.
 */
typedef struct LocalmtFfiHfMockTranslator LocalmtFfiHfMockTranslator;

/*
 * Opaque Rust-owned ORT generator preflight handle.
 *
 * Handles are created by localmt_ffi_ort_generator_open and must be released
 * exactly once with localmt_ffi_ort_generator_close. This handle proves that
 * generator sessions loaded; it is not a translation API.
 */
typedef struct LocalmtFfiOrtGenerator LocalmtFfiOrtGenerator;

/*
 * Opaque Rust-owned HF tokenizer preflight handle.
 *
 * Handles are created by localmt_ffi_hf_tokenizer_open and must be released
 * exactly once with localmt_ffi_hf_tokenizer_close. This handle proves that a
 * verified tokenizer.json loaded; it is not a translation API.
 */
typedef struct LocalmtFfiHfTokenizer LocalmtFfiHfTokenizer;

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

/*
 * Returns 1 when localmt-ffi was built with the ort-runtime feature, otherwise 0.
 */
uint8_t localmt_ffi_ort_runtime_enabled(void);

/*
 * Returns 1 when localmt-ffi was built with the hf-tokenizers feature,
 * otherwise 0.
 */
uint8_t localmt_ffi_hf_tokenizer_enabled(void);

/*
 * Opens a verified local model pack and constructs the deterministic mock
 * translator used for Android/JNI smoke checks before real inference exists.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * out_translator must point to writable pointer storage. It is set to NULL
 * before work and receives a non-null handle only on LOCALMT_FFI_OK.
 */
int32_t localmt_ffi_mock_translator_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiTranslator **out_translator);

/*
 * Releases a mock translator handle. NULL is accepted as a no-op.
 */
void localmt_ffi_mock_translator_close(LocalmtFfiTranslator *translator);

/*
 * Translates UTF-8 bytes through the deterministic mock translator.
 *
 * input_ptr/input_len must be valid UTF-8 bytes. output_ptr/output_capacity is
 * caller-owned byte storage and is not NUL terminated by Rust. written_len must
 * point to writable size_t storage. On LOCALMT_FFI_BUFFER_TOO_SMALL,
 * written_len contains the required byte count and output is not written.
 */
int32_t localmt_ffi_mock_translate(
    const LocalmtFfiTranslator *translator,
    uint8_t source_id,
    uint8_t target_id,
    const uint8_t *input_ptr,
    size_t input_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Opens a verified local model pack and constructs the HF-tokenizer-backed
 * mock translator.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * out_translator must point to writable pointer storage. It is set to NULL
 * before work and receives a non-null handle only on LOCALMT_FFI_OK.
 *
 * Default builds return LOCALMT_FFI_TOKENIZER_DISABLED after pack planning.
 * hf-tokenizers builds map tokenizer load failures to
 * LOCALMT_FFI_TOKENIZER_ERROR.
 */
int32_t localmt_ffi_hf_mock_translator_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiHfMockTranslator **out_translator);

/*
 * Releases an HF-tokenizer mock translator handle. NULL is accepted as a no-op.
 */
void localmt_ffi_hf_mock_translator_close(LocalmtFfiHfMockTranslator *translator);

/*
 * Translates UTF-8 bytes through the HF-tokenizer-backed mock translator.
 *
 * input_ptr/input_len must be valid UTF-8 bytes. output_ptr/output_capacity is
 * caller-owned byte storage and is not NUL terminated by Rust. written_len must
 * point to writable size_t storage. On LOCALMT_FFI_BUFFER_TOO_SMALL,
 * written_len contains the required byte count and output is not written.
 */
int32_t localmt_ffi_hf_mock_translate(
    const LocalmtFfiHfMockTranslator *translator,
    uint8_t source_id,
    uint8_t target_id,
    const uint8_t *input_ptr,
    size_t input_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Verifies a model pack and attempts to load tokenizer.json through the
 * feature-gated Hugging Face tokenizer backend.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * out_tokenizer must point to writable pointer storage. It is set to NULL
 * before work and receives a non-null handle only on LOCALMT_FFI_OK.
 *
 * Default builds return LOCALMT_FFI_TOKENIZER_DISABLED after pack planning.
 * hf-tokenizers builds map tokenizer load failures to
 * LOCALMT_FFI_TOKENIZER_ERROR.
 */
int32_t localmt_ffi_hf_tokenizer_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiHfTokenizer **out_tokenizer);

/*
 * Releases an HF tokenizer preflight handle. NULL is accepted as a no-op.
 */
void localmt_ffi_hf_tokenizer_close(LocalmtFfiHfTokenizer *tokenizer);

/*
 * Verifies a model pack and attempts to load ONNX generator sessions.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * out_generator must point to writable pointer storage. It is set to NULL
 * before work and receives a non-null handle only on LOCALMT_FFI_OK.
 *
 * Default builds return LOCALMT_FFI_RUNTIME_DISABLED after pack planning.
 * ort-runtime builds map ORT session-load failures to LOCALMT_FFI_ORT_ERROR.
 */
int32_t localmt_ffi_ort_generator_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiOrtGenerator **out_generator);

/*
 * Releases an ORT generator preflight handle. NULL is accepted as a no-op.
 */
void localmt_ffi_ort_generator_close(LocalmtFfiOrtGenerator *generator);

#ifdef __cplusplus
}
#endif

#endif
