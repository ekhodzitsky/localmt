# Model Doctor Startup Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `localmt model doctor PACK` include the Android startup ABI summary check.

**Architecture:** Reuse the new `localmt_ffi_startup_summary` buffer ABI inside the existing doctor command. The doctor output gains `ffi_startup: ok` before model-pack-specific checks, so the single readiness gate covers both app startup contract and pack readiness.

**Tech Stack:** Rust CLI crate, existing localmt-ffi startup summary export, current model-pack fixtures.

---

### Task 1: Lock Doctor Output

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add failing test assertion**

Update the existing doctor test to assert:

```rust
assert!(output.contains("ffi_startup: ok"));
```

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli model_doctor
```

Expected: fail because doctor does not yet include the startup summary line.

### Task 2: Implement Startup Check

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 3: Add helper**

Add:

```rust
fn doctor_ffi_startup() -> Result<&'static str, CliError>
```

It calls `localmt_ffi_startup_summary` through `ffi_bytes`, validates UTF-8, and returns `ok`.

- [x] **Step 4: Include it in doctor output**

Add:

```text
ffi_startup: ok
```

between `model_plan: ok` and `ffi_model_pack_summary: ok`.

- [x] **Step 5: Update docs**

Update README and Android docs so doctor mentions startup ABI summary.

### Task 3: Verify And Commit

- [x] **Step 6: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli model_doctor
cargo test -p localmt-cli
cargo test -p localmt-cli --features hf-tokenizers
cargo check -p localmt-cli --features ort-runtime
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 7: Commit**

Commit the doctor output update, docs, tests, and plan with the Lore commit protocol.
