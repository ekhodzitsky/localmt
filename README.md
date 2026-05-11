# LocalMT

[![CI](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml/badge.svg)](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](Cargo.toml)
[![Android arm64-v8a](https://img.shields.io/badge/android-arm64--v8a-brightgreen.svg)](docs/android-build.md)
[![FFI ABI 14](https://img.shields.io/badge/FFI_ABI-14-blue.svg)](crates/localmt-ffi/include/localmt_ffi.h)
[![Runtime: GGUF/llama.cpp](https://img.shields.io/badge/runtime-GGUF%2Fllama.cpp-blueviolet.svg)](ROADMAP.md)

**Offline Android translation SDK in Rust.**

LocalMT runs verified translation model packs on-device through a small
privacy-first Rust + FFI/JNI surface: no network, no API keys, no cloud
round-trips, and no text leaving the phone.

The project is Android `arm64-v8a` first. Xiaomi 17-class phones are the
performance target, Redmi Note 14 is the first real-device smoke target, and
Tencent Hunyuan Hy-MT1.5 GGUF is the production model direction. ONNX Runtime
stays available as an experimental compatibility backend.

```bash
# Offline GGUF/llama.cpp smoke path
export LLAMA_CPP_DYLIB_PATH=/absolute/path/to/libllama.dylib
cargo run -p localmt-cli --features llama-runtime -- \
  ffi gguf-translate-smoke ./models/hymt-gguf en ru \
  "Where is the nearest train station?"
```

Host proof already returned a real Russian translation with Hy-MT Q4_K_M GGUF:

```text
translation: Где находится ближайшая железнодорожная станция?
```

---

## Status

| Capability | Status |
|------------|--------|
| Rust SDK, model-pack verification, checksums, trust artifacts | Implemented |
| C ABI / JNI surface for Android `arm64-v8a` | Implemented and CI checked |
| GGUF/llama.cpp host translation | Proven with Hy-MT Q4_K_M GGUF |
| Hy-MT1.5-1.8B-1.25bit mobile target | Waiting on stable upstream STQ1_0 / Sherry support |
| Redmi Note 14 airplane-mode translation smoke | Next device proof |
| ONNX Runtime translation path | Experimental compatibility backend |

## Why LocalMT?

- **Privacy-first** - text never leaves the device. No API keys, no telemetry.
- **Offline by default** - works in airplane mode, in remote areas, behind
  corporate firewalls.
- **Hy-MT ready** - the first production path targets Hy-MT1.5-1.8B-1.25bit
  GGUF, a 440 MB on-device translation model that covers the starting language
  set.
- **Verified model packs** - SHA-256 checksums, typed file roles, and trust
  artifacts so you know exactly what model is running.
- **Swappable backends** - llama.cpp/GGUF first, ONNX Runtime experimental, and
  future engines behind the same Rust traits. Your app code stays the same.

## Supported languages

| Language | Code |
|----------|------|
| English  | `en` |
| Japanese | `ja` |
| Russian  | `ru` |
| Thai     | `th` |
| Vietnamese | `vi` |

More languages are on the roadmap; the architecture is model-agnostic.

## Quick start

### 1. Install the CLI

```bash
# Fast model-pack and ABI checks.
cargo install --path crates/localmt-cli

# GGUF/llama.cpp translation smoke support.
cargo install --path crates/localmt-cli --features llama-runtime --force
```

### 2. Prepare a GGUF model pack

A model pack is a directory with a `manifest.json` and the local model/runtime
files it declares. Model weights are never committed to this repository.

For Hy-MT/GGUF, start from the example pack shape:

```bash
mkdir -p ./models/hymt-gguf
cp examples/model-packs/hymt-1.25bit/manifest.example.json \
  ./models/hymt-gguf/manifest.json
cp examples/model-packs/hymt-1.25bit/llama-runtime.example.json \
  ./models/hymt-gguf/llama-runtime.json
# After placing your local GGUF file:
localmt model hash ./models/hymt-gguf/hymt.gguf
```

Then replace the example SHA-256 values in `manifest.json` with hashes from
your local files. For ONNX-style compatibility packs, the CLI can generate a
manifest for an existing directory:

```bash
localmt model write-manifest ./models/m2m100-418m-int8 \
  m2m100-418m-int8 0.1.0 m2m100 onnx-runtime MIT
```

### 3. Verify and smoke-test

```bash
# Verify integrity
localmt model verify ./models/hymt-gguf

# Full health check (works without heavy runtime dependencies)
localmt model doctor ./models/hymt-gguf

# Offline GGUF translation with llama.cpp (requires `llama-runtime`)
export LLAMA_CPP_DYLIB_PATH=/absolute/path/to/libllama.dylib
localmt ffi gguf-translate-smoke ./models/hymt-gguf en ru "hello"
```

The GGUF/Hy-MT FFI readiness gate is:

```bash
localmt ffi gguf-translate-smoke ./models/hymt-1.25bit en ru "hello world"
```

Default builds report `runtime disabled`. `llama-runtime` builds can accept an
explicit dynamic-library path through `localmt_ffi_llama_runtime_configure` or
`LLAMA_CPP_DYLIB_PATH`; the native loader opens that library, requires the
llama.cpp model/context/tokenizer/sampler C API symbols, opens the GGUF
model/context, and runs bounded tokenization, eval, sampling, and UTF-8 decode.

The 1.25-bit/STQ package remains the production size target, but stable support
depends on upstream llama.cpp STQ kernel availability. The lower-level GGUF
model-pack doctor is:

```bash
localmt model doctor ./models/hymt-1.25bit
```

See [`examples/model-packs/hymt-1.25bit`](examples/model-packs/hymt-1.25bit)
for the expected manifest shape.

### 4. Embed in Android

Build the FFI crate for `arm64-v8a` and link it in your NDK project:

```bash
cargo ndk -t arm64-v8a -o target/android-jniLibs build -p localmt-ffi --release
```

The C header is at `crates/localmt-ffi/include/localmt_ffi.h`. See
[`docs/android-build.md`](docs/android-build.md) for the full JNI flow.

## Architecture

```text
localmt-cli  ->  localmt facade  ->  localmt-engine
     |                 |                    |
     v                 v                    v
localmt-ffi  ->  localmt-pipeline  ->  localmt-engine-llama
     |                 |                    |
     v                 v                    v
Android app  ->  localmt-models   ->  llama.cpp / GGUF
```

- **Library-first** - every crate is a reusable building block. Apps and mobile
  shells are thin adapters around the Rust API.
- **Feature-gated backends** - default builds compile in milliseconds with mock
  engines; enable backend features for real inference.
- **Rust-owned llama boundary** - `localmt-engine-llama` owns Hy-MT prompt
  formatting, GGUF asset planning, runtime config, and the narrow dynamic
  loading boundary to llama.cpp.
- **Pointer-free C ABI** - null-safe FFI handles, explicit buffer contracts, and
  no undefined behaviour across the JNI boundary.

## Production Model Target

The first production target is Hy-MT1.5-1.8B-1.25bit GGUF:

- 440 MB compressed translation model package.
- 33 languages and 1,056 translation directions reported by Tencent Hunyuan.
- Covers the initial `localmt` languages: English, Russian, Thai, Vietnamese,
  and Japanese.
- Intended for on-device offline mobile translation.
- Requires llama.cpp support for STQ1_0 / Sherry quantization before it can be
  treated as stable in this project.

Model assets stay outside the repository. `localmt` should verify and load
user-supplied model packs; it must not commit or redistribute model weights.

References:

- https://huggingface.co/tencent/Hy-MT1.5-1.8B-1.25bit-GGUF
- https://huggingface.co/tencent/HY-MT1.5-1.8B
- https://github.com/ggml-org/llama.cpp/pull/22836

## Platform support

| Platform | Target | Status |
|----------|--------|--------|
| Android arm64 | `aarch64-linux-android` | Primary |
| macOS host | `aarch64-apple-darwin` | Dev / CI |
| Linux host | `x86_64-unknown-linux-gnu` | Dev / CI |

## Documentation

| Doc | What it covers |
|-----|---------------|
| [`docs/API.md`](docs/API.md) | Internal architecture, crate boundaries, FFI contract, model-pack format, generation config |
| [`docs/android-build.md`](docs/android-build.md) | Android/JNI build notes, feature flags, and FFI call flow |
| [`docs/performance.md`](docs/performance.md) | Real-model latency baselines and bottleneck notes |
| [`ROADMAP.md`](ROADMAP.md) | Hy-MT/GGUF product and engineering roadmap |
| [`TODO.md`](TODO.md) | Current implementation checklist |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
