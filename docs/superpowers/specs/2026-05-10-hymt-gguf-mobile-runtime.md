# Hy-MT GGUF Mobile Runtime Spec

## Goal

Make `localmt` a Rust SDK for offline mobile translation with Hy-MT1.5-1.8B
1.25-bit GGUF as the first production model target.

## Product Contract

`localmt` is not a model repository and not primarily an app. It is the SDK
layer that verifies local model packs, exposes backend-agnostic Rust APIs,
provides C/JNI adapter surfaces, and packages native Android artifacts.

Runtime behavior must work without internet access. Model weights and runtime
libraries are user-supplied local assets and are not committed to this
repository.

## Model Target

Primary model:

- Name: Hy-MT1.5-1.8B-1.25bit GGUF.
- Provider: Tencent Hunyuan / AngelSlim.
- Published release note: 2026-04-29.
- Package size: 440 MB according to the upstream model card.
- Language coverage: 33 languages and 1,056 translation directions according
  to the upstream model card.
- Required starting languages: English, Russian, Thai, Vietnamese, Japanese.
- Runtime direction: llama.cpp with STQ1_0 / Sherry quantization support.

References:

- https://huggingface.co/tencent/Hy-MT1.5-1.8B-1.25bit-GGUF
- https://huggingface.co/tencent/HY-MT1.5-1.8B
- https://github.com/ggml-org/llama.cpp/pull/22836

## Runtime Strategy

Add a new llama.cpp-backed backend instead of stretching the existing ONNX
adapter to fit GGUF.

Planned crate:

- `localmt-engine-llama`: owns llama.cpp/GGUF integration details. The current
  slice is present: Rust-owned GGUF model plan, HY-MT prompt formatting, typed
  runtime config parsing, native loader preflight for a configured `libllama`,
  CPU-only model/context open lifecycle, and bounded prompt tokenization,
  eval, sampling, and UTF-8 decode through the dynamic llama.cpp C ABI.

Existing crates remain responsible for:

- `localmt-core`: language, text, request, and translation invariants.
- `localmt-engine`: backend-neutral translator traits.
- `localmt-models`: verified model-pack manifests and file integrity.
- `localmt-pipeline`: backend-neutral orchestration where useful.
- `localmt-ffi`: stable C ABI and Android-callable handles.
- `localmt-cli`: developer smoke commands.

ONNX Runtime remains available as an experimental backend. It must not be the
default production path for Hy-MT1.5.

## Model-Pack Requirements

A Hy-MT GGUF model pack must declare:

- Model id and version.
- Runtime family: `llama.cpp`.
- Required role: GGUF model file.
- Required language set.
- SHA-256 digest for every local file.
- Optional prompt/runtime config.
- License metadata copied from the user's accepted upstream source.

The verifier must reject:

- Missing GGUF model role.
- Unknown file roles.
- Duplicate file roles.
- Unsafe relative paths.
- Hash mismatches.
- Language pairs outside the supported set.

## Prompt Contract

For non-Chinese language pairs, the backend should use the HY-MT-style prompt:

```text
Translate the following segment into {target_language}, without additional explanation.

{source_text}
```

The backend must own the target-language label mapping instead of letting app
callers provide arbitrary prompt strings. This keeps prompt behavior stable
across FFI and Android callers.

## Android Device Strategy

Use Redmi Note 14 as the first physical smoke target because it is available
now and should cover the critical Android constraints:

- `arm64-v8a` ABI.
- Android dynamic library loading.
- JNI UTF-16 to Rust UTF-8 conversion.
- Local model-pack staging through `adb`.
- Offline execution in airplane mode.
- Memory and latency measurements on a real phone.

Use Xiaomi 17-class hardware for final performance claims.

## Acceptance Criteria

- README and roadmap identify Hy-MT GGUF as the production target.
- A Hy-MT GGUF model pack can be verified without loading the runtime.
- Host CLI can run a real Hy-MT GGUF smoke translation.
- Android staging can push FFI, runtime, and model-pack assets to a physical
  `arm64-v8a` phone.
- Android smoke can open a translator and return non-empty translations for
  English, Russian, Thai, Vietnamese, and Japanese smoke phrases.
- Runtime errors are explicit and typed across Rust and FFI.
- No model weights are committed to git.
- Existing ONNX tests continue to pass or are clearly reclassified as
  experimental compatibility coverage.

## Non-Goals

- Building a general chat app.
- Reimplementing llama.cpp kernels in Rust.
- Shipping Tencent model weights inside this repository.
- Claiming production readiness before real Android device smoke passes.
