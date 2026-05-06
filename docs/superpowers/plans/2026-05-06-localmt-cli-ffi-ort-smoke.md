# CLI FFI ORT Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a host-side CLI command that exercises the Android FFI ORT generator preflight boundary.

**Architecture:** Extend `localmt ffi` with `ort-smoke PACK`. The command verifies and summarizes the model pack through `localmt_ffi_model_pack_summary`, then calls `localmt_ffi_ort_generator_open` and closes the handle on success. Default builds return the stable runtime-disabled status; `ort-runtime` builds attempt real ORT session loading through the existing FFI export.

**Tech Stack:** Rust CLI crate, existing `localmt-ffi` ORT preflight export, existing model-pack fixtures.

---

### Task 1: Lock CLI Contract

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `crates/localmt-cli/Cargo.toml`

- [x] **Step 1: Add failing tests**

Add a default-build test:

```rust
#[test]
#[cfg(not(feature = "ort-runtime"))]
fn cli_ffi_ort_smoke_reports_runtime_disabled_after_pack_planning()
-> Result<(), Box<dyn std::error::Error>> {
    let root = create_plannable_pack()?;
    let args = [
        "localmt".to_owned(),
        "ffi".to_owned(),
        "ort-smoke".to_owned(),
        root.display().to_string(),
    ];

    let result = run(args.into_iter());

    assert!(matches!(result, Err(ref error) if error.to_string().contains("runtime disabled")));
    Ok(())
}
```

Do not run `ort-smoke` under `ort-runtime` with fake `.onnx` fixtures in tests:
the command intentionally attempts runtime session loading and may block in the
dynamic ORT loader. Verify the feature build with `cargo check` instead.

Also assert top-level help contains `localmt ffi ort-smoke PACK`.

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli ffi_ort_smoke
```

Expected: fail because `ort-smoke` is not recognized under `localmt ffi`.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 3: Forward the feature**

Add the CLI feature:

```toml
ort-runtime = ["localmt/ort-runtime", "localmt-ffi/ort-runtime"]
```

- [x] **Step 4: Add command routing and implementation**

Add `ort-smoke` to `run_ffi`. Implement `run_ffi_ort_smoke` with one `PACK` argument:

```rust
let _summary = ffi_bytes(|output_ptr, output_capacity, written_len| {
    localmt_ffi::localmt_ffi_model_pack_summary(...)
})?;
let mut generator: *mut localmt_ffi::LocalmtFfiOrtGenerator = ptr::null_mut();
ffi_ok(localmt_ffi::localmt_ffi_ort_generator_open(..., &mut generator))?;
localmt_ffi::localmt_ffi_ort_generator_close(generator);
```

Success output:

```text
ffi_abi: 6
model_pack_summary: ok
ort_generator_open: ok
```

- [x] **Step 5: Update docs**

Document:

```bash
cargo run -p localmt --features ort-runtime -- ffi ort-smoke ./models/m2m100-418m-int8
```

Clarify that default builds return `runtime disabled`, and feature builds run a
runtime/session preflight, not translation.

### Task 3: Verify And Commit

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-cli ffi_ort_smoke
cargo check -p localmt-cli --features ort-runtime
cargo test -p localmt-cli cli_prints_help
```

Expected: pass.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli ffi_ort_smoke
cargo check -p localmt-cli --features ort-runtime
cargo test -p localmt-cli
cargo test -p localmt-cli --features ort-runtime
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the ORT FFI CLI smoke command, docs, tests, and plan with the Lore commit protocol.
