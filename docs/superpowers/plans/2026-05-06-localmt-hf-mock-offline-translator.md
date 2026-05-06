# HF Mock Offline Translator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a facade-level translator that loads real Hugging Face tokenization from a verified model pack while still using the deterministic mock token generator.

**Architecture:** Add `HfMockOfflineTranslator` behind `localmt/hf-tokenizers`. It owns `OfflineTranslatorAssets` plus `TranslationPipeline<HfTokenizer, MockTokenGenerator>`. It is explicitly a tokenizer integration smoke path, not real translation.

**Tech Stack:** Rust facade crate, existing `HfTokenizer`, existing `TranslationPipeline`, existing `MockTokenGenerator`.

---

### Task 1: Lock Facade Contract

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add failing tests**

Add feature-gated tests that prove:
- `HfMockOfflineTranslator::from_model_pack_path` verifies assets, loads a valid tokenizer JSON, and round-trips text through `translate`
- invalid tokenizer JSON maps to a tokenizer load error
- prepared assets remain accessible through `assets()`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt --features hf-tokenizers hf_mock
```

Expected: fail because `HfMockOfflineTranslator` and its error type do not exist.

### Task 2: Implement Translator

**Files:**
- Modify: `crates/localmt/src/lib.rs`
- Modify: `README.md`

- [x] **Step 3: Add error type**

Add `HfMockOfflineTranslatorError` with:
- `Assets(OfflineTranslatorAssetsError)`
- `Tokenizer(TokenizerError)`

- [x] **Step 4: Add `HfMockOfflineTranslator`**

Add:
- `from_model_pack_path`
- `from_assets`
- `assets`
- inherent `translate`
- `TranslatorEngine` implementation

The type must load `HfTokenizer` from the verified tokenizer path and compose it with `MockTokenGenerator`.

- [x] **Step 5: Update README**

Document the new facade smoke path and clarify that generation still uses a mock generator.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt --features hf-tokenizers hf_mock
```

Expected: pass.

### Task 3: Verify And Commit

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt --features hf-tokenizers hf_mock
cargo test -p localmt
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features hf-tokenizers
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt`.

- [x] **Step 8: Commit**

Commit the facade tokenizer-backed mock translator, tests, docs, and plan with the Lore commit protocol.
