# FFI ORT Generator Preflight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Android/JNI adapters check whether the Rust build can load ONNX generator sessions from a verified local model pack.

**Architecture:** Extend `localmt-ffi` with an opaque ORT generator preflight handle. The API verifies the model pack through the facade, then calls `OrtTokenGenerator::load`; default builds return a runtime-disabled status, while `ort-runtime` builds attempt real session loading.

**Tech Stack:** Rust workspace, existing optional `ort-runtime` feature, `localmt-ffi`, C header, std-only tests.

---

### Task 1: Lock ORT Preflight API

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Add failing tests**

Add tests for:
- `localmt_ffi_ort_runtime_enabled` returning `0` without `ort-runtime` and `1` with it
- opening an ORT generator from a verified pack returning runtime-disabled in default builds
- null out pointer and invalid UTF-8 rejection before runtime work

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi ffi_ort
```

Expected: fail because ORT preflight symbols and status codes are not implemented yet.

### Task 2: Implement ORT Preflight ABI

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`

- [x] **Step 3: Add status codes and handle**

Add:
- bump `LOCALMT_FFI_ABI_VERSION` to `2`
- `LOCALMT_FFI_RUNTIME_DISABLED`
- `LOCALMT_FFI_ORT_ERROR`
- `LocalmtFfiOrtGenerator`

- [x] **Step 4: Add exported ORT preflight functions**

Add:
- `localmt_ffi_ort_runtime_enabled()`
- `localmt_ffi_ort_generator_open(path_ptr, path_len, out_generator)`
- `localmt_ffi_ort_generator_close(generator)`

Rules:
- `open` nulls the out pointer before work.
- `open` maps facade model-pack/plan errors to `LOCALMT_FFI_MODEL_PACK_ERROR`.
- `open` maps `OrtRuntimeFeatureDisabled` to `LOCALMT_FFI_RUNTIME_DISABLED`.
- `open` maps other ORT load failures to `LOCALMT_FFI_ORT_ERROR`.
- `close` is a no-op for null and consumes handles produced by `open`.

- [x] **Step 5: Update C header**

Document runtime-enabled probing, ORT preflight ownership, and status-code mapping.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-ffi ffi_ort
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-ffi-ort-generator-preflight.md`

- [x] **Step 7: Update README**

Document ORT generator preflight as a runtime/model-file load check, not a translation API.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi ffi_ort
cargo test -p localmt-ffi
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 9: Commit**

Commit the ORT preflight ABI, tests, header, and docs with the Lore commit protocol.
