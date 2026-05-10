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
#define LOCALMT_FFI_RUNTIME_NOT_CONFIGURED 13

#define LOCALMT_FFI_ABI_VERSION 14
#define LOCALMT_FFI_MODEL_PACK_TRUST_SCHEMA_VERSION 1
#define LOCALMT_FFI_ANDROID_ABI_ARM64_V8A 1
#define LOCALMT_FFI_RUNTIME_ONNX_MOBILE_XNNPACK 1
#define LOCALMT_FFI_RUNTIME_LLAMA_CPP 2

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
 * Opaque Rust-owned ORT translator handle.
 *
 * Handles are created by localmt_ffi_ort_translator_open and must be released
 * exactly once with localmt_ffi_ort_translator_close. This path uses the
 * verified tokenizer.json plus ORT encoder/decoder sessions for translation.
 */
typedef struct LocalmtFfiOrtTranslator LocalmtFfiOrtTranslator;

/*
 * Opaque Rust-owned llama.cpp translator handle.
 *
 * Handles are created by localmt_ffi_llama_translator_open and must be
 * released exactly once with localmt_ffi_llama_translator_close. This path
 * verifies GGUF/HY-MT packs and validates llama runtime metadata before
 * loading the native backend.
 */
typedef struct LocalmtFfiLlamaTranslator LocalmtFfiLlamaTranslator;

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
/* Returns LOCALMT_FFI_MODEL_PACK_TRUST_SCHEMA_VERSION. */
uint16_t localmt_ffi_model_pack_trust_schema_version(void);
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
/*
 * Writes a stable UTF-8 message for a status code.
 *
 * Unknown input status values write "unknown status" and still return
 * LOCALMT_FFI_OK when the message fits. output_ptr/output_capacity is
 * caller-owned byte storage and is not NUL terminated by Rust. written_len must
 * point to writable size_t storage. On LOCALMT_FFI_BUFFER_TOO_SMALL,
 * written_len contains the required byte count and output is not written.
 */
int32_t localmt_ffi_status_message(
    int32_t status,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Writes the Android startup contract for this localmt-ffi build.
 *
 * The summary includes ABI version, trust-artifact schema version, max text
 * length, Xiaomi 17 metadata, feature flags, and stable language codes.
 * output_ptr/output_capacity is caller-owned byte storage and is not NUL
 * terminated by Rust. written_len must point to writable size_t storage. On
 * LOCALMT_FFI_BUFFER_TOO_SMALL, written_len contains the required byte count
 * and output is not written.
 */
int32_t localmt_ffi_startup_summary(
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/* Xiaomi 17 target metadata encoded as stable small integers. */
uint16_t localmt_ffi_xiaomi17_android_abi_code(void);
uint16_t localmt_ffi_xiaomi17_ram_class_gib(void);
uint16_t localmt_ffi_xiaomi17_preferred_runtime_code(void);

/*
 * Verifies and plans a model pack, then writes a stable UTF-8 newline summary.
 *
 * This does not load tokenizer backends or ONNX Runtime sessions. It only
 * performs model-pack discovery, checksum verification, asset planning, and
 * optional generation_config parsing through the Rust facade.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * output_ptr/output_capacity is caller-owned byte storage and is not NUL
 * terminated by Rust. written_len must point to writable size_t storage. On
 * LOCALMT_FFI_BUFFER_TOO_SMALL, written_len contains the required byte count
 * and output is not written.
 */
int32_t localmt_ffi_model_pack_summary(
    const uint8_t *path_ptr,
    size_t path_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Verifies and plans a GGUF/llama.cpp model pack, validates llama runtime
 * configuration, then writes a stable UTF-8 newline summary.
 *
 * This does not load the native llama.cpp backend. It only performs model-pack
 * discovery, checksum verification, GGUF asset planning, and runtime config
 * parsing through the Rust facade.
 */
int32_t localmt_ffi_gguf_model_pack_summary(
    const uint8_t *path_ptr,
    size_t path_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Fully verifies a model pack and writes its local trust artifact.
 *
 * This is the install/update-time path: it performs checksum verification and
 * records a trusted metadata snapshot beside the model pack. Runtime hot paths
 * can later use the trusted APIs below to avoid re-hashing large model files.
 * The artifact schema is LOCALMT_FFI_MODEL_PACK_TRUST_SCHEMA_VERSION.
 */
int32_t localmt_ffi_model_pack_trust(
    const uint8_t *path_ptr,
    size_t path_len);

/*
 * Plans a model pack through the local trust artifact, then writes a summary.
 *
 * This does not re-hash model files. It requires a trust artifact previously
 * written by localmt_ffi_model_pack_trust and validates manifest hash,
 * manifest file identity, byte length, and modified timestamp before planning.
 */
int32_t localmt_ffi_model_pack_trusted_summary(
    const uint8_t *path_ptr,
    size_t path_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Verifies, plans, and strictly validates ORT runtime configuration.
 *
 * This does not load tokenizer backends or ONNX Runtime sessions. It requires
 * both generation_config and ort_io metadata to be present and valid, then
 * writes the selected generation limit and ONNX tensor names.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * output_ptr/output_capacity is caller-owned byte storage and is not NUL
 * terminated by Rust. written_len must point to writable size_t storage. On
 * LOCALMT_FFI_BUFFER_TOO_SMALL, written_len contains the required byte count
 * and output is not written.
 */
int32_t localmt_ffi_runtime_config_summary(
    const uint8_t *path_ptr,
    size_t path_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

/*
 * Returns 1 when localmt-ffi was built with the ort-runtime feature, otherwise 0.
 */
uint8_t localmt_ffi_ort_runtime_enabled(void);

/*
 * Configures the absolute ONNX Runtime dynamic-library path for ORT-enabled
 * builds. Call this before opening an ORT generator or translator when the app
 * wants to pass its nativeLibraryDir/libonnxruntime.so path through the C ABI
 * instead of relying on ORT_DYLIB_PATH.
 *
 * Default builds return LOCALMT_FFI_RUNTIME_DISABLED. ort-runtime builds return
 * LOCALMT_FFI_RUNTIME_NOT_CONFIGURED when the path is missing, relative, or not
 * a file.
 */
int32_t localmt_ffi_ort_runtime_configure(
    const uint8_t *path_ptr,
    size_t path_len);

/*
 * Returns 1 when localmt-ffi was built with the hf-tokenizers feature,
 * otherwise 0.
 */
uint8_t localmt_ffi_hf_tokenizer_enabled(void);

/*
 * Returns 1 when localmt-ffi was built with the llama-runtime feature,
 * otherwise 0.
 */
uint8_t localmt_ffi_llama_runtime_enabled(void);

/*
 * Configures the absolute llama.cpp dynamic-library path for llama-enabled
 * builds. Call this before opening a llama translator when the app wants to
 * pass its nativeLibraryDir/libllama.so path through the C ABI instead of
 * relying on LLAMA_CPP_DYLIB_PATH.
 *
 * Default builds return LOCALMT_FFI_RUNTIME_DISABLED. llama-runtime builds
 * return LOCALMT_FFI_RUNTIME_NOT_CONFIGURED when the path is missing, relative,
 * or not a file.
 */
int32_t localmt_ffi_llama_runtime_configure(
    const uint8_t *path_ptr,
    size_t path_len);

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
 * Opens a verified GGUF/llama.cpp model pack and constructs the llama-backed
 * translator.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * out_translator must point to writable pointer storage. It is set to NULL
 * before work and receives a non-null handle only on LOCALMT_FFI_OK.
 *
 * Current builds return LOCALMT_FFI_RUNTIME_DISABLED after pack planning until
 * the native llama.cpp loader is implemented.
 */
int32_t localmt_ffi_llama_translator_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiLlamaTranslator **out_translator);

/*
 * Releases a llama translator handle. NULL is accepted as a no-op.
 */
void localmt_ffi_llama_translator_close(LocalmtFfiLlamaTranslator *translator);

/*
 * Translates UTF-8 bytes through the llama-backed translator.
 *
 * input_ptr/input_len must be valid UTF-8 bytes. output_ptr/output_capacity is
 * caller-owned byte storage and is not NUL terminated by Rust. written_len must
 * point to writable size_t storage. On LOCALMT_FFI_BUFFER_TOO_SMALL,
 * written_len contains the required byte count and output is not written.
 */
int32_t localmt_ffi_llama_translate(
    const LocalmtFfiLlamaTranslator *translator,
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
 * ort-runtime builds return LOCALMT_FFI_RUNTIME_NOT_CONFIGURED when the ONNX
 * Runtime dylib path is missing or invalid, and map other ORT session-load
 * failures to LOCALMT_FFI_ORT_ERROR.
 */
int32_t localmt_ffi_ort_generator_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiOrtGenerator **out_generator);

/*
 * Releases an ORT generator preflight handle. NULL is accepted as a no-op.
 */
void localmt_ffi_ort_generator_close(LocalmtFfiOrtGenerator *generator);

/*
 * Opens a verified local model pack and constructs the ORT-backed translator.
 *
 * path_ptr/path_len must be valid UTF-8 bytes for the model-pack directory.
 * out_translator must point to writable pointer storage. It is set to NULL
 * before work and receives a non-null handle only on LOCALMT_FFI_OK.
 *
 * Default builds return LOCALMT_FFI_TOKENIZER_DISABLED after pack planning.
 * hf-tokenizers-only builds return LOCALMT_FFI_RUNTIME_DISABLED.
 * hf-tokenizers + ort-runtime builds map tokenizer load failures to
 * LOCALMT_FFI_TOKENIZER_ERROR, ORT dylib configuration failures to
 * LOCALMT_FFI_RUNTIME_NOT_CONFIGURED, and other ORT session/generation failures
 * to LOCALMT_FFI_ORT_ERROR or LOCALMT_FFI_TRANSLATION_ERROR.
 */
int32_t localmt_ffi_ort_translator_open(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiOrtTranslator **out_translator);

/*
 * Opens a trusted local model pack and constructs the ORT-backed translator.
 *
 * This uses the local trust artifact instead of full checksum verification on
 * the hot path. Regenerate the artifact when
 * localmt_ffi_model_pack_trust_schema_version() changes. The status mapping
 * and handle lifetime are identical to localmt_ffi_ort_translator_open.
 */
int32_t localmt_ffi_ort_translator_open_trusted(
    const uint8_t *path_ptr,
    size_t path_len,
    LocalmtFfiOrtTranslator **out_translator);

/*
 * Releases an ORT translator handle. NULL is accepted as a no-op.
 */
void localmt_ffi_ort_translator_close(LocalmtFfiOrtTranslator *translator);

/*
 * Translates UTF-8 bytes through the ORT-backed translator.
 *
 * input_ptr/input_len must be valid UTF-8 bytes. output_ptr/output_capacity is
 * caller-owned byte storage and is not NUL terminated by Rust. written_len must
 * point to writable size_t storage. On LOCALMT_FFI_BUFFER_TOO_SMALL,
 * written_len contains the required byte count and output is not written.
 */
int32_t localmt_ffi_ort_translate(
    const LocalmtFfiOrtTranslator *translator,
    uint8_t source_id,
    uint8_t target_id,
    const uint8_t *input_ptr,
    size_t input_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);

#ifdef __cplusplus
}
#endif

#endif
