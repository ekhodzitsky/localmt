# localmt Library Design

## Goal

Build a Rust library for offline machine translation on Xiaomi 17-class Android
devices. The first implementation target is library correctness and API shape;
real ONNX Runtime inference enters after the API and benchmark harness are
stable.

## Product Contract

`localmt` is a library, not an app. Mobile apps, JNI, UniFFI, and CLIs are
adapters around the library. The crate must never require internet access at
runtime. Model assets are supplied as local model packs and are not embedded in
the Rust crates.

## Initial Device Profile

- Device: Xiaomi 17
- OS: Android 16 / HyperOS 3
- Architecture: arm64-v8a
- RAM class: 12GB
- Preferred runtime: ONNX Runtime Mobile with XNNPACK CPU execution
- Experimental runtime: NNAPI

## Languages

The first supported public languages are English, Russian, Thai, Vietnamese,
and Japanese. Public APIs expose a `Language` enum, not raw strings. Backend
adapters map those variants to model-specific language codes.

## Architecture

The workspace is split into narrow crates:

- `localmt-core`: invariant-bearing types such as `Language`, `LanguagePair`,
  `NonEmptyText`, `TranslateRequest`, and `Translation`.
- `localmt-engine`: `TranslatorEngine` trait plus a deterministic mock engine.
- `localmt`: facade crate that re-exports stable public API and owns
  `Translator<E>`.
- `localmt-cli`: development-only smoke CLI.

The later ONNX Runtime crate will be added as `localmt-engine-ort`, behind a
feature flag. It must not leak `ort` types into `localmt-core` or `localmt`.

## Model Strategy

The first real benchmark candidate is M2M100 418M INT8/ORT because it supports
the starting language set and has a permissive model license. NLLB-200
distilled 600M can be supported as a user-supplied optional model pack, but it
must carry a license warning and must not become the default bundled option.

## Acceptance Criteria For The First Slice

- A new git repository exists at `/Users/ekhodzitsky/Documents/personal/localmt`.
- The Rust workspace builds without external dependencies.
- Core invalid states are rejected through constructors:
  - empty text
  - same-language translation pairs
  - unsupported language codes
- The mock translator returns deterministic non-empty output.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features
  -- -D warnings`, and `cargo doc --no-deps` pass.
