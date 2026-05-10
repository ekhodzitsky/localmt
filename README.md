# localmt

[![CI](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml/badge.svg)](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

**Offline translation that fits in your pocket.**

`localmt` is a Rust library for running neural machine translation entirely on
device: no network, no cloud round-trips, no data leaving the phone. Built for
Android `arm64-v8a` devices first, with Xiaomi 17-class phones as the
performance target and Redmi Note 14 as the first real-device smoke target.

The production model strategy is now Hy-MT1.5-first: use Tencent Hunyuan's
Hy-MT1.5-1.8B-1.25bit GGUF model as the first real mobile translation target,
run it through a llama.cpp/STQ1_0 backend, and keep ONNX Runtime as an
experimental compatibility backend.

```bash
# Translate offline in one command
cargo run -p localmt-cli --features "hf-tokenizers ort-runtime" -- \
  ffi ort-translate-smoke ./models/my-pack en ru "hello world"
```

---

## Why localmt?

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
cargo install --path crates/localmt-cli

# Build the CLI with real ONNX Runtime translation support.
cargo install --path crates/localmt-cli --features "hf-tokenizers ort-runtime" --force
```

### 2. Grab or build a model pack

A model pack is a directory with a `manifest.json` and the local model/runtime
files it declares. The next production pack format will support Hy-MT GGUF
assets; the current implemented pack flow supports ONNX/graph roles.

You can generate a manifest for an existing ONNX-style directory:

```bash
localmt model write-manifest ./models/m2m100-418m-int8 \
  m2m100-418m-int8 0.1.0 m2m100 onnx-runtime MIT
```

### 3. Verify and smoke-test

```bash
# Verify integrity
localmt model verify ./models/m2m100-418m-int8

# Full health check (works without heavy runtime dependencies)
localmt model doctor ./models/m2m100-418m-int8

# Offline translation with ONNX Runtime (requires `ort-runtime` + `hf-tokenizers`)
export ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib
localmt ffi ort-translate-smoke ./models/m2m100-418m-int8 en ja "hello"
```

The GGUF/Hy-MT FFI readiness gate is:

```bash
localmt ffi gguf-translate-smoke ./models/hymt-1.25bit en ru "hello world"
```

Today it verifies the Hy-MT GGUF pack, validates llama runtime metadata,
constructs the Rust-owned prompt, and reports `runtime disabled` until model
and context creation are wired. `llama-runtime` builds can also accept an
explicit dynamic-library path through `localmt_ffi_llama_runtime_configure` or
`LLAMA_CPP_DYLIB_PATH`; the native loader opens that library and requires the
minimal llama.cpp model/context C API symbols before the path is accepted. The
lower-level GGUF model-pack doctor is:

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
  formatting, GGUF asset planning, and runtime config before native llama.cpp
  loading is wired in.
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
