# ORT Token Generator Trait Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `OrtTokenGenerator` satisfy the pipeline `TokenGenerator` contract while real ONNX token generation remains intentionally unavailable.

**Architecture:** Implement `TokenGenerator` in `localmt-engine-ort`, using the existing tokenizer-owned `TokenizerOutput` and `TokenSequence` types. The implementation returns `BackendUnavailable` until encoder/decoder tensor I/O and decoding semantics are added.

**Tech Stack:** Rust workspace, existing `localmt-pipeline`, `localmt-tokenizer`, and `localmt-engine-ort` crates. No external dependencies.

---

### Task 1: Lock The Contract

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write the failing test**

Add a default-build facade test that constructs a `TokenizerOutput`, calls `TokenGenerator::generate(&OrtTokenGenerator, &input)`, and expects `TokenGeneratorError::BackendUnavailable("ONNX token generation loop is not implemented")`.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt ort_token_generator_generate_reports_unimplemented_backend`

Expected: compile failure because `OrtTokenGenerator` does not implement `TokenGenerator`.

### Task 2: Implement The Placeholder Generator

**Files:**
- Modify: `crates/localmt-engine-ort/Cargo.toml`
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 3: Add explicit tokenizer type dependency**

Add `localmt-tokenizer` as a local dependency because the trait implementation names `TokenizerOutput` and `TokenSequence`.

- [x] **Step 4: Implement `TokenGenerator`**

Return `TokenGeneratorError::BackendUnavailable("ONNX token generation loop is not implemented")` from `OrtTokenGenerator::generate`.

- [x] **Step 5: Run GREEN**

Run: `cargo test -p localmt ort_token_generator_generate_reports_unimplemented_backend`

Expected: the test passes.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-ort-token-generator-trait.md`

- [x] **Step 6: Update docs**

Document that `OrtTokenGenerator` now satisfies the pipeline trait but returns an explicit unavailable-backend error until real token generation is implemented.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt ort_token_generator_generate_reports_unimplemented_backend
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt-engine-ort --features ort-runtime
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-engine-ort` and `crates/localmt`.

- [x] **Step 8: Commit**

Commit the trait implementation, test, and docs with the Lore commit protocol.
