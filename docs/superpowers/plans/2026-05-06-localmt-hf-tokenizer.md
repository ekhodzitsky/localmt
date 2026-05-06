# HF Tokenizer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a real tokenizer backend that can load Hugging Face `tokenizer.json` files while keeping the default build dependency-light.

**Architecture:** Add an optional `hf-tokenizers` feature to `localmt-tokenizer` and forward it through the `localmt` facade. `HfTokenizer` implements the existing `TokenizerEngine` trait using the `tokenizers` crate; mock tokenization remains the default.

**Tech Stack:** Rust workspace, optional `tokenizers = 0.21.4` with default features disabled and pure-Rust `fancy-regex`, existing tokenizer boundary tests.

---

### Task 1: Lock HF Tokenizer Contract

**Files:**
- Modify: `crates/localmt-tokenizer/Cargo.toml`
- Modify: `crates/localmt/Cargo.toml`
- Modify: `crates/localmt-tokenizer/src/lib.rs`

- [x] **Step 1: Add feature and failing tests**

Add:
- `localmt-tokenizer/hf-tokenizers`
- `localmt/hf-tokenizers`
- tests that build a small `WordLevel` tokenizer JSON, load it through `HfTokenizer::from_file`, encode text into token ids, and decode token ids back to text
- a test that missing tokenizer JSON maps to `TokenizerError::TokenizerLoad`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-tokenizer --features hf-tokenizers hf_tokenizer
```

Expected: fail because `HfTokenizer` and new tokenizer error variants are not implemented yet.

### Task 2: Implement HF Tokenizer

**Files:**
- Modify: `crates/localmt-tokenizer/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 3: Add error variants**

Add:
- `TokenizerLoad { path, reason }`
- `TokenizerEncode(String)`
- `TokenizerDecode(String)`

- [x] **Step 4: Add `HfTokenizer`**

Add:
- `HfTokenizer::from_file(path)`
- `TokenizerEngine for HfTokenizer`
- encode maps tokenizers ids to `TokenId`
- decode maps local token ids to tokenizers ids and validates `NonEmptyText`

- [x] **Step 5: Re-export from facade**

Re-export `HfTokenizer` from `localmt` behind `hf-tokenizers`.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-tokenizer --features hf-tokenizers hf_tokenizer
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-hf-tokenizer.md`

- [x] **Step 7: Update README**

Document the optional `hf-tokenizers` backend and clarify that real translation still needs decoder execution.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-tokenizer --features hf-tokenizers hf_tokenizer
cargo test -p localmt-tokenizer
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features "ort-runtime hf-tokenizers"
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-tokenizer`.

- [x] **Step 9: Commit**

Commit the optional tokenizer backend, tests, docs, and lockfile with the Lore commit protocol.
