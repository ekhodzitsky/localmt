# localmt

[![CI](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml/badge.svg)](https://github.com/ekhodzitsky/localmt/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

**Offline translation that fits in your pocket.**

`localmt` is a Rust library for running neural machine translation entirely on
device: no network, no cloud round-trips, no data leaving the phone. Built for
high-end Android devices first (Xiaomi 17 / `arm64-v8a`), designed to
generalise to any platform.

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
- **Verified model packs** - SHA-256 checksums, typed file roles, and trust
  artifacts so you know exactly what model is running.
- **Swappable backends** - ONNX Runtime Mobile today, Candle or custom engines
  tomorrow. Your app code stays the same.

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

A model pack is a directory with a `manifest.json` and the ONNX/graph files it
declares. You can generate a manifest for an existing directory:

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
localmt-ffi  ->  localmt-pipeline  ->  localmt-engine-ort
     |                 |                    |
     v                 v                    v
Android app  ->  localmt-tokenizer ->  localmt-models
```

- **Library-first** - every crate is a reusable building block. Apps and mobile
  shells are thin adapters around the Rust API.
- **Feature-gated backends** - default builds compile in milliseconds with mock
  engines; enable `hf-tokenizers` and `ort-runtime` for real inference.
- **Pointer-free C ABI** - null-safe FFI handles, explicit buffer contracts, and
  no undefined behaviour across the JNI boundary.

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

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
