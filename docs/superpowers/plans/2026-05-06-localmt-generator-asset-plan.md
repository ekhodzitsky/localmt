# localmt Generator Asset Plan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a token-generator asset planning boundary that connects verified
model packs to future ONNX-backed generation without executing ONNX yet.

**Architecture:** `localmt-pipeline` owns `TokenGenerator`, so it also owns the
generator asset contract. `GeneratorAssetPlan::from_pack` requires verified
`encoder` and `decoder` roles, preserves optional `decoder_with_past` and
`generation_config` paths, and uses `ModelPack<Verified>::file_path`.

**Tech Stack:** Rust 1.95, existing `localmt-pipeline` and `localmt-models`; no
new external dependencies.

---

## Files

- Modify: `crates/localmt-pipeline/Cargo.toml`
- Modify: `crates/localmt-pipeline/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Generator Asset Tests

- [x] **Step 1: Resolve required and optional paths**

Prove `GeneratorAssetPlan` resolves `encoder`, `decoder`,
`decoder_with_past`, and `generation_config` paths from a verified pack.

- [x] **Step 2: Reject missing encoder**

Prove planning rejects a verified pack without an `encoder` role.

- [x] **Step 3: Reject missing decoder**

Prove planning rejects a verified pack without a `decoder` role.

### Task 2: Implementation

- [x] **Step 1: Add generator asset plan**

Implement `GeneratorAssetPlan::from_pack`, accessors, and
`TokenGeneratorError::MissingGeneratorAsset`.

- [x] **Step 2: Re-export facade API**

Expose `GeneratorAssetPlan` from the `localmt` facade.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document generator asset planning as the pre-ONNX boundary.

- [x] **Step 2: Update design spec**

Record the generator asset planning contract and acceptance criterion.

### Task 4: Verification

- [x] **Step 1: Run targeted tests**

Run `cargo test -p localmt-pipeline generator_asset_plan`.

- [x] **Step 2: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 3: Run Kimi checks**

Run `cargo kimi check` in `localmt-pipeline` and `localmt`.
