# localmt Tokenizer Asset Plan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a tokenizer asset planning boundary that connects verified model
packs to future tokenizer implementations without parsing tokenizer formats yet.

**Architecture:** `localmt-tokenizer` depends on `localmt-models` only for
verified pack role-path resolution. `TokenizerAssetPlan::from_pack` requires a
declared `tokenizer` role and carries optional `vocab` and `config` paths.
`MockTokenizer` remains dependency-free at runtime behavior level and does not
read model files.

**Tech Stack:** Rust 1.95, existing `localmt-tokenizer` and `localmt-models`;
no new external dependencies.

---

## Files

- Modify: `crates/localmt-tokenizer/Cargo.toml`
- Modify: `crates/localmt-tokenizer/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Asset-Plan Tests

- [x] **Step 1: Resolve required and optional paths**

Prove `TokenizerAssetPlan` resolves `tokenizer`, `vocab`, and `config` paths
from a verified pack.

- [x] **Step 2: Reject missing tokenizer asset**

Prove asset planning rejects a verified pack that has no `tokenizer` role.

### Task 2: Implementation

- [x] **Step 1: Add tokenizer asset plan**

Implement `TokenizerAssetPlan::from_pack`, path accessors, and
`TokenizerError::MissingTokenizerAsset`.

- [x] **Step 2: Re-export facade API**

Expose `TokenizerAssetPlan` from the `localmt` facade.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document tokenizer asset planning as a pre-parser boundary.

- [x] **Step 2: Update design spec**

Record the tokenizer asset planning contract and acceptance criterion.

### Task 4: Verification

- [x] **Step 1: Run targeted tests**

Run `cargo test -p localmt-tokenizer tokenizer_asset_plan`.

- [x] **Step 2: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 3: Run Kimi checks**

Run `cargo kimi check` in `localmt-tokenizer` and `localmt`.
