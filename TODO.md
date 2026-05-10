# TODO

Current focus: transition from ONNX-first infrastructure to a Hy-MT1.5
GGUF-first production backend while preserving the existing Rust SDK, model-pack,
FFI, and Android foundations.

## Now

- [x] Add Hy-MT GGUF model-pack roles and validation.
- [x] Add a sample Hy-MT model-pack manifest without committing model weights.
- [x] Add `localmt model doctor` coverage for GGUF model packs.
- [x] Create `localmt-engine-llama` as a feature-gated backend crate.
- [x] Add Rust-owned HY-MT prompt formatting for llama.cpp completion.
- [x] Add typed llama runtime config parsing without native loading.
- [x] Add host CLI/FFI readiness command:
      `localmt ffi gguf-translate-smoke ./models/hymt-1.25bit en ru "hello"`.
- [x] Decide the llama.cpp integration boundary: dynamic library, static build,
      or vendored source build.
- [x] Add FFI configuration for the llama.cpp dynamic library path.
- [x] Add native loader preflight for configured llama.cpp dynamic libraries.
- [x] Implement llama.cpp model/context creation behind the existing GGUF FFI
      handle.
- [x] Implement prompt tokenization, llama eval, sampling, and UTF-8 decode.
- [ ] Run the llama path against a real local `libllama` and Hy-MT GGUF model.

## Android

- [ ] Keep `scripts/package-android-ffi.sh` working for `arm64-v8a`.
- [ ] Extend Android packaging to include llama runtime artifacts.
- [ ] Use Redmi Note 14 for first real-device smoke.
- [ ] Capture device metadata with `adb`.
- [ ] Run smoke tests in airplane mode.
- [ ] Record cold start, model open time, first token latency, total latency,
      peak memory, and output text.

## Hy-MT Specific

- [ ] Verify supported language names and prompt labels for `en`, `ru`, `th`,
      `vi`, and `ja`.
- [ ] Encode the HY-MT prompt template in Rust-owned backend config.
- [ ] Verify STQ1_0 support status in llama.cpp before claiming stable support.
- [ ] Pin a patched llama.cpp build revision and record build provenance in
      release artifacts.
- [ ] Document model license and redistribution constraints.
- [ ] Add a local-only download/build guide for users who accept the upstream
      model terms.

## Release Readiness

- [ ] Add README quick start for Hy-MT GGUF.
- [ ] Add Android sample app or native runner that performs a real translation.
- [ ] Add benchmark report format for Android devices.
- [ ] Add CI checks for new GGUF manifest examples.
- [ ] Keep ONNX Runtime documented as experimental until it has comparable
      model quality and mobile performance evidence.
