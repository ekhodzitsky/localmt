# FFI HF Mock Translator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Android/JNI adapters run the tokenizer-backed mock translation path through the C ABI.

**Architecture:** Add an opaque `LocalmtFfiHfMockTranslator` handle to `localmt-ffi`. Default builds verify the model pack and return `LOCALMT_FFI_TOKENIZER_DISABLED`; `hf-tokenizers` builds load `HfMockOfflineTranslator`. Translation reuses the existing UTF-8 buffer contract and remains mock generation, not real ONNX translation.

**Tech Stack:** Rust FFI crate, existing ABI validation helpers, `HfMockOfflineTranslator`, existing status codes.

---

### Task 1: Lock FFI Contract

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`

- [x] **Step 1: Add failing tests**

Add tests that prove:
- default builds verify a pack then return `LOCALMT_FFI_TOKENIZER_DISABLED`
- feature builds open a verified pack with valid tokenizer JSON and return a non-null handle
- feature builds translate UTF-8 input through the HF-tokenizer-backed mock pipeline
- feature builds map invalid tokenizer JSON to `LOCALMT_FFI_TOKENIZER_ERROR`
- null and invalid UTF-8 inputs map to existing FFI statuses

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi hf_mock_translator
```

Expected: fail because the HF mock translator FFI symbols do not exist.

### Task 2: Implement FFI API

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`
- Modify: `README.md`

- [x] **Step 3: Add handle and ABI v4**

Add:
- `LOCALMT_FFI_ABI_VERSION = 4`
- `LocalmtFfiHfMockTranslator`

- [x] **Step 4: Add open/translate/close**

Add:
- `localmt_ffi_hf_mock_translator_open`
- `localmt_ffi_hf_mock_translate`
- `localmt_ffi_hf_mock_translator_close`

The translate call must follow the existing output-buffer contract and return `LOCALMT_FFI_BUFFER_TOO_SMALL` with required byte count when needed.

- [x] **Step 5: Update C header and README**

Document that this path uses real tokenizer loading plus mock generation.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-ffi hf_mock_translator
cargo test -p localmt-ffi --features hf-tokenizers hf_mock_translator
```

Expected: both pass.

### Task 3: Verify And Commit

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi hf_mock_translator
cargo test -p localmt-ffi --features hf-tokenizers hf_mock_translator
cargo test -p localmt-ffi
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt-ffi --features hf-tokenizers
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 8: Commit**

Commit the FFI HF mock translator API, tests, docs, header, and plan with the Lore commit protocol.
