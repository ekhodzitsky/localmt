# localmt

Offline-first Rust translation library for high-end mobile devices.

The first target profile is Xiaomi 17 on Android arm64-v8a. The library keeps
translation API, language routing, and model-pack validation independent from
the concrete inference backend. ONNX Runtime Mobile is the first intended real
backend; the current workspace has a verified model-pack layer, a feature-gated
Hugging Face tokenizer JSON loader, and a pipeline-backed mock translation path
so API, asset-loading, and text/token contracts can be tested before real
inference.

## Workspace

- `crates/localmt-core` - language, text, request, and invariant types
- `crates/localmt-engine` - engine trait and mock engine
- `crates/localmt-engine-ort` - ONNX Runtime session planning and gated loading
- `crates/localmt-models` - model-pack manifest parsing and checksum verification
- `crates/localmt-pipeline` - tokenizer/generator translation pipeline skeleton
- `crates/localmt-tokenizer` - tokenizer trait, token invariants, mock tokenizer, and gated HF tokenizer
- `crates/localmt-bench` - benchmark profiles and mock benchmark skeleton
- `crates/localmt` - public facade crate
- `crates/localmt-cli` - development CLI for smoke testing
- `crates/localmt-ffi` - pointer-free C ABI for Android/JNI adapters

## Docs

- `docs/android-build.md` - Android/JNI build notes, feature flags, and FFI call flow

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
localmt --help
localmt model help
localmt model write-manifest ./models/m2m100-418m-int8 m2m100-418m-int8 0.1.0 m2m100 onnx-runtime MIT
localmt model hash ./models/m2m100-418m-int8/encoder.onnx
localmt model inspect ./models/m2m100-418m-int8
localmt model verify ./models/m2m100-418m-int8
localmt model plan ./models/m2m100-418m-int8
localmt model doctor ./models/m2m100-418m-int8
localmt model tokenize ./models/m2m100-418m-int8 en ru "hello offline"
localmt ffi startup
localmt ffi header
localmt bench --help
```

`model hash` computes the lowercase SHA-256 digest used in `manifest.json`.
SDK and tooling code can also build manifests without hand-written JSON:
`ModelManifest::new_current` accepts validated model metadata, supported
languages, safe relative file paths, typed file roles, and SHA-256 values, then
`to_json_string_pretty` serializes the current schema for `manifest.json`.
`model write-manifest` is the CLI path for standard local pack layouts: it
requires `encoder.onnx`, `decoder.onnx`, and `tokenizer.json`, includes optional
`decoder-with-past.onnx`, `vocab.txt`, `config.json`, and `generation.json`
when present, computes SHA-256 values, and prints JSON to stdout.
`model plan` verifies the pack, builds the facade-level
`OfflineTranslatorPlan`, and parses an optional `generation_config`. It is a
no-inference smoke command: it does not load ONNX Runtime sessions or execute
decoder graphs.
`model doctor` is the single local readiness gate for a pack: it runs facade
planning, the shared FFI model-pack summary, deterministic mock FFI translation,
and HF/ORT preflight status checks. In default builds, disabled tokenizer and
runtime features are reported as diagnostic lines instead of command failures.

## Facade Planning

`OfflineTranslatorPlan::from_pack(&verified_pack)` is the SDK-level bridge from
a verified model pack to the assets needed by a future offline translator. It
combines `TokenizerAssetPlan` and `OrtGeneratorPlan` so application adapters do
not need to manually wire tokenizer, encoder, decoder, cached decoder, and
generation-config paths.

This object is still a load plan. It does not parse tokenizer files, load ONNX
Runtime sessions, or run translation.
`OfflineTranslatorPlan::parse_generation_config` exposes facade-level parsing
for the optional generation config while still avoiding tokenizer parsing,
session loading, and decoder execution.
`OfflineTranslatorAssets::from_pack` is the next no-inference preparation layer:
it owns the verified plan plus parsed optional generation config for CLI and
future mobile adapters.
`OfflineTranslatorAssets::from_model_pack_path` is the SDK-level convenience
entry point for a local model-pack directory: it discovers, verifies, plans, and
parses generation config without loading runtime resources.
`OfflineTranslatorAssets::summary` returns a structured preflight summary with
the verified model id, tokenizer/generator paths, optional cached decoder, and
optional generation config.
`MockOfflineTranslator::from_model_pack_path` is the facade-level integration
translator for app/adapter work before real inference: it requires the same
prepared assets, then delegates translation to the deterministic mock
tokenizer/generator pipeline.
`HfMockOfflineTranslator` is available behind `hf-tokenizers` for a stronger
tokenizer integration smoke path: it loads the verified Hugging Face
`tokenizer.json` and still uses `MockTokenGenerator`, so it proves tokenizer
loading and pipeline wiring but does not perform real translation.

SDK preflight example:

```bash
cargo run -p localmt --example model_pack_preflight -- ./models/m2m100-418m-int8
```

CLI smoke checks:

```bash
cargo run -p localmt -- model plan ./models/m2m100-418m-int8
cargo run -p localmt -- model doctor ./models/m2m100-418m-int8
cargo run -p localmt -- ffi startup
cargo run -p localmt -- ffi header
cargo run -p localmt -- ffi smoke ./models/m2m100-418m-int8 en ru "hello offline"
cargo run -p localmt --features hf-tokenizers -- ffi hf-smoke ./models/m2m100-418m-int8 en ru "hello offline"
cargo run -p localmt --features ort-runtime -- ffi ort-smoke ./models/m2m100-418m-int8
```

`localmt ffi startup` prints the Android-visible startup contract through the
same output-buffer ABI as JNI callers: ABI version, max text length, Xiaomi 17
metadata, compiled feature flags, and stable language codes.
`localmt ffi header` prints the checked-in C ABI header from
`crates/localmt-ffi/include/localmt_ffi.h` for Android/NDK consumers.
`localmt ffi smoke` runs the host-side equivalent of the default JNI call flow:
ABI query, model-pack summary, mock translator open, mock translate, status
message mapping on errors, and handle close.
`localmt ffi hf-smoke` uses the same C ABI shape with the HF-tokenizer-backed
mock translator: it verifies `tokenizer.json` loading and the translation buffer
contract while token generation remains deterministic mock generation.
`localmt ffi ort-smoke` uses the ORT generator preflight ABI to verify runtime
session loading for a local pack; it is a model/runtime load check, not
translation.

## Android FFI Boundary

`localmt-ffi` builds as `cdylib`, `staticlib`, and `rlib` so Android/JNI
adapters can link the Rust contract before real model execution is wired. The
primitive ABI is pointer-free: callers can query the ABI version, supported
language ids, two-byte ISO language codes, language-pair validation,
`MAX_TEXT_CHARS`, and the Xiaomi 17 target metadata.
`localmt_ffi_status_message` maps stable status codes to stable UTF-8 messages
through the same output-buffer contract; unknown input status values return the
message `unknown status`.

`localmt_ffi_model_pack_summary` lets Android/JNI adapters verify and plan a
local model pack through the Rust facade, then read the same stable newline
summary as `localmt model plan`. This does checksum verification, asset
planning, and optional `generation_config` parsing, but does not load tokenizer
backends or ONNX Runtime sessions. The CLI and FFI paths share
`OfflineTranslatorAssetsSummary::to_preflight_text` so adapter-visible summary
text has one source of truth.

For Android smoke integration, `localmt-ffi` also exposes a Rust-owned opaque
`LocalmtFfiTranslator` handle around `MockOfflineTranslator`. Callers open it
from a verified local model-pack path, pass UTF-8 input bytes, and provide the
output buffer; Rust writes translated UTF-8 bytes without NUL termination and
reports the required byte count when the buffer is too small. This path is still
mock translation.

`localmt_ffi_hf_tokenizer_enabled` and `LocalmtFfiHfTokenizer` provide tokenizer
preflight. Default builds verify the model pack and report tokenizer disabled;
`hf-tokenizers` builds verify the model pack and load the declared
`tokenizer.json`. This proves tokenizer JSON loading through the Android ABI,
not translation.

`LocalmtFfiHfMockTranslator` and `localmt_ffi_hf_mock_translate` provide the
next Android/JNI smoke path behind `hf-tokenizers`: the model pack is verified,
the Hugging Face `tokenizer.json` is loaded, and the normal UTF-8 translation
buffer contract is exercised. Token generation is still deterministic mock
generation, so this is stronger adapter coverage but not real translation.

`localmt_ffi_ort_runtime_enabled` and `LocalmtFfiOrtGenerator` provide runtime
preflight. Default builds report runtime disabled; `ort-runtime` builds can
verify a model pack and attempt to load encoder/decoder ONNX sessions. This is
a model/runtime load check, not translation. Wiring tokenizer output into ONNX
decoder execution remains future work.

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

`localmt-tokenizer` defines the tokenizer-side API. It owns `TokenId`,
non-empty bounded `TokenSequence`, `TokenizerInput`, `TokenizerOutput`, and the
`TokenizerEngine` trait. `MockTokenizer` performs deterministic UTF-8 byte
roundtrips so the future translation pipeline can be tested without
model-specific tokenizer dependencies. `HfTokenizer` is available behind the
`hf-tokenizers` feature and loads Hugging Face `tokenizer.json` files through
the `tokenizers` crate with default features disabled and pure-Rust
`fancy-regex` enabled. The `localmt` facade forwards the same feature.
`TokenizerAssetPlan` builds from a verified model pack, requires a declared
`tokenizer` role, and carries optional `vocab` and `config` paths for tokenizer
implementations.

```bash
cargo test -p localmt-tokenizer --features hf-tokenizers hf_tokenizer
cargo check -p localmt --features hf-tokenizers
cargo test -p localmt-cli --features hf-tokenizers tokenizer
```

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

Real translation still requires an ONNX-backed generator that consumes tokenizer
ids and executes the decoder loop; `HfTokenizer` only proves tokenizer JSON
loading, encoding, and decoding.

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
memory metrics will be added with an ONNX-backed `TokenGenerator`. The first
profile is `xiaomi17`, with Android ABI `arm64-v8a`, 12 GiB RAM class, and
preferred runtime hint `onnx-runtime-mobile-xnnpack`.

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
