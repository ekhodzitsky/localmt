# localmt Model File Roles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace free-form manifest file kinds with typed model file roles
that backend adapters can select without string matching.

**Architecture:** `localmt-models` owns the `ModelFileRole` enum and parses
`files[].kind` into a role during discovery. Unknown and duplicate roles are
manifest errors. `localmt-engine-ort` maps its own `OrtModelRole` to
`ModelFileRole` before selecting a graph file.

**Tech Stack:** Rust 1.95, existing `localmt-models`, no new dependencies.

---

## Files

- Modify: `crates/localmt-models/src/lib.rs`
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Role Tests

- [x] **Step 1: Expose typed roles**

Add tests proving discovered files expose `ModelFileRole` values.

- [x] **Step 2: Reject invalid roles**

Add tests proving unknown `files[].kind` values fail manifest discovery.

- [x] **Step 3: Reject duplicate roles**

Add tests proving repeated model file roles fail manifest discovery.

### Task 2: Implementation

- [x] **Step 1: Add `ModelFileRole`**

Support `encoder`, `decoder`, `decoder_with_past`, `tokenizer`, `vocab`,
`config`, and `generation_config`.

- [x] **Step 2: Parse roles during discovery**

Convert raw manifest strings to typed roles before checksum verification.

- [x] **Step 3: Update ORT selection**

Use `ModelFileRole` equality instead of string comparisons in
`localmt-engine-ort`.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document the supported role vocabulary and duplicate-role rule.

- [x] **Step 2: Update design spec**

Record typed file roles as part of the model-pack contract.

### Task 4: Verification

- [x] **Step 1: Run workspace checks**

Run `cargo fmt --check`, `cargo test`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 2: Run Kimi checks**

Run `cargo kimi check` in each member crate.
