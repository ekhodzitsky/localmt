# localmt Pipeline Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add the first end-to-end translation pipeline shape without real
model inference.

**Architecture:** `localmt-pipeline` composes `TokenizerEngine` and
`TokenGenerator`. The pipeline exposes a direct `translate_pipeline` method and
implements the existing `TranslatorEngine` trait so future real generators can
slot into the public SDK without reshaping callers.

**Tech Stack:** Rust 1.95, existing `localmt-core`, `localmt-engine`, and
`localmt-tokenizer`; no new external dependencies.

---

## Files

- Modify: `Cargo.toml`
- Create: `crates/localmt-pipeline/Cargo.toml`
- Create: `crates/localmt-pipeline/src/lib.rs`
- Modify: `crates/localmt/Cargo.toml`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Pipeline Tests

- [x] **Step 1: Mock end-to-end pipeline**

Prove `MockTokenizer + MockTokenGenerator` returns a non-empty `Translation`.

- [x] **Step 2: TranslatorEngine integration**

Prove the pipeline can be called through the existing `TranslatorEngine` trait.

- [x] **Step 3: Pair and error behavior**

Prove the generator sees source/target languages and generator errors surface
through pipeline and `TranslatorEngine` mappings.

### Task 2: Pipeline Implementation

- [x] **Step 1: Add generator contract**

Implement `TokenGenerator`, `TokenGeneratorError`, and `MockTokenGenerator`.

- [x] **Step 2: Add pipeline type**

Implement `TranslationPipeline<T, G>` over `TokenizerEngine` and
`TokenGenerator`.

- [x] **Step 3: Implement TranslatorEngine**

Map `PipelineError` into the existing `TranslationError` shape.

### Task 3: Facade And Docs

- [x] **Step 1: Re-export pipeline API**

Expose pipeline types from `localmt`.

- [x] **Step 2: Document current boundary**

Document that the current generator is still mock behavior and real ONNX-backed
generation remains future work.

### Task 4: Verification

- [x] **Step 1: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 2: Run Kimi checks**

Run `cargo kimi check` in each member crate.
