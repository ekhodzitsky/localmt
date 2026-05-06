# FFI HF Tokenizer Preflight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Android/JNI adapters verify that a local model pack contains a loadable Hugging Face `tokenizer.json` without exposing token buffers over C ABI yet.

**Architecture:** Add a `hf-tokenizers` feature to `localmt-ffi` and forward it to `localmt/hf-tokenizers`. Bump the FFI ABI to v3, add tokenizer-specific disabled/load error status codes, and expose an opaque `LocalmtFfiHfTokenizer` preflight handle with `enabled/open/close` functions.

**Tech Stack:** Rust workspace, existing FFI raw-pointer validation pattern, optional `HfTokenizer`, existing verified model-pack helpers.

---

### Task 1: Lock FFI Tokenizer Contract

**Files:**
- Modify: `crates/localmt-ffi/Cargo.toml`
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`

- [x] **Step 1: Add failing tests**

Add tests that prove:
- default builds report `localmt_ffi_hf_tokenizer_enabled() == 0`
- feature builds report `localmt_ffi_hf_tokenizer_enabled() == 1`
- default builds verify a pack then return `LOCALMT_FFI_TOKENIZER_DISABLED`
- feature builds load a valid tokenizer JSON and return an owned non-null handle
- null and invalid UTF-8 inputs map to existing FFI statuses

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi hf_tokenizer
```

Expected: fail because the tokenizer FFI symbols and statuses do not exist.

### Task 2: Implement FFI Tokenizer Preflight

**Files:**
- Modify: `crates/localmt-ffi/Cargo.toml`
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`
- Modify: `README.md`

- [x] **Step 3: Add FFI feature and status codes**

Add:
- `localmt-ffi/hf-tokenizers`
- `LOCALMT_FFI_TOKENIZER_DISABLED`
- `LOCALMT_FFI_TOKENIZER_ERROR`
- `LOCALMT_FFI_ABI_VERSION = 3`

- [x] **Step 4: Add handle API**

Add:
- `LocalmtFfiHfTokenizer`
- `localmt_ffi_hf_tokenizer_enabled`
- `localmt_ffi_hf_tokenizer_open`
- `localmt_ffi_hf_tokenizer_close`

`open` verifies the model pack through `OfflineTranslatorAssets`; feature-disabled builds return tokenizer-disabled after planning; feature builds load `HfTokenizer` from the verified tokenizer path.

- [x] **Step 5: Update C header and README**

Expose the new status constants, ABI version, opaque type, and function declarations. Document that this is tokenizer load preflight only, not translation.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-ffi hf_tokenizer
cargo test -p localmt-ffi --features hf-tokenizers hf_tokenizer
```

Expected: both pass.

### Task 3: Verify And Commit

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi hf_tokenizer
cargo test -p localmt-ffi --features hf-tokenizers hf_tokenizer
cargo test -p localmt-ffi
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt-ffi --features "ort-runtime hf-tokenizers"
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 8: Commit**

Commit the tokenizer FFI preflight API, tests, docs, header, and plan with the Lore commit protocol.
