# CLI FFI Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a host-side CLI command that exercises the same default FFI call flow Android/JNI adapters should use.

**Architecture:** Add `localmt ffi smoke PACK FROM TO TEXT` to `localmt-cli`. The command calls `localmt-ffi` exports directly: ABI version, model-pack summary, mock translator open/translate/close, and status-message lookup on errors. It uses the existing output-buffer retry contract so the CLI tests cover JNI buffer sizing behavior without requiring Android SDK or NDK.

**Tech Stack:** Rust CLI crate, `localmt-ffi` workspace dependency, existing test model-pack fixtures.

---

### Task 1: Lock CLI Contract

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `crates/localmt-cli/Cargo.toml`

- [x] **Step 1: Add failing tests**

Add tests:

```rust
#[test]
fn cli_runs_ffi_smoke_for_verified_pack() -> Result<(), Box<dyn std::error::Error>> {
    let root = create_plannable_pack()?;
    let args = [
        "localmt".to_owned(),
        "ffi".to_owned(),
        "smoke".to_owned(),
        root.display().to_string(),
        "en".to_owned(),
        "ru".to_owned(),
        "hello offline".to_owned(),
    ];

    let output = run(args.into_iter())?;

    assert!(output.contains("ffi_abi: 6"));
    assert!(output.contains("model_pack_summary: ok"));
    assert!(output.contains("mock_translator_open: ok"));
    assert!(output.contains("mock_translate: ok"));
    assert!(output.contains("translation: hello offline"));
    Ok(())
}
```

Also add a help test assertion that top-level help contains `localmt ffi smoke PACK FROM TO TEXT`.

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli ffi_smoke
```

Expected: fail because `ffi` is not a recognized top-level command.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 3: Add dependency and help text**

Add to `crates/localmt-cli/Cargo.toml`:

```toml
localmt-ffi = { path = "../localmt-ffi" }
```

Update top-level help with:

```text
localmt ffi smoke PACK FROM TO TEXT
```

- [x] **Step 4: Add FFI helpers and command routing**

Add `run_ffi`, `run_ffi_smoke`, `ffi_bytes`, `ffi_status_text`, and `ffi_language_id`.

The smoke command output must be:

```text
ffi_abi: 6
model_pack_summary: ok
mock_translator_open: ok
mock_translate: ok
translation: hello offline
```

When any FFI call returns a non-OK status, return `CliError::FfiStatus { status, message }`.

- [x] **Step 5: Update docs**

Document the command in README and `docs/android-build.md` as the host-side JNI call-flow smoke test.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-cli ffi_smoke
cargo test -p localmt-cli cli_prints_help
```

Expected: pass.

### Task 3: Verify And Commit

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli ffi_smoke
cargo test -p localmt-cli cli_prints_help
cargo test -p localmt-cli
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the CLI FFI smoke command, docs, tests, and plan with the Lore commit protocol.
