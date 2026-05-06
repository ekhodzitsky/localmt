# localmt ORT Engine Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a narrow ONNX Runtime boundary that can plan a session from a
verified model pack and keep real runtime loading behind an explicit feature.

**Architecture:** `localmt-engine-ort` owns ORT-specific planning and loading.
Default builds do not initialize ONNX Runtime. The optional `ort-runtime`
feature compiles real session creation using `Session::builder()` and
`commit_from_file`.

**Tech Stack:** Rust 1.95, existing `localmt-models`, optional `ort`
`2.0.0-rc.12` with default features disabled.

---

## Files

- Modify: `Cargo.toml`
- Create: `crates/localmt-engine-ort/Cargo.toml`
- Create: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/Cargo.toml`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing ORT Boundary Tests

- [x] **Step 1: Add session-plan tests**

Prove that a verified ONNX Runtime pack selects the `encoder` file path for
`OrtModelRole::Encoder`.

- [x] **Step 2: Add rejection tests**

Prove that session planning rejects missing role files and non-ORT runtimes.

- [x] **Step 3: Add default-load test**

Prove that default builds return `OrtRuntimeFeatureDisabled` instead of trying
to create an ONNX Runtime session.

### Task 2: ORT Boundary Implementation

- [x] **Step 1: Add role and plan types**

Implement `OrtModelRole` and `OrtSessionPlan` with localmt-owned public API.

- [x] **Step 2: Add feature-gated engine**

Implement default disabled loading and `ort-runtime` session loading.

- [x] **Step 3: Keep `ort` private**

Expose localmt-owned counts and plans, not `ort` runtime types, through public
facade API.

### Task 3: Facade And Docs

- [x] **Step 1: Re-export localmt-owned ORT boundary types**

Expose `OrtEngine`, `OrtEngineError`, `OrtModelRole`, and `OrtSessionPlan` from
the facade crate.

- [x] **Step 2: Document current boundary**

Document that the crate currently stops at session loading, with tokenization
and graph composition still pending.

### Task 4: Verification

- [x] **Step 1: Run workspace checks**

Run `cargo fmt --check`, `cargo test`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 2: Run feature check**

Run `cargo check -p localmt-engine-ort --features ort-runtime`.

- [x] **Step 3: Run Kimi checks**

Run `cargo kimi check` in each member crate.
