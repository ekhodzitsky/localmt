# FFI ORT Translator Handle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose a C ABI handle for the real-tokenizer plus ORT offline translator.

**Architecture:** Add `LocalmtFfiOrtTranslator` beside existing mock/HF mock/ORT generator handles. The open function verifies assets first, returns tokenizer-disabled unless `hf-tokenizers` is enabled, returns runtime-disabled unless `ort-runtime` is enabled, and constructs `OrtOfflineTranslator` only when both features are enabled. The translate function mirrors the existing UTF-8 buffer contract used by mock translators.

**Tech Stack:** Rust FFI, existing status codes, existing `OrtOfflineTranslator`, existing C header string.

---

### Task 1: Add RED Unit Tests

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing FFI tests**

Add tests:

```rust
#[test]
#[cfg(not(feature = "hf-tokenizers"))]
fn ffi_ort_translator_open_reports_tokenizer_disabled() -> Result<(), Box<dyn std::error::Error>> {
    let root = create_runtime_config_pack()?;
    let path = path_bytes(&root)?;
    let mut translator: *mut LocalmtFfiOrtTranslator = ptr::null_mut();

    assert_eq!(
        localmt_ffi_ort_translator_open(path.as_ptr(), path.len(), &mut translator),
        LOCALMT_FFI_TOKENIZER_DISABLED
    );
    assert!(translator.is_null());

    localmt_ffi_ort_translator_close(translator);
    Ok(())
}

#[test]
fn ffi_ort_translator_open_rejects_nulls_and_invalid_utf8()
-> Result<(), Box<dyn std::error::Error>> {
    let root = create_runtime_config_pack()?;
    let path = path_bytes(&root)?;
    let mut translator: *mut LocalmtFfiOrtTranslator = ptr::null_mut();

    assert_eq!(
        localmt_ffi_ort_translator_open(path.as_ptr(), path.len(), ptr::null_mut()),
        LOCALMT_FFI_NULL_POINTER
    );
    assert_eq!(
        localmt_ffi_ort_translator_open([0xff].as_ptr(), 1, &mut translator),
        LOCALMT_FFI_INVALID_UTF8
    );
    assert!(translator.is_null());
    Ok(())
}

#[test]
fn ffi_ort_translate_rejects_null_handle() {
    let mut output = [0_u8; 32];
    let mut written_len = 0_usize;

    assert_eq!(
        localmt_ffi_ort_translate(
            ptr::null(),
            0,
            1,
            b"hello".as_ptr(),
            5,
            output.as_mut_ptr(),
            output.len(),
            &mut written_len,
        ),
        LOCALMT_FFI_NULL_POINTER
    );
}
```

Add CLI header assertion:

```rust
assert!(output.contains("int32_t localmt_ffi_ort_translator_open("));
assert!(output.contains("int32_t localmt_ffi_ort_translate("));
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-ffi ffi_ort_translator
cargo test -p localmt-cli cli_ffi_header_prints_bundled_header
```

Expected: compile failure because the new FFI handle and functions do not exist yet.

### Task 2: Implement FFI Handle

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Add handle type and imports**

Add `LocalmtFfiOrtTranslator` with a cfg-gated `OrtOfflineTranslator` field. Import `OrtOfflineTranslator` and `OrtOfflineTranslatorError` only under `all(feature = "hf-tokenizers", feature = "ort-runtime")`.

- [x] **Step 2: Add open/close functions**

Add:

```rust
pub extern "C" fn localmt_ffi_ort_translator_open(...)
pub extern "C" fn localmt_ffi_ort_translator_close(...)
```

Open must:
- validate `out_translator`
- parse UTF-8 path
- prepare `OfflineTranslatorAssets`
- return `LOCALMT_FFI_TOKENIZER_DISABLED` when `hf-tokenizers` is disabled
- return `LOCALMT_FFI_RUNTIME_DISABLED` when `hf-tokenizers` is enabled but `ort-runtime` is disabled
- construct `OrtOfflineTranslator::from_assets(assets)` when both features are enabled
- map tokenizer errors to `LOCALMT_FFI_TOKENIZER_ERROR`, ORT generator errors to `LOCALMT_FFI_ORT_ERROR`, and asset errors to `LOCALMT_FFI_MODEL_PACK_ERROR`

- [x] **Step 3: Add translate function**

Add:

```rust
pub extern "C" fn localmt_ffi_ort_translate(...)
```

Mirror `localmt_ffi_mock_translate`: validate pointers, language ids, pair, text, call `translator.translate(&request)`, write output length before capacity check, and return the same status codes.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-ffi ffi_ort_translator
```

Expected: new default-build FFI tests pass.

### Task 3: Header, Docs, And Verification

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`

- [x] **Step 1: Bump ABI and update header**

Increment `LOCALMT_FFI_ABI_VERSION`, update startup/header tests from `8` to `9`, and add C prototypes for the ORT translator handle functions.

- [x] **Step 2: Document ORT FFI translator**

Update README to say the mobile FFI now has an ORT translator handle behind `hf-tokenizers + ort-runtime`.

- [x] **Step 3: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi ffi_ort_translator
cargo test -p localmt-cli cli_ffi_header_prints_bundled_header
cargo test -p localmt-ffi
cargo test -p localmt-cli
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

### Task 4: Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run Kimi**

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 2: Commit with Lore protocol**

Create a commit explaining why the ORT translator FFI handle is feature-gated and separate from the ORT generator preflight handle.
