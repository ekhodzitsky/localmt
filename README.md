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
The optional `config` role can carry tokenizer metadata and, for ONNX Runtime
packs, the local `ort_io` tensor-name contract consumed by `localmt-engine-ort`.

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
localmt model runtime-config ./models/m2m100-418m-int8
localmt model tokenize ./models/m2m100-418m-int8 en ru "hello offline"
localmt ffi startup
localmt ffi header
localmt ffi runtime-config ./models/m2m100-418m-int8
localmt ffi ort-translate-bench ./models/m2m100-418m-int8 en ru "hello offline" 3
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
`OfflineTranslatorPlan`, parses an optional `generation_config`, and reports the
`ort_io_config` readiness status. It is a no-inference smoke command: it does
not load ONNX Runtime sessions or execute decoder graphs.
`model runtime-config` is the stricter no-inference gate for future ORT
generation: it requires both a valid `generation_config` and a valid `ort_io`
contract, then prints the selected generation limit and tensor names.
`model doctor` is the single local readiness gate for a pack: it runs facade
planning, the Android startup ABI summary, the shared FFI model-pack summary,
deterministic mock FFI translation, HF mock translation, ORT generator
preflight, and the ORT translator FFI smoke path. In default builds, disabled
tokenizer and runtime features are reported as diagnostic lines instead of
command failures.

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
cargo run -p localmt -- ffi runtime-config ./models/m2m100-418m-int8
cargo run -p localmt -- ffi smoke ./models/m2m100-418m-int8 en ru "hello offline"
cargo run -p localmt --features hf-tokenizers -- ffi hf-smoke ./models/m2m100-418m-int8 en ru "hello offline"
cargo run -p localmt --features ort-runtime -- ffi ort-smoke ./models/m2m100-418m-int8
cargo run -p localmt --features "hf-tokenizers ort-runtime" -- ffi ort-translate-smoke ./models/m2m100-418m-int8 en ru "hello offline"
cargo run -p localmt --features "hf-tokenizers ort-runtime" -- ffi ort-translate-bench ./models/m2m100-418m-int8 en ru "hello offline" 3
```

`localmt ffi startup` prints the Android-visible startup contract through the
same output-buffer ABI as JNI callers: ABI version, max text length, Xiaomi 17
metadata, compiled feature flags, and stable language codes.
`localmt ffi header` prints the checked-in C ABI header from
`crates/localmt-ffi/include/localmt_ffi.h` for Android/NDK consumers.
`localmt ffi runtime-config` exercises
`localmt_ffi_runtime_config_summary`: it verifies the pack through the Android
buffer ABI, requires strict ORT runtime metadata, and still avoids tokenizer or
ONNX Runtime loading.
`localmt ffi smoke` runs the host-side equivalent of the default JNI call flow:
ABI query, model-pack summary, mock translator open, mock translate, status
message mapping on errors, and handle close.
`localmt ffi hf-smoke` uses the same C ABI shape with the HF-tokenizer-backed
mock translator: it verifies `tokenizer.json` loading and the translation buffer
contract while token generation remains deterministic mock generation.
`localmt ffi ort-smoke` uses the ORT generator preflight ABI to verify runtime
session loading for a local pack; it is a model/runtime load check, not
translation.
`localmt ffi ort-translate-smoke` is the full translator dogfood path for
mobile adapters: it opens `LocalmtFfiOrtTranslator` and calls
`localmt_ffi_ort_translate` through the same output-buffer ABI that JNI will
use.
`localmt ffi ort-translate-bench` keeps the same translator handle open across
bounded repeated translations and reports phase timings for model-pack summary,
ORT translator open, first translation, total translation time, average
translation time, and total command time.

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
planning, optional `generation_config` parsing, and `ort_io_config` readiness
reporting, but does not load tokenizer backends or ONNX Runtime sessions. The
CLI and FFI paths share
`OfflineTranslatorAssetsSummary::to_preflight_text` so adapter-visible summary
text has one source of truth.

`localmt_ffi_runtime_config_summary` is the stricter Android readiness gate for
real ORT generation. It uses the same output-buffer ABI, requires both
`generation_config` and `ort_io` to parse, and returns the generation limit plus
selected tensor names without opening tokenizer backends or ONNX Runtime
sessions.

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
a model/runtime load check, not translation. ORT-enabled builds require an
absolute `libonnxruntime` path via `localmt_ffi_ort_runtime_configure()` or the
`ORT_DYLIB_PATH` fallback; missing or invalid paths return
`LOCALMT_FFI_RUNTIME_NOT_CONFIGURED` before session loading.
`LocalmtFfiOrtTranslator` wraps the SDK-level `OrtOfflineTranslator`
for Android/JNI callers behind
`hf-tokenizers + ort-runtime`: open verifies the same model pack and builds the
real tokenizer plus ORT generator, while `localmt_ffi_ort_translate` uses the
same UTF-8 output-buffer contract as the mock translator handles.

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

When running ORT smoke commands locally, point `ORT_DYLIB_PATH` at the runtime
library, for example `/opt/homebrew/lib/libonnxruntime.dylib` on a macOS dev
machine. Android/JNI callers should pass the app
`nativeLibraryDir/libonnxruntime.so` path through
`localmt_ffi_ort_runtime_configure()` before opening ORT handles.

`OrtGeneratorPlan` builds on the pipeline-owned `GeneratorAssetPlan` and turns
verified encoder/decoder assets into ORT session plans. `OrtTokenGenerator`
loads the required ORT sessions only when `ort-runtime` is enabled; the default
build returns `OrtRuntimeFeatureDisabled`. `OrtTokenGenerator` now satisfies the
pipeline `TokenGenerator` trait: default builds still report
`BackendUnavailable("ONNX token generation loop is not implemented")`, while
`ort-runtime` builds run encoder execution plus the non-cached decoder loop.
`OrtOfflineTranslator` combines that generator with `HfTokenizer` when both
runtime features are enabled.

`OrtGeneratorPlan::parse_generation_config` parses the optional verified
`generation_config` asset into `GenerationConfig`. Packs without that role
return `Ok(None)`. Runtime session loading and decoder execution consume the
config only when `ort-runtime` is enabled.
`OrtGeneratorPlan::parse_ort_io_config` parses the optional verified `config`
asset when present and expects an `ort_io` object with the tensor names that the
decoder loop binds. Packs without `config` return `Ok(None)`;
packs with `config` but without `ort_io` fail the explicit ORT I/O parse.
The facade converts that missing-object case into the stable preflight line
`ort_io_config: missing`; valid contracts print `ort_io_config: parsed`, and
packs with no `config` role print `ort_io_config: absent`.
For runtime generation, `OrtGeneratorPlan::parse_runtime_config` is stricter:
it requires both a parsed `generation_config` and a parsed `ort_io` config.
Feature-enabled `OrtTokenGenerator::load` parses that strict config before
loading ONNX sessions, so missing decoder-loop settings or tensor names fail
before any model execution starts.
`OrtGenerationInputs::from_tokenizer_output` is the next no-inference boundary:
it converts tokenizer source ids into ONNX-friendly `i64` encoder ids, builds
the encoder attention mask, seeds decoder input ids with an optional
`decoder_start_token_id` followed by the configured target language token, and
carries `max_new_tokens` plus EOS. `OrtTokenGenerator` uses those inputs to seed
feature-enabled encoder and decoder execution.
`OrtGenerationState` now owns the deterministic decoder-loop state on top of
those inputs: it appends accepted token ids, extends decoder input ids, and
marks the loop finished on EOS or the max-new-token limit.
`OrtGenerationTensorInputs::from_generation_inputs` prepares the first concrete
ORT tensor-input binding layer: encoder input ids, encoder attention mask, and
decoder input ids are owned `i64` row tensors with `[1, N]` shapes, ready for
future `ort::value::Tensor::from_array` calls.
With `ort-runtime` enabled, `OrtEngine::run_encoder` is the first real execution
primitive: it binds named encoder input tensors, runs the mutable encoder
session, and copies the named `last_hidden_state` output into owned `f32`
values. `OrtEngine::run_decoder` then consumes growing decoder ids plus encoder
hidden states for the non-cached decoder loop.
Runtime-enabled `OrtTokenGenerator` stores encoder, decoder, and optional cached
decoder sessions in `OrtEngineSlot`. The slot uses a standard mutex to provide
mutable ORT session access behind the shared `TokenGenerator::generate(&self)`
API, so the pipeline trait can stay stable while real session execution is
wired incrementally.
With `ort-runtime`, `OrtTokenGenerator::generate` now prepares generation
inputs, runs the encoder stage, and drives the non-cached decoder loop until EOS
or the max-new-token limit. Default builds keep the immediate unavailable-backend
response because they intentionally do not link or load ONNX Runtime.
`OrtEngine::run_decoder` is available for the non-cached decoder graph: it binds
decoder input ids, encoder attention mask, and encoder hidden states, then
copies the named decoder logits output. Cached decoder-with-past execution
remains a future runtime step.
`OrtDecoderLogits::final_token_logits` now validates `[1, sequence, vocabulary]`
decoder output and extracts the final sequence-position vocabulary row.
`OrtDecoderLogits::select_next_token` delegates that row to the deterministic
argmax selector before any generator state mutation happens.
`OrtGenerationStep::accept_decoder_output` applies selected decoder tokens to
`OrtGenerationState` while preserving distinct logits and state-transition
errors.
`OrtGenerationTensorInputs::from_generation_state` rebuilds decoder input
tensors from the growing state while keeping encoder input ids and attention
mask anchored to the original generation inputs.
`OrtNextTokenSelector::select_argmax` handles the first logits-selection policy:
it rejects empty or non-finite logits, chooses the highest vocabulary index as a
`TokenId`, and keeps the first index on ties. Cached decoder-with-past,
real-model runtime validation, and detokenizing real model output remain the
next runtime steps.

Accepted local `config.json` fragment for ORT I/O:

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
    },
    "decoder_with_past": {
      "input_ids": "input_ids",
      "encoder_attention_mask": "encoder_attention_mask",
      "encoder_hidden_states": "encoder_hidden_states",
      "logits": "logits"
    }
  }
}
```

`decoder_with_past` inside `ort_io` is optional and mirrors the optional
`decoder_with_past` model-pack role.

## Tokenizer Boundary

`localmt-tokenizer` defines the tokenizer-side API. It owns `TokenId`,
non-empty bounded `TokenSequence`, `TokenizerInput`, `TokenizerOutput`, and the
`TokenizerEngine` trait. `MockTokenizer` performs deterministic UTF-8 byte
roundtrips so the future translation pipeline can be tested without
model-specific tokenizer dependencies. `HfTokenizer` is available behind the
`hf-tokenizers` feature and loads Hugging Face `tokenizer.json` files through
the `tokenizers` crate with default features disabled and pure-Rust
`fancy-regex` enabled. When the tokenizer exposes the first supported NLLB
language tokens, `HfTokenizer` prefixes the encoded source with the request's
source language token and appends `</s>` instead of relying on the tokenizer's
serialized default source language. The `localmt` facade forwards the same
feature.
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

Real translation is wired through `OrtOfflineTranslator` when both
`hf-tokenizers` and `ort-runtime` are enabled; default builds still use mock
tokenizer/generator components for deterministic tests and adapter smoke checks.
Cached decoder execution, beam search, and model-specific quality tuning remain
future work.

## Generation Config

`GenerationConfig` captures decoder-loop settings as typed values. It owns a
bounded `MaxNewTokens`, required BOS/EOS token ids, an optional
`decoder_start_token_id`, and target-language token ids for `en`, `ru`, `th`,
`vi`, and `ja`.
`GenerationConfig::with_default_limit` uses `DEFAULT_MAX_NEW_TOKENS` and rejects
duplicate or colliding token roles.

Accepted local `generation_config` JSON shape:

```json
{
  "max_new_tokens": 64,
  "bos_token_id": 0,
  "decoder_start_token_id": 2,
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

`max_new_tokens` and `decoder_start_token_id` are optional. Omitted
`max_new_tokens` uses `DEFAULT_MAX_NEW_TOKENS`; omitted
`decoder_start_token_id` preserves the legacy decoder seed of only the target
language token.
`GenerationConfig::from_json_file` and `GenerationConfig::from_json_str` parse
this schema and then run the same token-role validation.

## Benchmark Skeleton

The top-level benchmark command verifies a model pack, then runs 10 fixed
language pair scenarios through `TranslationPipeline<MockTokenizer,
MockTokenGenerator>`. It is intentionally labeled `runtime: mock-pipeline`.
Real ORT latency measurement lives in `localmt ffi ort-translate-bench`, which
uses the same FFI translator path as Android adapters and keeps one translator
handle warm across repeated runs. The first profile is `xiaomi17`, with Android
ABI `arm64-v8a`, 12 GiB RAM class, and preferred runtime hint
`onnx-runtime-mobile-xnnpack`.

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
