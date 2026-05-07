# Facade ORT I/O Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the ORT I/O tensor-name contract in facade, CLI, and FFI preflight summaries without requiring real ONNX generation.

**Architecture:** Add a facade-owned `OrtIoConfigStatus` enum so `config` absent, valid `ort_io`, and `config` present without `ort_io` are distinguishable. Keep invalid or malformed `ort_io` as an error, but report missing `ort_io` as a readiness status because current packs may still carry ordinary Hugging Face `config.json`.

**Tech Stack:** Rust facade crate, existing ORT parser, CLI/FFI preflight summaries, TDD.

---

### Task 1: Add Facade Tests

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests for:

```rust
offline_translator_plan_reports_missing_ort_io_config_object
offline_translator_assets_summary_reports_ort_io_config
```

- [x] **Step 2: Verify RED**

Run: `cargo test -p localmt ort_io_config`

Expected: compile failure because `OrtIoConfigStatus`, `parse_ort_io_config_status`, and summary accessors do not exist.

### Task 2: Implement Facade Status

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Re-export ORT config types**

Expose `OrtIoConfig`, `OrtIoConfigError`, and `OrtIoConfigParseError` from the facade.

- [x] **Step 2: Add status enum and parsing method**

Implement `OrtIoConfigStatus::{Absent, Missing, Parsed(OrtIoConfig)}` and `OfflineTranslatorPlan::parse_ort_io_config_status`.

- [x] **Step 3: Carry status through prepared assets and summaries**

Add `ort_io_config_status` to `OfflineTranslatorAssets` and `OfflineTranslatorAssetsSummary`, and print stable summary lines.

- [x] **Step 4: Verify GREEN**

Run: `cargo test -p localmt ort_io_config`

Expected: new facade tests pass.

### Task 3: Update CLI/FFI Coverage And Docs

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `crates/localmt-ffi/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/android-build.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

- [x] **Step 1: Extend assertions**

Update existing model-plan and FFI summary tests to assert the new `ort_io_config` summary line.

- [x] **Step 2: Update docs**

Document `ort_io_config: parsed|absent|missing` in preflight outputs.

- [x] **Step 3: Verify**

Run:

```bash
cargo fmt --check
cargo test -p localmt ort_io_config
cargo test -p localmt-cli model_plan
cargo test -p localmt-ffi model_pack_summary
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

### Task 4: Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run Kimi**

Run crate-level `cargo kimi check` from changed crates.

- [ ] **Step 2: Commit with Lore protocol**

Create a commit explaining why preflight summaries now expose ORT I/O readiness before real generation.
