# CLI FFI HF Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a host-side CLI command that exercises the tokenizer-backed Android FFI mock translation path.

**Architecture:** Extend `localmt ffi` with `hf-smoke PACK FROM TO TEXT`. The default build verifies the model pack and reports the stable tokenizer-disabled FFI status. The `hf-tokenizers` build forwards the feature into `localmt-ffi`, opens `LocalmtFfiHfMockTranslator`, translates through `localmt_ffi_hf_mock_translate`, and closes the handle.

**Tech Stack:** Rust CLI crate, existing `localmt-ffi` exports, existing Hugging Face tokenizer feature and test fixtures.

---

### Task 1: Lock CLI Contract

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `crates/localmt-cli/Cargo.toml`

- [x] **Step 1: Add failing tests**

Add a default-build test:

```rust
#[test]
#[cfg(not(feature = "hf-tokenizers"))]
fn cli_ffi_hf_smoke_reports_tokenizer_feature_disabled_after_pack_planning()
-> Result<(), Box<dyn std::error::Error>> {
    let root = create_plannable_pack()?;
    let args = [
        "localmt".to_owned(),
        "ffi".to_owned(),
        "hf-smoke".to_owned(),
        root.display().to_string(),
        "en".to_owned(),
        "ru".to_owned(),
        "hello offline".to_owned(),
    ];

    let result = run(args.into_iter());

    assert!(
        matches!(result, Err(ref error) if error.to_string().contains("tokenizer disabled"))
    );
    Ok(())
}
```

Add a feature-build test:

```rust
#[test]
#[cfg(feature = "hf-tokenizers")]
fn cli_ffi_hf_smoke_loads_tokenizer_and_translates_through_ffi()
-> Result<(), Box<dyn std::error::Error>> {
    let root = create_hf_plannable_pack()?;
    let args = [
        "localmt".to_owned(),
        "ffi".to_owned(),
        "hf-smoke".to_owned(),
        root.display().to_string(),
        "en".to_owned(),
        "ru".to_owned(),
        "hello offline".to_owned(),
    ];

    let output = run(args.into_iter())?;

    assert!(output.contains("ffi_abi: 6"));
    assert!(output.contains("model_pack_summary: ok"));
    assert!(output.contains("hf_mock_translator_open: ok"));
    assert!(output.contains("hf_mock_translate: ok"));
    assert!(output.contains("translation: hello offline"));
    Ok(())
}
```

Also assert top-level help contains `localmt ffi hf-smoke PACK FROM TO TEXT`.

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli ffi_hf_smoke
```

Expected: fail because `hf-smoke` is not recognized under `localmt ffi`.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 3: Forward the feature**

Change the CLI feature to:

```toml
hf-tokenizers = ["localmt/hf-tokenizers", "localmt-ffi/hf-tokenizers"]
```

- [x] **Step 4: Add command routing and implementation**

Add `hf-smoke` to `run_ffi`. Implement `run_ffi_hf_smoke` with the same argument parsing and output-buffer helper as `run_ffi_smoke`, but call:

```rust
localmt_ffi::localmt_ffi_hf_mock_translator_open(...)
localmt_ffi::localmt_ffi_hf_mock_translate(...)
localmt_ffi::localmt_ffi_hf_mock_translator_close(...)
```

The success output must be:

```text
ffi_abi: 6
model_pack_summary: ok
hf_mock_translator_open: ok
hf_mock_translate: ok
translation: hello offline
```

- [x] **Step 5: Update docs**

Document:

```bash
cargo run -p localmt --features hf-tokenizers -- ffi hf-smoke ./models/m2m100-418m-int8 en ru "hello offline"
```

Clarify that this verifies tokenizer loading and FFI translation shape, but still uses mock generation.

### Task 3: Verify And Commit

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-cli ffi_hf_smoke
cargo test -p localmt-cli --features hf-tokenizers ffi_hf_smoke
cargo test -p localmt-cli cli_prints_help
```

Expected: pass.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli ffi_hf_smoke
cargo test -p localmt-cli --features hf-tokenizers ffi_hf_smoke
cargo test -p localmt-cli
cargo test -p localmt-cli --features hf-tokenizers
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the HF-tokenizer FFI CLI smoke command, docs, tests, and plan with the Lore commit protocol.
