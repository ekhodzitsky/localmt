# Android FFI Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimal C ABI crate for Android/JNI adapters to query the Rust library contract without internet access or new dependencies.

**Architecture:** Add `crates/localmt-ffi` as an adapter crate around the `localmt` facade. Keep the first ABI pointer-free: language ids, two-byte ISO language codes, language-pair validation, max text length, ABI version, and Xiaomi 17 target metadata. This gives Android code a stable low-risk bridge while string/model-pack handle APIs are designed later.

**Tech Stack:** Rust workspace, `localmt-ffi` cdylib/staticlib/rlib crate, std-only tests, C header.

---

### Task 1: Lock FFI Contract

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/localmt-ffi/Cargo.toml`
- Create: `crates/localmt-ffi/src/lib.rs`

- [x] **Step 1: Add failing tests**

Add tests for:
- supported language count and language-code mapping
- two-byte ISO code parsing
- same-language pair rejection and invalid language rejection
- ABI version, max text length, and Xiaomi 17 metadata

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-ffi
```

Expected: fail because exported FFI functions are not implemented yet.

### Task 2: Implement Pointer-Free C ABI

**Files:**
- Modify: `crates/localmt-ffi/src/lib.rs`
- Create: `crates/localmt-ffi/include/localmt_ffi.h`

- [x] **Step 3: Add exported constants and structs**

Add:
- status codes: OK, invalid language, invalid pair
- ABI version
- `LocalmtFfiLanguageCode { first, second }`

- [x] **Step 4: Add exported functions**

Add `extern "C"` functions for:
- supported language count
- language id to two-byte ISO code
- two-byte ISO code to language id
- language pair validation
- max text chars
- Xiaomi 17 ABI/RAM/runtime profile codes

- [x] **Step 5: Add C header**

Document the pointer-free ABI in `include/localmt_ffi.h`.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-ffi
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-android-ffi-boundary.md`

- [x] **Step 7: Update README**

Document the `localmt-ffi` crate and its pointer-free first ABI scope.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-ffi
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-ffi`.

- [x] **Step 9: Commit**

Commit the FFI crate, tests, header, and docs with the Lore commit protocol.
