# FFI Runtime Config Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the strict ORT runtime config readiness gate through the Android C ABI.

**Architecture:** Add `localmt_ffi_runtime_config_summary` beside `localmt_ffi_model_pack_summary`. It verifies and plans a pack, requires parsed `generation_config` plus `ort_io`, writes stable UTF-8 summary text through the existing output-buffer ABI, and never loads tokenizer backends or ONNX sessions. Adding a new exported symbol bumps `LOCALMT_FFI_ABI_VERSION` from 7 to 8 and updates the bundled header plus CLI smoke surface.

**Tech Stack:** Rust `localmt-ffi`, existing facade asset planning, existing ORT runtime config parser, C header, CLI FFI smoke helpers, TDD.

---

### Task 1: Add Failing FFI Tests

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests for:

```rust
ffi_runtime_config_summary_reports_strict_ort_contract
ffi_runtime_config_summary_reports_required_buffer_len
ffi_runtime_config_summary_rejects_nulls_and_invalid_utf8
ffi_reports_abi_and_xiaomi17_contract
```

The runtime-config summary test should call:

```rust
localmt_ffi_runtime_config_summary(
    path.as_ptr(),
    path.len(),
    output.as_mut_ptr(),
    output.len(),
    &mut written_len,
)
```

Expected summary snippets:

```text
runtime_config: ok
max_new_tokens: 32
encoder_input_ids: encoder_input_ids
decoder_input_ids: decoder_input_ids
decoder_logits: decoder_logits
```

The ABI test should expect `localmt_ffi_abi_version() == 8`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-ffi runtime_config_summary
cargo test -p localmt-ffi ffi_reports_abi_and_xiaomi17_contract
```

Expected: compile/test failure because `localmt_ffi_runtime_config_summary` does not exist and the ABI is still 7.

### Task 2: Implement FFI Runtime Config Summary

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Bump ABI**

Change:

```rust
pub const LOCALMT_FFI_ABI_VERSION: u32 = 8;
```

- [x] **Step 2: Add formatter helper**

Add:

```rust
fn runtime_config_summary(path: &str) -> Result<String, i32> {
    let assets = OfflineTranslatorAssets::from_model_pack_path(path)
        .map_err(|_error| LOCALMT_FFI_MODEL_PACK_ERROR)?;
    let config = assets
        .plan()
        .generator()
        .parse_runtime_config()
        .map_err(|_error| LOCALMT_FFI_MODEL_PACK_ERROR)?;

    Ok(format!(
        "runtime_config: ok\nmax_new_tokens: {}\nencoder_input_ids: {}\ndecoder_input_ids: {}\ndecoder_logits: {}",
        config.generation_config().max_new_tokens().value(),
        config.ort_io_config().encoder().input_ids(),
        config.ort_io_config().decoder().input_ids(),
        config.ort_io_config().decoder().logits(),
    ))
}
```

- [x] **Step 3: Add exported function**

Add:

```rust
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_runtime_config_summary(
    path_ptr: *const u8,
    path_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32
```

It must follow the same output-buffer behavior as `localmt_ffi_model_pack_summary`: set `written_len` to `0` before path parsing, set required length before buffer checks, return `LOCALMT_FFI_BUFFER_TOO_SMALL` for short output, and return `LOCALMT_FFI_NULL_POINTER` for null output after required length is known.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-ffi runtime_config_summary
cargo test -p localmt-ffi ffi_reports_abi_and_xiaomi17_contract
```

Expected: tests pass.

### Task 3: Update Header, CLI, And Docs

**Files:**
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 1: Update C header**

Set:

```c
#define LOCALMT_FFI_ABI_VERSION 8
```

Declare:

```c
int32_t localmt_ffi_runtime_config_summary(
    const uint8_t *path_ptr,
    size_t path_len,
    uint8_t *output_ptr,
    size_t output_capacity,
    size_t *written_len);
```

- [x] **Step 2: Add CLI FFI route**

Add `localmt ffi runtime-config PACK`, implemented by calling `localmt_ffi_runtime_config_summary` through the same buffer helper pattern used by `ffi smoke`.

- [x] **Step 3: Update CLI tests**

Update header/help tests to expect ABI 8 and the new symbol. Add `cli_ffi_runtime_config_reports_strict_ort_contract`, expecting:

```text
ffi_abi: 8
runtime_config_summary: ok
runtime_config: ok
decoder_logits: decoder_logits
```

- [x] **Step 4: Update docs**

Mention that Android should call `localmt_ffi_runtime_config_summary()` after `localmt_ffi_model_pack_summary()` and before ORT generator open.

### Task 4: Verify And Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run full verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi runtime_config_summary
cargo test -p localmt-ffi ffi_reports_abi_and_xiaomi17_contract
cargo test -p localmt-cli cli_ffi_runtime_config_reports_strict_ort_contract
cargo test -p localmt-cli cli_ffi_header_prints_bundled_header
cargo test -p localmt-ffi
cargo test -p localmt-cli
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

- [x] **Step 2: Run Kimi**

Run `cargo kimi check` from `crates/localmt-ffi` and `crates/localmt-cli`.

- [x] **Step 3: Commit with Lore protocol**

Create a commit explaining why strict ORT runtime readiness is now visible through the Android ABI before real generation wiring.
