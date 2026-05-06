# Mock Offline Translator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a facade-level mock offline translator that requires prepared model-pack assets before running the existing mock translation pipeline.

**Architecture:** Keep the type in the `localmt` facade crate. It should own `OfflineTranslatorAssets` plus `TranslationPipeline<MockTokenizer, MockTokenGenerator>`, expose assets for adapters, and implement/offer the existing `TranslatorEngine` translation contract.

**Tech Stack:** Rust workspace, `localmt` facade, existing mock tokenizer/generator/pipeline only.

---

### Task 1: Lock Facade Behavior

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing test**

Add `mock_offline_translator_prepares_assets_before_translation`:
- create a verified plannable model pack
- load `MockOfflineTranslator::from_model_pack_path`
- assert generation config is available through `assets()`
- translate a valid request and assert mock output equals input text

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt mock_offline_translator
```

Expected: fail because `MockOfflineTranslator` does not exist.

### Task 2: Implement Facade Type

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 3: Add type and constructors**

Add:
- `MockOfflineTranslator`
- `from_model_pack_path`
- `from_assets`
- `assets`

- [x] **Step 4: Add translation behavior**

Delegate translation to `TranslationPipeline<MockTokenizer, MockTokenGenerator>` and implement `TranslatorEngine`.

- [x] **Step 5: Run GREEN**

Run:

```bash
cargo test -p localmt mock_offline_translator
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-mock-offline-translator.md`

- [x] **Step 6: Update README**

Document that `MockOfflineTranslator` is for adapter integration only and still uses mock tokenizer/generator behavior.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt mock_offline_translator
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt`.

- [x] **Step 8: Commit**

Commit the facade type, tests, and docs with the Lore commit protocol.
