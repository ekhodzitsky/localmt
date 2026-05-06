# localmt Tokenizer Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a model-agnostic tokenizer boundary so the future translation
pipeline can be tested before choosing a real SentencePiece/BPE dependency.

**Architecture:** `localmt-tokenizer` owns token ids, bounded non-empty token
sequences, tokenizer input/output types, a backend trait, and a deterministic
mock tokenizer. The public facade re-exports these types from `localmt`.

**Tech Stack:** Rust 1.95, existing `localmt-core`, no new external
dependencies.

---

## Files

- Modify: `Cargo.toml`
- Create: `crates/localmt-tokenizer/Cargo.toml`
- Create: `crates/localmt-tokenizer/src/lib.rs`
- Modify: `crates/localmt/Cargo.toml`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Tokenizer Tests

- [x] **Step 1: Input contract tests**

Prove tokenizer input preserves validated source, target, and text invariants.

- [x] **Step 2: Token sequence tests**

Prove empty and oversized token sequences are rejected.

- [x] **Step 3: Mock tokenizer tests**

Prove mock tokenizer UTF-8 roundtrip behavior and invalid UTF-8 rejection.

### Task 2: Tokenizer Implementation

- [x] **Step 1: Add token types**

Implement `TokenId`, `TokenSequence`, and `MAX_TOKENS`.

- [x] **Step 2: Add tokenizer IO types**

Implement `TokenizerInput` and `TokenizerOutput` using `localmt-core`
language/text invariants.

- [x] **Step 3: Add tokenizer trait and mock**

Implement `TokenizerEngine` and `MockTokenizer` without external dependencies.

### Task 3: Facade And Docs

- [x] **Step 1: Re-export tokenizer API**

Expose tokenizer types from `localmt`.

- [x] **Step 2: Document current boundary**

Document that real SentencePiece/BPE wiring is still future work.

### Task 4: Verification

- [x] **Step 1: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 2: Run Kimi checks**

Run `cargo kimi check` in each member crate.
