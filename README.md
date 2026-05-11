# LocalMT

[![CI](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml/badge.svg)](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust 1.95+](https://img.shields.io/badge/rust-1.95%2B-orange.svg)](Cargo.toml)
[![Android arm64-v8a](https://img.shields.io/badge/android-arm64--v8a-brightgreen.svg)](docs/android-build.md)
[![FFI ABI 14](https://img.shields.io/badge/FFI_ABI-14-blue.svg)](crates/localmt-ffi/include/localmt_ffi.h)
[![Runtime: GGUF/llama.cpp](https://img.shields.io/badge/runtime-GGUF%2Fllama.cpp-blueviolet.svg)](ROADMAP.md)

**Offline Android translation SDK in Rust.**

LocalMT runs verified translation model packs on-device through a small
privacy-first Rust + C ABI/JNI surface. There are no API keys, no cloud
round-trips, no telemetry, and no text leaving the phone.

The main path is Android `arm64-v8a` + GGUF + llama.cpp. Xiaomi 17-class phones
are the performance target, Redmi Note 14 is the first real-device smoke target,
and Tencent Hunyuan Hy-MT1.5 is the model family the SDK is being hardened
around.

```bash
export LLAMA_CPP_DYLIB_PATH=/absolute/path/to/libllama.dylib
cargo run -p localmt-cli --features llama-runtime -- \
  ffi gguf-translate-smoke ./models/hymt-gguf en ru \
  "Where is the nearest train station?"
```

Host proof already returned real Russian output with Hy-MT Q4_K_M GGUF:

```text
translation: Где находится ближайшая железнодорожная станция?
```

## What Works Today

| Capability | Status |
|------------|--------|
| Rust SDK, model-pack parsing, SHA-256 verification, trust artifacts | Implemented |
| Android-facing C ABI and JNI smoke surface | Implemented and CI checked |
| GGUF/llama.cpp host translation | Proven with Hy-MT Q4_K_M GGUF |
| Hy-MT1.5-1.8B-1.25bit mobile target | Waiting on stable upstream STQ1_0 / Sherry support |
| Redmi Note 14 airplane-mode translation smoke | Next device proof |
| ONNX Runtime translation path | Experimental compatibility backend |

LocalMT is a library/SDK, not a finished translator app. Model weights are not
committed or redistributed by this repository; applications provide local model
packs and accept the upstream model terms themselves.

## Why LocalMT?

- **Private by construction** - translation runs locally; user text stays on
  the device.
- **Offline by default** - useful in airplane mode, field work, travel, or
  locked-down environments.
- **Android-first** - the ABI, packaging scripts, and JNI smoke surface target
  `arm64-v8a` rather than desktop-only demos.
- **Verified model packs** - manifests use typed file roles and SHA-256 checks
  before runtime loading.
- **Backend boundary kept small** - GGUF/llama.cpp is the production direction;
  ONNX Runtime remains behind the same SDK shape for compatibility experiments.

## Supported Languages

| Language | Code |
|----------|------|
| English | `en` |
| Russian | `ru` |
| Thai | `th` |
| Vietnamese | `vi` |
| Japanese | `ja` |

The architecture is model-agnostic, but these are the first language IDs kept
stable across the Rust API, C ABI, and JNI sample.

## Quick Start

### 1. Install the CLI

```bash
cargo install --path crates/localmt-cli --features llama-runtime --force
```

Use a default build when you only need model-pack checks and ABI inspection:

```bash
cargo install --path crates/localmt-cli --force
```

### 2. Prepare a GGUF Model Pack

Create a local pack from the checked-in example. The example manifest contains
placeholder hashes; replace them with hashes from your local files.

```bash
mkdir -p ./models/hymt-gguf
cp examples/model-packs/hymt-1.25bit/manifest.example.json \
  ./models/hymt-gguf/manifest.json
cp examples/model-packs/hymt-1.25bit/llama-runtime.example.json \
  ./models/hymt-gguf/llama-runtime.json
```

Then place the model assets beside the manifest:

```text
models/hymt-gguf/
  manifest.json
  hymt.gguf
  chat-template.jinja
  llama-runtime.json
```

Compute hashes and update `manifest.json`:

```bash
localmt model hash ./models/hymt-gguf/hymt.gguf
localmt model hash ./models/hymt-gguf/chat-template.jinja
localmt model hash ./models/hymt-gguf/llama-runtime.json
```

### 3. Verify and Translate

```bash
localmt model verify ./models/hymt-gguf
localmt model doctor ./models/hymt-gguf

export LLAMA_CPP_DYLIB_PATH=/absolute/path/to/libllama.dylib
localmt ffi gguf-translate-smoke ./models/hymt-gguf en ru "hello world"
```

Default builds report `runtime disabled` for translation. Builds with
`llama-runtime` load the configured llama.cpp dynamic library, open the verified
GGUF model/context, and run bounded tokenization, eval, sampling, UTF-8 decode,
and C ABI buffer handling.

The 1.25-bit Hy-MT package is the production size target, but LocalMT does not
claim it stable until llama.cpp has stable STQ1_0 / Sherry support and a real
Android smoke passes.

## Android Integration

Build the Rust FFI artifact for Android:

```bash
cargo ndk -t arm64-v8a -o target/android-jniLibs build \
  -p localmt-ffi --release --features llama-runtime
```

Or use the repository packaging script, which builds the JNI-ready artifact and
checks the exported C ABI symbols:

```bash
scripts/package-android-ffi.sh
```

Before a full app shell exists, stage the native libraries and model pack on a
connected `arm64-v8a` device:

```bash
scripts/android-device-preflight.sh \
  --llama-runtime /path/to/libllama.so \
  --model-pack ./models/hymt-gguf
```

The C header is at
[`crates/localmt-ffi/include/localmt_ffi.h`](crates/localmt-ffi/include/localmt_ffi.h).
The JNI smoke adapter is under
[`examples/android-jni-smoke`](examples/android-jni-smoke).

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

- `localmt` is the public Rust facade.
- `localmt-models` verifies local model packs and trust artifacts.
- `localmt-engine-llama` owns GGUF planning, Hy-MT prompt formatting, runtime
  config, and the narrow dynamic loading boundary to llama.cpp.
- `localmt-ffi` exposes pointer-safe C ABI handles for Android/JNI callers.
- `localmt-cli` dogfoods the same ABI paths used by Android adapters.

## Compatibility Backend

ONNX Runtime support remains available for compatibility experiments and
regression baselines:

```bash
export ORT_DYLIB_PATH=/absolute/path/to/libonnxruntime.dylib
cargo run -p localmt-cli --features "hf-tokenizers ort-runtime" -- \
  ffi ort-translate-smoke ./models/m2m100-418m-int8 en ru "hello"
```

The GGUF/llama.cpp path is the default product direction. Treat ORT as
experimental unless a model pack has comparable quality and mobile performance
evidence.

## Documentation

| Doc | What it covers |
|-----|---------------|
| [`docs/API.md`](docs/API.md) | Crate boundaries, FFI contract, model-pack format, generation config |
| [`docs/android-build.md`](docs/android-build.md) | Android/JNI build notes, feature flags, FFI call flow |
| [`docs/performance.md`](docs/performance.md) | Real-model latency baselines and bottleneck notes |
| [`ROADMAP.md`](ROADMAP.md) | Hy-MT/GGUF product and engineering roadmap |
| [`TODO.md`](TODO.md) | Current implementation checklist |

## References

- https://huggingface.co/tencent/Hy-MT1.5-1.8B-1.25bit-GGUF
- https://huggingface.co/tencent/HY-MT1.5-1.8B
- https://github.com/ggml-org/llama.cpp/pull/22836

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
