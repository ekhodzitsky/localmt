# localmt Roadmap

`localmt` is a Rust-first offline translation SDK for Android-class devices.
The library owns the stable API, model-pack verification, FFI/JNI surface, and
mobile packaging. Runtime engines are replaceable implementation details.

## Strategic Direction

The production direction is Hy-MT1.5-first:

- Use Hy-MT1.5-1.8B-1.25bit GGUF as the first real translation model target.
- Use a llama.cpp-backed runtime for GGUF and STQ1_0 / Sherry quantization.
- Keep ONNX Runtime as an experimental compatibility backend, not the default
  production path.
- Keep model weights out of git. Users provide verified local model packs.
- Prove everything on real Android hardware before calling it production-ready.

## Target Devices

- Primary performance target: Xiaomi 17-class Android, `arm64-v8a`.
- First available smoke device: Redmi Note 14, `arm64-v8a`.
- Host development target: macOS arm64.

Redmi Note 14 is enough for ABI, JNI, model-pack staging, runtime loading,
Unicode, memory, and first latency checks. Xiaomi 17 remains the target for
final benchmark claims.

## Milestone 1: Hy-MT Model-Pack Contract

Goal: represent a Hy-MT GGUF pack with the same trust guarantees as the current
ONNX-style pack flow.

- Add typed model-file roles for `gguf_model`, `chat_template`, and
  `llama_runtime_config`. Completed in the model-pack layer.
- Add manifest examples for Hy-MT1.5-1.8B-1.25bit GGUF. Completed under
  `examples/model-packs/hymt-1.25bit`.
- Add CLI validation that rejects missing GGUF files before runtime loading.
  Completed through `localmt model doctor` for `runtime: llama.cpp` packs.
- Add unsupported language-pair validation before runtime loading.
- Document license and redistribution requirements as external-user
  responsibilities.

Exit criteria:

- `localmt model doctor ./models/hymt-1.25bit` validates a local Hy-MT pack.
- No model weights are committed.

## Milestone 2: llama.cpp / GGUF Backend

Goal: add a feature-gated runtime backend without leaking llama.cpp types into
the public facade.

- Add `localmt-engine-llama` as an adapter crate. Completed.
- Encode translation as prompt-completion using the HY-MT prompt template.
  Completed at the prompt-boundary layer.
- Expose backend config through typed localmt-owned structs. Completed for
  no-runtime config parsing.
- Expose GGUF readiness through the Android-facing FFI ABI. Completed with
  `localmt ffi gguf-translate-smoke`, which validates the pack, prompt, and
  open path before decode-loop inference exists.
- Use dynamic llama.cpp loading as the integration boundary. Completed at the
  native-loader preflight layer through `localmt_ffi_llama_runtime_configure`
  and `LLAMA_CPP_DYLIB_PATH`; `llama-runtime` builds open the configured
  library and require the llama.cpp model/context/tokenizer/sampler C API
  symbols before accepting it.
- Bind to llama.cpp model/context creation through a narrow Rust-owned
  interface. Completed for CPU-only model/context open and drop lifecycle.
- Implement prompt tokenization, llama eval, sampling, and UTF-8 decode.
  Completed for the host dynamic-library runtime path; still needs real
  `libllama` + Hy-MT GGUF smoke evidence before release.
- Support CPU-only `arm64-v8a` execution first.

Exit criteria:

- Host CLI can run a GGUF smoke translation.
- Public API remains backend-agnostic.

## Milestone 3: Android Runtime Smoke

Goal: prove the real backend on a physical Android phone.

- Build Android native artifacts for `arm64-v8a`.
- Stage `liblocalmt_ffi.so`, llama runtime libraries, and a Hy-MT model pack
  with `adb`.
- Add a runnable Android smoke path that calls startup, model-pack validation,
  translator open, translate, and close.
- Capture device metadata, memory footprint, cold start, first token latency,
  and total translation latency.

Exit criteria:

- Redmi Note 14 produces real offline translations for English, Russian, Thai,
  Vietnamese, and Japanese smoke phrases.
- The smoke flow works with airplane mode enabled.

## Milestone 4: Production Hardening

Goal: turn the runtime into an SDK other apps can embed.

- Stabilize FFI ABI for GGUF translator handles.
- Add structured runtime errors for missing assets, unsupported language pairs,
  bad UTF-8/UTF-16, memory pressure, and backend initialization failures.
- Add deterministic benchmark scenarios and regression thresholds.
- Add release packaging for Android libraries and headers.
- Update docs with real-device benchmark data.

Exit criteria:

- CI covers host checks, Android cross-compilation, JNI syntax, and symbol
  drift.
- Manual device smoke instructions are short, repeatable, and verified.
- README shows the Hy-MT path as the default path.

## Milestone 5: Public-Quality Release

Goal: make the repository credible for external users.

- Publish a minimal Android sample app.
- Add a complete model-pack creation guide for Hy-MT GGUF.
- Add a benchmark table for Redmi Note 14 and Xiaomi 17-class devices.
- Add examples for direct Rust, CLI, C ABI, and JNI callers.
- Add clear limitations around model licensing, memory, and runtime maturity.

Exit criteria:

- A new user can clone the repo, provide model files, build the SDK, and run an
  offline translation smoke test on Android.
