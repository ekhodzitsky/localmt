# localmt

Offline-first Rust translation library for high-end mobile devices.

The first target profile is Xiaomi 17 on Android arm64-v8a. The library keeps
translation API, language routing, and model-pack validation independent from
the concrete inference backend. ONNX Runtime Mobile is the first intended real
backend; the current workspace has a mock engine plus a verified model-pack
layer so API and asset-loading contracts can be tested before real inference.

## Workspace

- `crates/localmt-core` - language, text, request, and invariant types
- `crates/localmt-engine` - engine trait and mock engine
- `crates/localmt-engine-ort` - ONNX Runtime session planning and gated loading
- `crates/localmt-models` - model-pack manifest parsing and checksum verification
- `crates/localmt-bench` - benchmark profiles and mock benchmark skeleton
- `crates/localmt` - public facade crate
- `crates/localmt-cli` - development CLI for smoke testing

## Model Packs

Model packs are local directories with a `manifest.json` file and the files it
declares. Verification checks that paths cannot escape the model-pack root, all
declared files exist, SHA-256 digests match, and the first supported languages
are present: `en`, `ru`, `th`, `vi`, and `ja`.

Each file `kind` is parsed as a typed role, not a free-form string. Supported
roles are `encoder`, `decoder`, `decoder_with_past`, `tokenizer`, `vocab`,
`config`, and `generation_config`. A manifest cannot declare the same role more
than once.

Minimal manifest shape:

```json
{
  "schema_version": 0,
  "model_id": "m2m100-418m-int8",
  "version": "0.1.0",
  "architecture": "m2m100",
  "runtime": "onnx-runtime",
  "license": "MIT",
  "languages": ["en", "ru", "th", "vi", "ja"],
  "files": [
    {
      "path": "encoder.onnx",
      "kind": "encoder",
      "sha256": "b1c4c05f286afb2531d4c847c4ca1e56260fc61281b7a04d50e09d09ab7a682b"
    }
  ]
}
```

Development CLI:

```bash
localmt model inspect ./models/m2m100-418m-int8
localmt model verify ./models/m2m100-418m-int8
```

## ONNX Runtime Boundary

`localmt-engine-ort` selects ONNX graph files from a verified model pack before
any runtime session is created. In the default build, `OrtEngine::load` returns
`OrtRuntimeFeatureDisabled`, so normal workspace checks do not load or link ONNX
Runtime accidentally. Real session creation is available behind the
`ort-runtime` feature:

```bash
cargo check -p localmt-engine-ort --features ort-runtime
cargo check -p localmt --features ort-runtime
```

This crate currently stops at session loading. Tokenization, decoder graph
composition, and actual translation are intentionally still future work.

## Benchmark Skeleton

The current benchmark command verifies a model pack, then runs 10 fixed language
pair scenarios through the mock engine. It is intentionally labeled
`runtime: mock`; real ONNX latency and memory metrics will be added with the
`localmt-engine-ort` backend.

```bash
localmt bench --profile xiaomi17 --model-pack ./models/m2m100-418m-int8
```

## Verify

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```
