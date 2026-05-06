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
still returns an explicit unavailable-backend error for real token generation.
If this feature is used in an Android app, the app build must also package the
matching ONNX Runtime Mobile library expected by the runtime loader.

## JNI Call Flow

At app startup:

1. Call `localmt_ffi_abi_version()` and require `LOCALMT_FFI_ABI_VERSION == 6`.
2. Call `localmt_ffi_xiaomi17_android_abi_code()` and require
   `LOCALMT_FFI_ANDROID_ABI_ARM64_V8A`.
3. Call `localmt_ffi_supported_language_count()` and map language ids through
   `localmt_ffi_language_code()`.

Before opening a translator:

1. Call `localmt_ffi_model_pack_summary()` with a local model-pack directory.
2. If the buffer is too small, allocate `written_len` bytes and call again.
3. Use `localmt_ffi_status_message()` to turn any non-zero status into a local
   log or UI diagnostic.

Smoke translation choices:

- Use `localmt_ffi_mock_translator_open()` and `localmt_ffi_mock_translate()` for
  pure deterministic adapter smoke tests.
- Use `localmt_ffi_hf_mock_translator_open()` and
  `localmt_ffi_hf_mock_translate()` when the Rust library is built with
  `hf-tokenizers`; this verifies a real tokenizer file while keeping generation
  mocked.

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

Use the development CLI to prepare and inspect packs locally:

```bash
cargo run -p localmt -- model hash <file>
cargo run -p localmt -- model write-manifest <pack-dir>
cargo run -p localmt -- model verify <pack-dir>
cargo run -p localmt -- model plan <pack-dir>
cargo run -p localmt -- ffi smoke <pack-dir> en ru "hello offline"
cargo run -p localmt --features hf-tokenizers -- ffi hf-smoke <pack-dir> en ru "hello offline"
```

`localmt ffi smoke` exercises the same default C ABI flow described above on the
host: ABI query, model-pack summary, mock translator open, mock translate,
status-message lookup on errors, and handle close.
`localmt ffi hf-smoke` exercises the tokenizer-backed mock translator path
through the same host-side FFI helpers; it requires `hf-tokenizers`, verifies
`tokenizer.json` loading, and still keeps token generation mocked.

## Verification

Host-side checks before handing the library to Android:

```bash
cargo fmt --check
cargo test -p localmt-ffi
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

The Android target build should be verified in CI or locally with the installed
NDK. This repository does not vendor Android SDK, NDK, ONNX Runtime binaries, or
model files.
