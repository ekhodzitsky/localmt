# Model Doctor ORT Translate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `localmt model doctor` report the full ORT translator FFI path.

**Architecture:** Add a `doctor_ort_translate` helper beside `doctor_ort_generator`. It opens `LocalmtFfiOrtTranslator`, treats tokenizer-disabled and runtime-disabled as explicit readiness statuses for feature-gated builds, translates `DOCTOR_SMOKE_TEXT` only after open succeeds, and closes the handle before returning.

**Tech Stack:** Rust CLI, existing `localmt-ffi` translator ABI, existing `ffi_bytes`, `ffi_ok`, and status-message helpers.

---

### Task 1: Add RED Doctor Test

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Extend the default doctor test**

In `cli_model_doctor_reports_default_readiness_for_verified_pack`, add:

```rust
assert!(output.contains("ffi_ort_translate: tokenizer disabled"));
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-cli cli_model_doctor_reports_default_readiness_for_verified_pack
```

Expected: test fails because doctor output does not include `ffi_ort_translate`.

### Task 2: Implement Doctor Helper

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Wire helper into `doctor_model`**

Call `doctor_ort_translate(path_bytes, source_id, target_id)?` and append:

```text
ffi_ort_translate: {ort_translate_status}
```

to the doctor output.

- [x] **Step 2: Add `doctor_ort_translate`**

Add:

```rust
/// { path_bytes is a UTF-8 model-pack path and source_id/target_id form a valid FFI pair }
/// fn doctor_ort_translate(path_bytes: &[u8], source_id: u8, target_id: u8) -> Result<String, CliError>
/// { ret is Ok only when ORT FFI translation succeeds or required runtime features are disabled }
fn doctor_ort_translate(
    path_bytes: &[u8],
    source_id: u8,
    target_id: u8,
) -> Result<String, CliError> {
    let mut translator: *mut localmt_ffi::LocalmtFfiOrtTranslator = ptr::null_mut();
    let status = localmt_ffi::localmt_ffi_ort_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    );
    if status == localmt_ffi::LOCALMT_FFI_TOKENIZER_DISABLED
        || status == localmt_ffi::LOCALMT_FFI_RUNTIME_DISABLED
    {
        return Ok(ffi_status_text(status));
    }
    ffi_ok(status)?;

    let input = DOCTOR_SMOKE_TEXT.as_bytes();
    let translation = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_ort_translate(
            translator,
            source_id,
            target_id,
            input.as_ptr(),
            input.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    });
    localmt_ffi::localmt_ffi_ort_translator_close(translator);
    let _translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok("ok".to_owned())
}
```

- [x] **Step 3: Verify GREEN**

Run:

```bash
cargo test -p localmt-cli cli_model_doctor_reports_default_readiness_for_verified_pack
```

Expected: default doctor test passes and reports tokenizer disabled for the ORT translator path.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Document doctor coverage**

Update README `model doctor` text to mention the ORT translator FFI smoke status.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli cli_model_doctor_reports_default_readiness_for_verified_pack
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

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 2: Commit with Lore protocol**

Create a commit explaining why `model doctor` now checks both ORT generator preflight and ORT translator FFI readiness.
