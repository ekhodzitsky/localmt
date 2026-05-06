# localmt

Offline-first Rust translation library for high-end mobile devices.

The first target profile is Xiaomi 17 on Android arm64-v8a. The library keeps
translation API, language routing, and model-pack validation independent from
the concrete inference backend. ONNX Runtime Mobile is the first intended real
backend; the current workspace has a verified model-pack layer plus a
pipeline-backed mock translation path so API, asset-loading, and text/token
contracts can be tested before real inference.

## Workspace

- `crates/localmt-core` - language, text, request, and invariant types
- `crates/localmt-engine` - engine trait and mock engine
- `crates/localmt-engine-ort` - ONNX Runtime session planning and gated loading
- `crates/localmt-models` - model-pack manifest parsing and checksum verification
- `crates/localmt-pipeline` - tokenizer/generator translation pipeline skeleton
- `crates/localmt-tokenizer` - tokenizer trait, token invariants, and mock tokenizer
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
than once. After verification, `ModelPack<Verified>::file_path(role)` resolves a
declared role to its model-pack-root-qualified file path; missing optional roles
return `None`.

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
localmt model plan ./models/m2m100-418m-int8
```

`model plan` verifies the pack, builds the facade-level
`OfflineTranslatorPlan`, and parses an optional `generation_config`. It is a
no-inference smoke command: it does not load ONNX Runtime sessions or execute
decoder graphs.

## Facade Planning

`OfflineTranslatorPlan::from_pack(&verified_pack)` is the SDK-level bridge from
a verified model pack to the assets needed by a future offline translator. It
combines `TokenizerAssetPlan` and `OrtGeneratorPlan` so application adapters do
not need to manually wire tokenizer, encoder, decoder, cached decoder, and
generation-config paths.

This object is still a load plan. It does not parse tokenizer files, load ONNX
Runtime sessions, or run translation.

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

`OrtGeneratorPlan` builds on the pipeline-owned `GeneratorAssetPlan` and turns
verified encoder/decoder assets into ORT session plans. `OrtTokenGenerator`
loads the required ORT sessions only when `ort-runtime` is enabled; the default
build returns `OrtRuntimeFeatureDisabled`. `OrtTokenGenerator` now satisfies the
pipeline `TokenGenerator` trait, but `generate` returns
`BackendUnavailable("ONNX token generation loop is not implemented")` until
encoder/decoder tensor I/O is implemented. Real tokenization, decoder graph
execution, and translation are intentionally future work.

`OrtGeneratorPlan::parse_generation_config` parses the optional verified
`generation_config` asset into `GenerationConfig`. Packs without that role
return `Ok(None)`. Session loading and decoder execution still do not consume
the config yet.

## Tokenizer Boundary

`localmt-tokenizer` defines the tokenizer-side API before a real SentencePiece
or BPE implementation is selected. It owns `TokenId`, non-empty bounded
`TokenSequence`, `TokenizerInput`, `TokenizerOutput`, and the `TokenizerEngine`
trait. `MockTokenizer` performs deterministic UTF-8 byte roundtrips so the
future translation pipeline can be tested without model-specific tokenizer
dependencies. `TokenizerAssetPlan` builds from a verified model pack, requires a
declared `tokenizer` role, and carries optional `vocab` and `config` paths for
future tokenizer implementations.

## Pipeline Skeleton

`localmt-pipeline` composes a `TokenizerEngine` with a `TokenGenerator` and
implements the existing `TranslatorEngine` trait. `MockTokenGenerator` echoes
source tokens so the SDK can exercise the end-to-end request shape today.
`GeneratorAssetPlan` builds from a verified model pack, requires `encoder` and
`decoder` graph roles, and carries optional `decoder_with_past` and
`generation_config` paths for future generator implementations:

```text
TranslateRequest -> TokenizerEngine::encode -> TokenGenerator::generate -> TokenizerEngine::decode -> Translation
```

Real translation still requires replacing `MockTokenGenerator` with an
ONNX-backed generator and replacing `MockTokenizer` with a model-specific
tokenizer.

## Generation Config

`GenerationConfig` captures decoder-loop settings as typed values before the
loop exists. It owns a bounded `MaxNewTokens`, required BOS/EOS token ids, and
target-language token ids for `en`, `ru`, `th`, `vi`, and `ja`.
`GenerationConfig::with_default_limit` uses `DEFAULT_MAX_NEW_TOKENS` and rejects
duplicate or colliding token roles. Parsing `generation_config` files and using
these values inside ONNX decoder execution are separate steps.

Accepted local `generation_config` JSON shape:

```json
{
  "max_new_tokens": 64,
  "bos_token_id": 0,
  "eos_token_id": 1,
  "language_token_ids": {
    "en": 10,
    "ru": 11,
    "th": 12,
    "vi": 13,
    "ja": 14
  }
}
```

`max_new_tokens` is optional; omitted configs use `DEFAULT_MAX_NEW_TOKENS`.
`GenerationConfig::from_json_file` and `GenerationConfig::from_json_str` parse
this schema and then run the same token-role validation. Decoder execution still
does not consume this config yet.

## Benchmark Skeleton

The current benchmark command verifies a model pack, then runs 10 fixed language
pair scenarios through `TranslationPipeline<MockTokenizer, MockTokenGenerator>`.
It is intentionally labeled `runtime: mock-pipeline`; real ONNX latency and
memory metrics will be added with an ONNX-backed `TokenGenerator`.

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
