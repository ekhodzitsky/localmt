# localmt ORT Generator Plan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Connect the ONNX Runtime boundary to the pipeline-owned generator
asset contract without executing ONNX inference yet.

**Architecture:** `localmt-engine-ort` depends on `localmt-pipeline` for
`GeneratorAssetPlan`. `OrtGeneratorPlan::from_pack` verifies the model runtime,
uses `GeneratorAssetPlan::from_pack`, and converts encoder/decoder assets into
`OrtSessionPlan`s. Optional `decoder_with_past` becomes an optional session
plan; optional `generation_config` remains a verified path.

**Tech Stack:** Rust 1.95, existing `localmt-engine-ort`,
`localmt-pipeline`, and `localmt-models`; no new external dependencies.

---

## Files

- Modify: `crates/localmt-engine-ort/Cargo.toml`
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing ORT Generator Plan Tests

- [x] **Step 1: Resolve required and optional ORT assets**

Prove `OrtGeneratorPlan` selects encoder, decoder, optional
`decoder_with_past`, and optional `generation_config` from a verified ONNX pack.

- [x] **Step 2: Reject non-ORT runtime**

Prove generator planning rejects a verified pack whose runtime is not
`onnx-runtime`.

- [x] **Step 3: Reject missing decoder**

Prove missing required generator assets are surfaced through `OrtEngineError`.

### Task 2: Implementation

- [x] **Step 1: Expand ORT roles**

Add decoder and cached-decoder roles to `OrtModelRole`.

- [x] **Step 2: Add ORT generator plan**

Implement `OrtGeneratorPlan::from_pack` and accessors over generated session
plans and generation config path.

- [x] **Step 3: Re-export facade API**

Expose `OrtGeneratorPlan` from `localmt`.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document ORT generator planning as pre-inference planning.

- [x] **Step 2: Update design spec**

Record the ORT generator planning contract and acceptance criterion.

### Task 4: Verification

- [x] **Step 1: Run targeted tests**

Run `cargo test -p localmt-engine-ort generator_plan`.

- [x] **Step 2: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 3: Run ORT feature checks**

Run `cargo check -p localmt-engine-ort --features ort-runtime` and
`cargo check -p localmt --features ort-runtime`.

- [x] **Step 4: Run Kimi checks**

Run `cargo kimi check` in `localmt-engine-ort` and `localmt`.
