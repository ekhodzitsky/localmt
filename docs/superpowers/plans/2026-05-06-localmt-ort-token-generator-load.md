# localmt ORT Token Generator Load Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add the ONNX Runtime token-generator load boundary without
implementing the generation loop.

**Architecture:** `OrtTokenGenerator::load` consumes `OrtGeneratorPlan`.
Default builds return `OrtRuntimeFeatureDisabled`. With `ort-runtime`, it loads
required encoder and decoder sessions, loads optional cached decoder when
present, and exposes loaded-session accessors for the future generation loop.

**Tech Stack:** Rust 1.95, existing `localmt-engine-ort`,
`localmt-pipeline`, and optional `ort-runtime`; no new dependencies.

---

## Files

- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Load Test

- [x] **Step 1: Disabled build behavior**

Prove `OrtTokenGenerator::load(OrtGeneratorPlan)` returns
`OrtRuntimeFeatureDisabled` when built without `ort-runtime`.

### Task 2: Implementation

- [x] **Step 1: Add disabled loader**

Implement the default-build `OrtTokenGenerator` load boundary.

- [x] **Step 2: Add feature-enabled loader**

Implement `ort-runtime` loading of encoder, decoder, and optional cached
decoder sessions from `OrtGeneratorPlan`.

- [x] **Step 3: Re-export facade API**

Expose `OrtTokenGenerator` from `localmt`.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document the feature-gated generator load boundary.

- [x] **Step 2: Update design spec**

Record that `OrtTokenGenerator::load` exists but generation remains future work.

### Task 4: Verification

- [x] **Step 1: Run targeted test**

Run `cargo test -p localmt-engine-ort token_generator_load_returns_feature_disabled_without_ort_runtime`.

- [x] **Step 2: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 3: Run ORT feature checks**

Run `cargo check -p localmt-engine-ort --features ort-runtime` and
`cargo check -p localmt --features ort-runtime`.

- [x] **Step 4: Run Kimi checks**

Run `cargo kimi check` in `localmt-engine-ort` and `localmt`.
