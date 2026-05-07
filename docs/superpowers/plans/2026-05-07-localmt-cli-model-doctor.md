# CLI Model Doctor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `localmt model doctor PACK`, a single local readiness report for a verified model pack and the Android-facing smoke paths.

**Architecture:** Extend the existing `model` command group with a no-network diagnostic command. The command verifies facade planning, checks the shared FFI model-pack summary, runs the deterministic mock FFI translation, and reports HF tokenizer / ORT runtime preflight statuses without treating compile-time-disabled features as command failures.

**Tech Stack:** Rust CLI crate, existing `localmt` facade planning, existing `localmt-ffi` C ABI helpers, current in-file CLI test fixtures.

---

### Task 1: Lock CLI Contract

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add failing tests**

Add tests that call:

```rust
localmt model doctor <pack>
```

For the default build, assert the output contains:

```text
doctor: ok
model_plan: ok
ffi_model_pack_summary: ok
ffi_mock_translate: ok
ffi_hf_mock_translate: tokenizer disabled
ffi_ort_generator: runtime disabled
```

Also assert top-level and model help include:

```text
localmt model doctor PACK
```

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli model_doctor
cargo test -p localmt-cli cli_prints_help
```

Expected: tests fail because `doctor` is not routed under `localmt model` and help does not mention it.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 3: Add model routing**

Add `doctor` to `run_model`:

```rust
"doctor" => doctor_model(single_model_path(args)?),
```

- [x] **Step 4: Add diagnostic helpers**

Add a small status formatter that accepts selected FFI statuses:

```rust
fn ffi_status_label(status: i32) -> String {
    ffi_status_text(status)
}
```

Use the existing `ffi_bytes`, `ffi_language_id`, and FFI close calls. Do not introduce new dependencies.

- [x] **Step 5: Implement doctor**

`doctor_model(path)` should:

1. Call `OfflineTranslatorAssets::from_model_pack_path(path.clone())`.
2. Call `localmt_ffi_model_pack_summary`.
3. Open/translate/close `LocalmtFfiTranslator` using `en -> ru` and `hello offline`.
4. Call `localmt_ffi_hf_mock_translator_open`.
5. If HF open succeeds, call `localmt_ffi_hf_mock_translate` and close the handle.
6. If HF open returns `LOCALMT_FFI_TOKENIZER_DISABLED`, report `tokenizer disabled`.
7. Call `localmt_ffi_ort_generator_open`.
8. If ORT open succeeds, close the handle.
9. If ORT open returns `LOCALMT_FFI_RUNTIME_DISABLED`, report `runtime disabled`.

Return an error for any other unexpected FFI status.

### Task 3: Verify And Commit

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo fmt
cargo test -p localmt-cli model_doctor
cargo test -p localmt-cli cli_prints_help
```

Expected: pass.

- [x] **Step 7: Run standard verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli model_doctor
cargo test -p localmt-cli cli_prints_help
cargo test -p localmt-cli
cargo test -p localmt-cli --features hf-tokenizers
cargo check -p localmt-cli --features ort-runtime
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the CLI command, docs, tests, and plan with the Lore commit protocol.
