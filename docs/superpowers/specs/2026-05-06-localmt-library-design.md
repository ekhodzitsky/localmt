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
- `localmt-engine-ort`: selects ONNX files from a verified model pack and loads
  an ONNX Runtime session only when the `ort-runtime` feature is enabled.
- `localmt-models`: manifest parsing, safe relative path validation, required
  language checks, typed file-role validation, and SHA-256 verification for
  model-pack files.
- `localmt-pipeline`: composition layer that wires tokenizer encode, token
  generation, tokenizer decode, and the existing `TranslatorEngine` trait.
- `localmt-tokenizer`: typed token ids, non-empty bounded token sequences,
  tokenizer input/output types, tokenizer trait, and deterministic mock
  tokenizer.
- `localmt-bench`: device profiles and benchmark report types. The first
  implementation verifies a model pack and runs fixed smoke scenarios against
  the mock pipeline.
- `localmt`: facade crate that re-exports stable public API and owns
  `Translator<E>`.
- `localmt-cli`: development-only smoke CLI.

The ONNX Runtime crate is isolated as `localmt-engine-ort`, behind a feature
flag. It must not leak `ort` types into `localmt-core` or `localmt`; facade
users see only localmt-owned plan, role, engine, and error types.

Model-pack `files[].kind` values are not arbitrary labels. They are parsed into
`ModelFileRole` values: `encoder`, `decoder`, `decoder_with_past`, `tokenizer`,
`vocab`, `config`, and `generation_config`. The parser rejects unknown and
duplicate roles so backend adapters can rely on typed role selection.
Verified packs expose `file_path(role)` so tokenizer and inference adapters use
one safe root-qualified role-path lookup instead of repeating manifest scans.
`Sha256Digest::from_file` and `localmt model hash <file>` expose the same
digest implementation used by verification so local model-pack manifests can be
authored without separate checksum tooling.

The tokenizer boundary exists before real model-specific tokenization. It uses
localmt-owned `TokenId`, `TokenSequence`, `TokenizerInput`, and
`TokenizerOutput` types so SentencePiece/BPE implementations can be added later
without changing the public translation pipeline shape. `TokenizerAssetPlan`
bridges verified model packs to future tokenizer implementations by requiring a
`tokenizer` role and preserving optional `vocab` and `config` paths.

The pipeline layer is the first end-to-end SDK shape. It composes a
`TokenizerEngine` and a `TokenGenerator`, then adapts pipeline errors into
`TranslationError` through the existing `TranslatorEngine` trait. The current
mock generator echoes tokens; real translation will replace only that generator
and tokenizer implementation, not the public request/translation surface.
`GeneratorAssetPlan` bridges verified model packs to future generator
implementations by requiring `encoder` and `decoder` graph roles and preserving
optional `decoder_with_past` and `generation_config` paths.
`OrtGeneratorPlan` consumes that asset plan for ONNX Runtime and converts graph
assets into encoder/decoder session plans before any session is loaded.
`OrtTokenGenerator::load` loads those sessions behind `ort-runtime`; default
builds still return an explicit disabled-runtime error.
`OrtTokenGenerator` implements the pipeline `TokenGenerator` contract now, but
`generate` returns an explicit unavailable-backend error until encoder/decoder
tensor I/O and decoding semantics are implemented.
`GenerationConfig` provides typed generation-loop settings: bounded
`max_new_tokens`, BOS/EOS token ids, and target-language token ids for the first
language set. It validates duplicate or colliding token roles before any decoder
loop consumes those values. The pipeline crate parses the local
`generation_config` JSON schema into that type; `max_new_tokens` is optional and
defaults to `DEFAULT_MAX_NEW_TOKENS`.
`OrtGeneratorPlan::parse_generation_config` bridges the verified optional
`generation_config` model-pack asset into this typed config while keeping ORT
session loading and decoder execution separate.
`OfflineTranslatorPlan` is the facade-level planning object for SDK users. It
combines `TokenizerAssetPlan` and `OrtGeneratorPlan` from one verified model
pack without parsing tokenizer files, loading ORT sessions, or running
translation.
It also exposes facade-level `parse_generation_config` so SDK adapters do not
need to reach into ORT-specific plan internals just to validate decoder-loop
settings.
`OfflineTranslatorAssets` is the prepared no-inference facade object above raw
planning: it owns the plan plus parsed optional generation config for CLI and
future mobile adapters.
Its `from_model_pack_path` constructor is the SDK-level local-pack entry point
for discover -> verify -> prepare without exposing those lower-level steps to
app adapters.
Its `summary` method returns an owned preflight summary so CLI, Android, and
future UniFFI/JNI adapters can display or validate prepared paths without
parsing text output.
The development CLI and benchmark command use this pipeline path so smoke tests
exercise the same shape that real inference will fill.
`localmt model plan <pack>` is the no-inference CLI smoke path for verified
asset planning: it verifies the pack, builds `OfflineTranslatorPlan`, parses
optional generation config, and avoids ONNX Runtime session loading.

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
- Model packs can be inspected and verified before real inference loads them.
- Model-pack file roles are typed and duplicate roles are rejected at discovery.
- Verified model packs resolve declared file roles to root-qualified paths for
  backend adapters.
- SHA-256 manifest digests can be computed through the public model layer and
  development CLI.
- Tokenizer input/output and token sequence invariants are represented by
  localmt-owned types with a deterministic mock tokenizer.
- Tokenizer asset planning requires a verified `tokenizer` file role and
  preserves optional vocabulary/config paths.
- A pipeline skeleton composes tokenizer encode, token generation, and tokenizer
  decode while implementing `TranslatorEngine`.
- Generator asset planning requires verified `encoder` and `decoder` file roles
  and preserves optional cached-decoder/generation-config paths.
- ORT generator planning consumes verified generator assets and produces
  encoder/decoder session plans without running inference.
- ORT token-generator loading is feature-gated and default builds return an
  explicit disabled-runtime error.
- ORT token generator satisfies the pipeline trait while returning an explicit
  unavailable-backend error for the unimplemented generation loop.
- Typed generation config captures max-new-token limits, BOS/EOS ids, and
  target-language token ids, with parser support for the local
  `generation_config` JSON schema.
- ORT generator plans can parse optional verified generation config assets
  without loading sessions or running decoder inference.
- Facade-level offline translator planning combines tokenizer and ORT generator
  plans from one verified model pack without loading runtime resources.
- Facade-level offline translator plans can parse optional generation config
  without exposing ORT-specific generator internals to adapters.
- Facade-level prepared assets combine the verified offline translator plan and
  parsed optional generation config without loading runtime resources.
- Prepared assets can be created directly from a local model-pack path through
  the facade without adapter-side model-pack lifecycle wiring.
- Prepared assets expose a structured preflight summary for adapter-facing
  model id, asset paths, and generation-config status.
- Development CLI can run `localmt model plan <pack>` to smoke verified facade
  planning and generation-config parsing without inference.
- Xiaomi 17 benchmark command exists and clearly labels `mock-pipeline`
  results.
- ONNX Runtime session loading is gated behind `ort-runtime`; default builds
  can plan a session but return an explicit disabled-runtime error on load.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets --all-features
  -- -D warnings`, and `cargo doc --no-deps` pass.
