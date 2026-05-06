# localmt Model Pack Role Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Expose one verified model-pack API for resolving manifest file roles
to root-qualified file paths.

**Architecture:** Keep path validation and duplicate-role rejection inside
`localmt-models`. `ModelPack<Verified>::file_path(role)` returns `Some(PathBuf)`
only for manifest-declared roles. ORT session planning consumes this API instead
of scanning manifest files directly.

**Tech Stack:** Rust 1.95, existing `localmt-models` and `localmt-engine-ort`;
no new dependencies.

---

## Files

- Modify: `crates/localmt-models/src/lib.rs`
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Role-Path Test

- [x] **Step 1: Add verified pack path test**

Prove a verified pack resolves the `tokenizer` role to its root-qualified path
and returns `None` for an absent optional role.

### Task 2: Implementation

- [x] **Step 1: Add role-path API**

Implement `ModelPack<Verified>::file_path(ModelFileRole) -> Option<PathBuf>`.

- [x] **Step 2: Reuse role-path API in ORT planner**

Replace ORT's local manifest scan with `pack.file_path(role.model_file_role())`.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document verified role-path resolution.

- [x] **Step 2: Update design spec**

Record the adapter contract and acceptance criterion.

### Task 4: Verification

- [x] **Step 1: Run targeted tests**

Run the new model-pack role-path test and the ORT encoder planning test.

- [x] **Step 2: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 3: Run Kimi checks**

Run `cargo kimi check` in each changed member crate.
