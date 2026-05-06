# FFI Model Pack Summary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Android/JNI adapters verify a local model pack and read the prepared asset summary through the C ABI without loading tokenizer or ONNX runtime sessions.

**Architecture:** Add `localmt_ffi_model_pack_summary` to `localmt-ffi` and bump the C ABI to v5. The function reuses `OfflineTranslatorAssets::from_model_pack_path` and formats the same stable newline summary as the CLI `model plan` command. It follows the existing UTF-8 output-buffer contract: `written_len` always reports the required byte count after successful planning, and `LOCALMT_FFI_BUFFER_TOO_SMALL` means output was not written.

**Tech Stack:** Rust FFI crate, existing facade summary API, C header declarations, README docs.

---

### Task 1: Lock Summary FFI Contract

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Add failing tests**

Add tests to `mod tests` in `crates/localmt-ffi/src/lib.rs`:

```rust
#[test]
fn ffi_model_pack_summary_reports_verified_assets() -> Result<(), Box<dyn std::error::Error>> {
    let root = create_verified_pack()?;
    let path = path_bytes(&root)?;
    let mut output = [0_u8; 512];
    let mut written_len = 0_usize;

    assert_eq!(
        localmt_ffi_model_pack_summary(
            path.as_ptr(),
            path.len(),
            output.as_mut_ptr(),
            output.len(),
            &mut written_len,
        ),
        LOCALMT_FFI_OK
    );

    let summary = std::str::from_utf8(&output[..written_len])?;
    assert!(summary.contains("planned: m2m100-418m-int8"));
    assert!(summary.contains("tokenizer:"));
    assert!(summary.contains("encoder:"));
    assert!(summary.contains("decoder:"));
    assert!(summary.contains("decoder_with_past: absent"));
    assert!(summary.contains("generation_config: absent"));
    Ok(())
}
```

Also add tests for:
- small output buffer returns `LOCALMT_FFI_BUFFER_TOO_SMALL` and required byte count
- null `written_len`, null path, and invalid UTF-8 path map to existing status codes
- non-pack path maps to `LOCALMT_FFI_MODEL_PACK_ERROR`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi model_pack_summary
```

Expected: fail because `localmt_ffi_model_pack_summary` does not exist.

### Task 2: Implement Summary API

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `crates/localmt-ffi/include/localmt_ffi.h`
- Modify: `README.md`

- [x] **Step 3: Add ABI v5 and implementation**

Change `LOCALMT_FFI_ABI_VERSION` from `4` to `5`.

Add:

```rust
/// { path_ptr points to path_len readable bytes and output/written pointers follow the header contract }
/// fn localmt_ffi_model_pack_summary(path_ptr: *const u8, path_len: usize, output_ptr: *mut u8, output_capacity: usize, written_len: *mut usize) -> i32
/// { ret is OK only when output receives written_len UTF-8 summary bytes }
#[unsafe(no_mangle)] // SAFETY: FFI export validates raw pointers before use.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn localmt_ffi_model_pack_summary(
    path_ptr: *const u8,
    path_len: usize,
    output_ptr: *mut u8,
    output_capacity: usize,
    written_len: *mut usize,
) -> i32 {
    if written_len.is_null() {
        return LOCALMT_FFI_NULL_POINTER;
    }

    unsafe { *written_len = 0 }; // SAFETY: non-null writable length pointer.

    let path = match read_ffi_utf8(path_ptr, path_len) {
        Ok(value) => value,
        Err(status) => return status,
    };
    let assets = match OfflineTranslatorAssets::from_model_pack_path(path) {
        Ok(value) => value,
        Err(_error) => return LOCALMT_FFI_MODEL_PACK_ERROR,
    };
    let summary = format_assets_summary(&assets);
    let output = summary.as_bytes();

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

Add a private helper:

```rust
/// { assets were prepared successfully }
/// fn format_assets_summary(assets: &OfflineTranslatorAssets) -> String
/// { ret is the stable newline summary exposed through CLI and FFI }
fn format_assets_summary(assets: &OfflineTranslatorAssets) -> String {
    let summary = assets.summary();
    let decoder_with_past = summary
        .decoder_with_past_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "absent".to_owned());
    let mut lines = vec![
        format!("planned: {}", summary.model_id()),
        format!("tokenizer: {}", summary.tokenizer_path().display()),
        format!("encoder: {}", summary.encoder_path().display()),
        format!("decoder: {}", summary.decoder_path().display()),
        format!("decoder_with_past: {decoder_with_past}"),
    ];

    match summary.generation_config() {
        Some(config) => {
            lines.push("generation_config: parsed".to_owned());
            lines.push(format!(
                "max_new_tokens: {}",
                config.max_new_tokens().value()
            ));
        }
        None => lines.push("generation_config: absent".to_owned()),
    }

    lines.join("\n")
}
```

- [x] **Step 4: Update C header and README**

Document:
- ABI v5
- `localmt_ffi_model_pack_summary`
- the function verifies and plans the pack but does not load tokenizer or ORT sessions

- [x] **Step 5: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-ffi model_pack_summary
```

Expected: pass.

### Task 3: Verify And Commit

- [x] **Step 6: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi model_pack_summary
cargo test -p localmt-ffi
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 7: Commit**

Commit the FFI summary API, tests, docs, header, and plan with the Lore commit protocol.
