# FFI Status Messages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Android/JNI adapters turn stable FFI status codes into stable UTF-8 messages without duplicating Rust status mapping.

**Architecture:** Add `localmt_ffi_status_message` to `localmt-ffi` and bump the C ABI to v6. The function accepts any `int32_t` status and writes a short ASCII/UTF-8 message through the existing output-buffer contract. Known statuses get stable messages; unknown status values return the message `unknown status` while the message call itself succeeds.

**Tech Stack:** Rust FFI crate, C header declarations, README docs.

---

### Task 1: Lock Status Message Contract

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Add failing tests**

Add tests to `mod tests` in `crates/localmt-ffi/src/lib.rs`:

```rust
#[test]
fn ffi_status_message_reports_known_status_text() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = [0_u8; 64];
    let mut written_len = 0_usize;

    assert_eq!(
        localmt_ffi_status_message(
            LOCALMT_FFI_MODEL_PACK_ERROR,
            output.as_mut_ptr(),
            output.len(),
            &mut written_len,
        ),
        LOCALMT_FFI_OK
    );
    assert_eq!(
        std::str::from_utf8(&output[..written_len])?,
        "model pack error"
    );
    Ok(())
}
```

Also add tests for:
- unknown status writes `unknown status`
- small buffer returns `LOCALMT_FFI_BUFFER_TOO_SMALL` and required byte count
- null `written_len` and null output map to `LOCALMT_FFI_NULL_POINTER`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi status_message
```

Expected: fail because `localmt_ffi_status_message` does not exist.

### Task 2: Implement Status Message API

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`
- Modify: `README.md`

- [x] **Step 3: Add ABI v6 and status mapping**

Change `LOCALMT_FFI_ABI_VERSION` from `5` to `6`.

Add:

```rust
/// { status may be any i32 }
/// fn ffi_status_message(status: i32) -> &'static str
/// { ret is a stable short message for known status codes, otherwise unknown status }
const fn ffi_status_message(status: i32) -> &'static str {
    match status {
        LOCALMT_FFI_OK => "ok",
        LOCALMT_FFI_INVALID_LANGUAGE => "invalid language",
        LOCALMT_FFI_INVALID_PAIR => "invalid language pair",
        LOCALMT_FFI_NULL_POINTER => "null pointer",
        LOCALMT_FFI_INVALID_UTF8 => "invalid utf-8",
        LOCALMT_FFI_MODEL_PACK_ERROR => "model pack error",
        LOCALMT_FFI_TEXT_ERROR => "text error",
        LOCALMT_FFI_TRANSLATION_ERROR => "translation error",
        LOCALMT_FFI_BUFFER_TOO_SMALL => "buffer too small",
        LOCALMT_FFI_RUNTIME_DISABLED => "runtime disabled",
        LOCALMT_FFI_ORT_ERROR => "onnx runtime error",
        LOCALMT_FFI_TOKENIZER_DISABLED => "tokenizer disabled",
        LOCALMT_FFI_TOKENIZER_ERROR => "tokenizer error",
        _ => "unknown status",
    }
}
```

Add:

```rust
/// { output/written pointers follow the header contract }
/// fn localmt_ffi_status_message(status: i32, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 message bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_status_message(
    status: i32,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    let output = ffi_status_message(status).as_bytes();
    unsafe { *written_len = output.len() }; // SAFETY: non-null writable length pointer.

    if output_capacity < output.len() {
        return LOCALMT_FFI_BUFFER_TOO_SMALL;
    }
    if output_ptr.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len()) }; // SAFETY: output buffer capacity was checked.

    LOCALMT_FFI_OK
}
```

- [x] **Step 4: Update C header and README**

Document:
- ABI v6
- `localmt_ffi_status_message`
- unknown input status is handled as `unknown status`

- [x] **Step 5: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-ffi status_message
```

Expected: pass.

### Task 3: Verify And Commit

- [x] **Step 6: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi status_message
cargo test -p localmt-ffi
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 7: Commit**

Commit the status-message API, tests, docs, header, and plan with the Lore commit protocol.
