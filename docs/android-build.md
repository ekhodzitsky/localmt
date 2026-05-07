# Android FFI Build Notes

`localmt-ffi` is the Android/JNI boundary for the Rust library. The first target
device profile is Xiaomi 17-class Android on `arm64-v8a`.

## Build Outputs

The crate is configured as:

```toml
crate-type = ["cdylib", "staticlib", "rlib"]
```

Use the dynamic library for normal JNI packaging:

```bash
cargo build -p localmt-ffi --release
```

For Android, build the same crate for `aarch64-linux-android` with your local
NDK toolchain. If `cargo-ndk` is installed, the expected command shape is:

```bash
cargo ndk -t arm64-v8a -o target/android-jniLibs build -p localmt-ffi --release
```

This writes the JNI library under:

```text
target/android-jniLibs/arm64-v8a/
```

Copy that directory into the app module's `src/main/jniLibs/` or wire it into
the app build pipeline.
The C ABI header is checked in at
`crates/localmt-ffi/include/localmt_ffi.h`; the development CLI can also print
the same header with `cargo run -p localmt -- ffi header`.

## Feature Flags

Default builds are intentionally conservative:

```bash
cargo build -p localmt-ffi --release
```

Default behavior:

- model-pack summary works
- deterministic mock translation works
- status messages work
- HF tokenizer preflight returns `LOCALMT_FFI_TOKENIZER_DISABLED`
- ORT generator preflight returns `LOCALMT_FFI_RUNTIME_DISABLED`
- ORT translator open returns `LOCALMT_FFI_TOKENIZER_DISABLED`

Tokenizer smoke builds:

```bash
cargo build -p localmt-ffi --release --features hf-tokenizers
```

This enables real Hugging Face `tokenizer.json` loading and the
`LocalmtFfiHfMockTranslator` path. Token generation is still mock generation.

ORT runtime builds:

```bash
cargo build -p localmt-ffi --release --features ort-runtime
```

This enables ONNX Runtime session loading boundaries. The current generator
loads encoder/decoder sessions and runs the non-cached decoder loop when paired
with tokenizer output. If this feature is used in an Android app, the app build
must also package the matching ONNX Runtime Mobile library expected by the
runtime loader.

Full translator builds:

```bash
cargo build -p localmt-ffi --release --features "hf-tokenizers ort-runtime"
```

This enables `LocalmtFfiOrtTranslator`: model-pack verification, real
`tokenizer.json` loading, ORT encoder/decoder session loading, and translation
through `localmt_ffi_ort_translate`.

## JNI Call Flow

At app startup:

1. Call `localmt_ffi_startup_summary()` for a loggable startup contract, or
   call the individual probes below.
1. Call `localmt_ffi_abi_version()` and require `LOCALMT_FFI_ABI_VERSION == 11`.
1. Call `localmt_ffi_xiaomi17_android_abi_code()` and require
   `LOCALMT_FFI_ANDROID_ABI_ARM64_V8A`.
1. Call `localmt_ffi_supported_language_count()` and map language ids through
   `localmt_ffi_language_code()`.

Before opening a translator:

1. Call `localmt_ffi_model_pack_summary()` with a local model-pack directory.
2. If the buffer is too small, allocate `written_len` bytes and call again.
3. Call `localmt_ffi_runtime_config_summary()` before opening ORT generation;
   this requires valid `generation_config` and `ort_io` metadata without loading
   ONNX Runtime sessions.
4. Use `localmt_ffi_status_message()` to turn any non-zero status into a local
   log or UI diagnostic.

Smoke translation choices:

- Use `localmt_ffi_mock_translator_open()` and `localmt_ffi_mock_translate()` for
  pure deterministic adapter smoke tests.
- Use `localmt_ffi_hf_mock_translator_open()` and
  `localmt_ffi_hf_mock_translate()` when the Rust library is built with
  `hf-tokenizers`; this verifies a real tokenizer file while keeping generation
  mocked.
- Use `localmt_ffi_ort_generator_open()` when the Rust library is built with
  `ort-runtime`; this verifies ORT session loading without translation.
- Use `localmt_ffi_ort_translator_open()` and `localmt_ffi_ort_translate()`
  when the Rust library is built with `hf-tokenizers` and `ort-runtime`; this is
  the Android-visible real translation path.

For ORT-enabled builds, call `localmt_ffi_ort_runtime_configure()` with the
absolute `nativeLibraryDir/libonnxruntime.so` path before opening the generator
or translator. `ORT_DYLIB_PATH` remains a CLI/development fallback. Missing or
invalid paths return `LOCALMT_FFI_RUNTIME_NOT_CONFIGURED` instead of relying on
platform dynamic-library search.

The ORT translator flow is:

```c
LocalmtFfiOrtTranslator *translator = NULL;
int32_t status = localmt_ffi_ort_translator_open(
    path_ptr,
    path_len,
    &translator);
if (status == LOCALMT_FFI_OK) {
    size_t written_len = 0;
    status = localmt_ffi_ort_translate(
        translator,
        source_id,
        target_id,
        input_ptr,
        input_len,
        output_ptr,
        output_capacity,
        &written_len);
}
localmt_ffi_ort_translator_close(translator);
```

Every handle returned by an `open` function must be closed exactly once with the
matching `close` function. Passing a non-null pointer not created by Rust, or
closing the same handle twice, is invalid.

## Model Files

Model files are not committed to this repository. Android apps should store or
download model packs into app-local storage, then pass that directory to the FFI.
The Rust layer verifies `manifest.json` and SHA-256 checksums before exposing
paths to tokenizer or runtime adapters.

The initial standard pack layout is:

```text
manifest.json
encoder.onnx
decoder.onnx
tokenizer.json
generation.json          optional
decoder-with-past.onnx   optional
vocab.txt                optional
config.json              optional
```

For ONNX Runtime packs, `config.json` may include the local `ort_io` object that
names encoder and decoder tensors. This is metadata for the Rust ORT generation
boundary, not a network dependency:

```json
{
  "ort_io": {
    "encoder": {
      "input_ids": "input_ids",
      "attention_mask": "attention_mask",
      "last_hidden_state": "last_hidden_state"
    },
    "decoder": {
      "input_ids": "input_ids",
      "encoder_attention_mask": "encoder_attention_mask",
      "encoder_hidden_states": "encoder_hidden_states",
      "logits": "logits"
    }
  }
}
```

`localmt model plan` and `localmt_ffi_model_pack_summary` report this readiness
as `ort_io_config: parsed`, `ort_io_config: absent`, or
`ort_io_config: missing`. `missing` means a valid `config.json` exists but has
no `ort_io` object; malformed JSON or empty tensor names still fail preflight.

Use the development CLI to prepare and inspect packs locally:

```bash
cargo run -p localmt -- model hash <file>
cargo run -p localmt -- model write-manifest <pack-dir> m2m100-418m-int8 0.1.0 m2m100 onnx-runtime MIT
cargo run -p localmt -- model verify <pack-dir>
cargo run -p localmt -- model plan <pack-dir>
cargo run -p localmt -- model doctor <pack-dir>
cargo run -p localmt -- model runtime-config <pack-dir>
cargo run -p localmt -- ffi startup
cargo run -p localmt -- ffi header
cargo run -p localmt -- ffi runtime-config <pack-dir>
cargo run -p localmt -- ffi smoke <pack-dir> en ru "hello offline"
cargo run -p localmt --features hf-tokenizers -- ffi hf-smoke <pack-dir> en ru "hello offline"
cargo run -p localmt --features ort-runtime -- ffi ort-smoke <pack-dir>
cargo run -p localmt --features "hf-tokenizers ort-runtime" -- ffi ort-translate-smoke <pack-dir> en ru "hello offline"
```

`localmt ffi smoke` exercises the same default C ABI flow described above on the
host: ABI query, model-pack summary, mock translator open, mock translate,
status-message lookup on errors, and handle close.
`localmt model doctor` is the combined preflight gate before handing a pack to
Android: facade planning, startup ABI summary, shared FFI model-pack summary,
deterministic mock FFI translation, HF tokenizer status, ORT runtime status, and
ORT translator FFI status. Default builds report disabled HF/ORT features as
diagnostics, while feature builds use the compiled backends.
`localmt model runtime-config` is stricter than `model plan`: it requires both
`generation_config` and `ort_io` to be present and valid, but still avoids
loading ONNX Runtime sessions.
`localmt ffi runtime-config` exercises the same strict readiness gate through
`localmt_ffi_runtime_config_summary`, which is the Android/JNI-facing ABI call.
`localmt ffi hf-smoke` exercises the tokenizer-backed mock translator path
through the same host-side FFI helpers; it requires `hf-tokenizers`, verifies
`tokenizer.json` loading, and still keeps token generation mocked.
`localmt ffi ort-smoke` exercises the ORT generator preflight handle; it
requires `ort-runtime`, attempts session loading, and does not run translation.
`localmt ffi ort-translate-smoke` exercises the full ORT translator FFI handle;
it requires `hf-tokenizers` plus `ort-runtime` for real translation.

## Verification

Host-side checks before handing the library to Android:

```bash
cargo fmt --check
cargo test -p localmt-ffi
cargo check -p localmt-ffi --target aarch64-linux-android
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

The Android target build should be verified in CI or locally with the installed
NDK. This repository does not vendor Android SDK, NDK, ONNX Runtime binaries, or
model files.
