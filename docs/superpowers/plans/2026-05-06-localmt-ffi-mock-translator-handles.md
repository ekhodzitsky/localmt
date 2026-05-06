# FFI Mock Translator Handles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Android/JNI adapters open a verified local model pack and run the facade mock translator through a C ABI smoke path.

**Architecture:** Extend `localmt-ffi` with one opaque translator handle owned by Rust. Keep the ABI dependency-free: callers pass UTF-8 byte slices for paths/text and caller-owned output buffers; Rust returns explicit status codes and never allocates C strings for callers.

**Tech Stack:** Rust workspace, `localmt-ffi`, `localmt::MockOfflineTranslator`, C header, std-only tests.

---

### Task 1: Lock Handle API

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Add failing tests**

Add tests for:
- opening a mock translator handle from a verified model-pack path
- translating UTF-8 bytes into a caller-owned output buffer
- reporting required output length when the buffer is too small
- rejecting null pointers, invalid UTF-8, invalid language ids, same-language pairs, and invalid text

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi ffi_mock
```

Expected: fail because handle status codes and exported functions are not implemented yet.

### Task 2: Implement Handle API

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`

- [x] **Step 3: Add status codes and opaque handle**

Add:
- `LOCALMT_FFI_NULL_POINTER`
- `LOCALMT_FFI_INVALID_UTF8`
- `LOCALMT_FFI_MODEL_PACK_ERROR`
- `LOCALMT_FFI_TEXT_ERROR`
- `LOCALMT_FFI_TRANSLATION_ERROR`
- `LOCALMT_FFI_BUFFER_TOO_SMALL`
- `LocalmtFfiTranslator`

- [x] **Step 4: Add exported handle functions**

Add:
- `localmt_ffi_mock_translator_open(path_ptr, path_len, out_translator)`
- `localmt_ffi_mock_translator_close(translator)`
- `localmt_ffi_mock_translate(translator, source_id, target_id, input_ptr, input_len, output_ptr, output_capacity, written_len)`

Rules:
- `open` writes a non-null handle only on success and nulls the out pointer before work.
- `close` is a no-op for null and consumes handles produced by `open`.
- `translate` writes the required UTF-8 byte length before buffer-size failure.
- Output bytes are not NUL terminated.

- [x] **Step 5: Update C header**

Document pointer ownership, byte-slice inputs, output-buffer semantics, and close requirements.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-ffi ffi_mock
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-ffi-mock-translator-handles.md`

- [x] **Step 7: Update README**

Document the mock handle API as an Android smoke path, not real inference.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi ffi_mock
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

Commit the handle API, tests, header, and docs with the Lore commit protocol.
