# CLI ORT Translate Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a CLI smoke command that exercises `LocalmtFfiOrtTranslator` through the C ABI.

**Architecture:** Add `localmt ffi ort-translate-smoke PACK FROM TO TEXT` beside the existing FFI smoke commands. It verifies the model pack through `localmt_ffi_model_pack_summary`, opens `LocalmtFfiOrtTranslator`, maps feature-disabled and ORT failures through existing FFI status messages, and translates through `localmt_ffi_ort_translate` only after open succeeds.

**Tech Stack:** Rust CLI, existing `localmt-ffi` C ABI, existing FFI status and byte-buffer helpers.

---

### Task 1: Add RED CLI Tests

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add help and default-build smoke tests**

Add assertions to `cli_prints_help` and `cli_prints_ffi_help`:

```rust
assert!(output.contains("localmt ffi ort-translate-smoke PACK FROM TO TEXT"));
```

Add this default-build test near the existing ORT smoke tests:

```rust
#[test]
#[cfg(not(feature = "hf-tokenizers"))]
fn cli_ffi_ort_translate_smoke_reports_tokenizer_disabled_after_pack_planning()
-> Result<(), Box<dyn std::error::Error>> {
    let root = create_runtime_config_pack()?;
    let args = [
        "localmt".to_owned(),
        "ffi".to_owned(),
        "ort-translate-smoke".to_owned(),
        root.display().to_string(),
        "en".to_owned(),
        "ru".to_owned(),
        "hello offline".to_owned(),
    ];

    let result = run(args.into_iter());

    assert!(matches!(result, Err(ref error) if error.to_string().contains("tokenizer disabled")));
    Ok(())
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-cli cli_ffi_ort_translate_smoke
cargo test -p localmt-cli cli_prints_ffi_help
```

Expected: the new smoke test fails with `unknown ffi command: ort-translate-smoke`, and the help test fails because the usage text does not list the new command.

### Task 2: Implement CLI Command

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add command to usage and router**

Add `localmt ffi ort-translate-smoke PACK FROM TO TEXT` to `HELP_TEXT` and `FFI_HELP_TEXT`, then route `"ort-translate-smoke"` to `run_ffi_ort_translate_smoke(args)`.

- [x] **Step 2: Implement `run_ffi_ort_translate_smoke`**

Add a function mirroring `run_ffi_hf_smoke`, but using `LocalmtFfiOrtTranslator`:

```rust
/// { args contains FFI ORT translation smoke command arguments }
/// fn run_ffi_ort_translate_smoke(args: impl Iterator<Item = String>) -> Result<String, CliError>
/// { ret is Ok only when ORT-backed FFI translation succeeds }
fn run_ffi_ort_translate_smoke(mut args: impl Iterator<Item = String>) -> Result<String, CliError> {
    let path = args.next().ok_or(CliError::MissingArgument("MODEL_PACK"))?;
    let source = parse_language(args.next(), "FROM")?;
    let target = parse_language(args.next(), "TO")?;
    let text = args.next().ok_or(CliError::MissingArgument("TEXT"))?;

    if args.next().is_some() {
        return Err(CliError::TooManyArguments);
    }

    let path_bytes = path.as_bytes();
    let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
        localmt_ffi::localmt_ffi_model_pack_summary(
            path_bytes.as_ptr(),
            path_bytes.len(),
            output_ptr,
            output_capacity,
            written_len,
        )
    })?;
    let source_id = ffi_language_id(source)?;
    let target_id = ffi_language_id(target)?;

    let mut translator: *mut localmt_ffi::LocalmtFfiOrtTranslator = ptr::null_mut();
    ffi_ok(localmt_ffi::localmt_ffi_ort_translator_open(
        path_bytes.as_ptr(),
        path_bytes.len(),
        &mut translator,
    ))?;

    let input = text.as_bytes();
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
    let translation = String::from_utf8(translation?).map_err(CliError::FfiOutputUtf8)?;

    Ok(format!(
        "ffi_abi: {}\nmodel_pack_summary: ok\nort_translator_open: ok\nort_translate: ok\ntranslation: {translation}",
        localmt_ffi::localmt_ffi_abi_version()
    ))
}
```

- [x] **Step 3: Verify GREEN**

Run:

```bash
cargo test -p localmt-cli cli_ffi_ort_translate_smoke
cargo test -p localmt-cli cli_prints_ffi_help
```

Expected: both tests pass in the default build.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Document the command**

Add `cargo run -p localmt --features 'hf-tokenizers ort-runtime' -- ffi ort-translate-smoke ./models/m2m100-418m-int8 en ru "hello offline"` to the README command list, and document that it is the CLI dogfood path for `LocalmtFfiOrtTranslator`.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli cli_ffi_ort_translate_smoke
cargo test -p localmt-cli cli_prints_ffi_help
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

Create a commit explaining why the translator smoke command is separate from ORT generator preflight.
