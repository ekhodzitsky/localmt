# FFI Startup Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one C ABI call and one CLI command that summarize the Android startup contract without requiring a model pack.

**Architecture:** Bump the FFI ABI to v7 and expose `localmt_ffi_startup_summary` with the existing output-buffer contract. The summary includes ABI version, max text length, Xiaomi 17 metadata, enabled/disabled feature flags, and stable language codes. The CLI routes `localmt ffi startup` through that FFI function so host smoke scripts dogfood the Android-visible ABI.

**Tech Stack:** Rust FFI crate, existing C header, Rust CLI crate, current in-file tests.

---

### Task 1: Lock FFI And CLI Contract

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add failing FFI tests**

Add tests for `localmt_ffi_startup_summary`:

```rust
#[test]
fn ffi_startup_summary_reports_android_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = [0_u8; 512];
    let mut written_len = 0_usize;

    assert_eq!(
        localmt_ffi_startup_summary(output.as_mut_ptr(), output.len(), &mut written_len),
        LOCALMT_FFI_OK
    );
    let summary = std::str::from_utf8(&output[..written_len])?;

    assert!(summary.contains("ffi_abi: 7"));
    assert!(summary.contains("max_text_chars: 4096"));
    assert!(summary.contains("xiaomi17_android_abi: arm64-v8a"));
    assert!(summary.contains("xiaomi17_ram_class_gib: 12"));
    assert!(summary.contains("xiaomi17_preferred_runtime: onnx-runtime-mobile-xnnpack"));
    assert!(summary.contains("languages: en, ru, th, vi, ja"));
    Ok(())
}
```

Add buffer-too-small and null-pointer tests with the same status semantics as `localmt_ffi_status_message`.

- [x] **Step 2: Add failing CLI tests**

Add a test that calls:

```rust
localmt ffi startup
```

Assert the output contains:

```text
ffi_abi: 7
max_text_chars: 4096
languages: en, ru, th, vi, ja
```

Also assert top-level and FFI help include `localmt ffi startup`.

- [x] **Step 3: Run RED**

Run:

```bash
cargo test -p localmt-ffi startup_summary
cargo test -p localmt-cli ffi_startup
```

Expected: fail because the FFI symbol and CLI route do not exist.

### Task 2: Implement ABI v7 Summary

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 4: Bump ABI constants**

Change:

```rust
pub const LOCALMT_FFI_ABI_VERSION: u32 = 7;
```

and update the header define.

- [x] **Step 5: Add summary builder and export**

Add a pure helper:

```rust
fn ffi_startup_summary() -> String
```

Return stable newline text:

```text
ffi_abi: 7
max_text_chars: 4096
xiaomi17_android_abi: arm64-v8a
xiaomi17_ram_class_gib: 12
xiaomi17_preferred_runtime: onnx-runtime-mobile-xnnpack
hf_tokenizers: disabled
ort_runtime: disabled
languages: en, ru, th, vi, ja
```

`hf_tokenizers` and `ort_runtime` use `enabled` when the matching feature is compiled.

Export:

```rust
pub extern "C" fn localmt_ffi_startup_summary(
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32
```

It follows the same non-NUL-terminated output buffer contract as status messages.

- [x] **Step 6: Add CLI route**

Add `startup` under `localmt ffi`, calling the FFI export through `ffi_bytes` and returning UTF-8 text.

### Task 3: Docs, Verification, Commit

**Files:**
- Modify: `README.md`
- Modify: `docs/android-build.md`
- Modify: `docs/superpowers/plans/2026-05-07-localmt-ffi-startup-summary.md`

- [x] **Step 7: Update docs**

Document:

```bash
cargo run -p localmt -- ffi startup
```

Update Android startup flow to allow `localmt_ffi_startup_summary` before individual probes.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi startup_summary
cargo test -p localmt-cli ffi_startup
cargo test -p localmt-cli
cargo test -p localmt-cli --features hf-tokenizers
cargo check -p localmt-cli --features ort-runtime
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli` and `crates/localmt-ffi`.

- [x] **Step 9: Commit**

Commit the FFI export, CLI command, header, docs, tests, and plan with the Lore commit protocol.
