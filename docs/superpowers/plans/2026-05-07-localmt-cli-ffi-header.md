# CLI FFI Header Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let developers print the bundled C ABI header with `localmt ffi header`.

**Architecture:** Reuse the existing checked-in `crates/localmt-ffi/include/localmt_ffi.h` as the single source. The CLI embeds that file with `include_str!`, routes `localmt ffi header`, and documents the command in the Android build notes.

**Tech Stack:** Rust CLI crate, existing localmt-ffi header, current in-file CLI tests.

---

### Task 1: Lock CLI Contract

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add failing tests**

Add a test that calls:

```rust
localmt ffi header
```

Assert the output contains:

```text
#ifndef LOCALMT_FFI_H
#define LOCALMT_FFI_ABI_VERSION 6
int32_t localmt_ffi_model_pack_summary(
int32_t localmt_ffi_mock_translate(
int32_t localmt_ffi_ort_generator_open(
```

Also assert top-level and FFI help include:

```text
localmt ffi header
```

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli ffi_header
cargo test -p localmt-cli cli_prints_help
```

Expected: fail because `header` is not routed under `localmt ffi` and help does not mention it.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 3: Embed the header**

Add:

```rust
const FFI_HEADER_TEXT: &str = include_str!("../../localmt-ffi/include/localmt_ffi.h");
```

- [x] **Step 4: Add FFI routing**

Add `header` to `run_ffi`:

```rust
"header" => ffi_header(args),
```

`ffi_header` accepts no extra arguments and returns `FFI_HEADER_TEXT.to_owned()`.

- [x] **Step 5: Update docs**

Document:

```bash
cargo run -p localmt -- ffi header > localmt_ffi.h
```

Clarify that Android/NDK consumers can use the checked-in header directly or print it from the CLI.

### Task 3: Verify And Commit

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-cli ffi_header
cargo test -p localmt-cli cli_prints_help
```

Expected: pass.

- [x] **Step 7: Run standard verification**

Run:

```bash
cargo fmt --check
git diff --check
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

- [x] **Step 8: Commit**

Commit the CLI header command, docs, tests, and plan with the Lore commit protocol.
