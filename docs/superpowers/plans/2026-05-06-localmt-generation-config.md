# Generation Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add typed generation configuration for future decoder loops without parsing files or running inference.

**Architecture:** Keep the types in `localmt-pipeline` because they describe token generation policy, not tokenizer internals or ORT session loading. Re-export them from `localmt` for SDK consumers. Validate token limits and token-role collisions up front so future decoder code does not depend on magic numbers.

**Tech Stack:** Rust workspace, existing `localmt-core`, `localmt-tokenizer`, `localmt-pipeline`, and facade crates. No new external dependencies.

---

### Task 1: Lock Default Config Shape

**Files:**
- Modify: `crates/localmt-pipeline/src/lib.rs`

- [x] **Step 1: Write failing test**

Add a test that builds `GenerationSpecialTokens`, `LanguageTokenIds`, and `GenerationConfig::with_default_limit`, then asserts:
- `max_new_tokens().value() == DEFAULT_MAX_NEW_TOKENS`
- `bos_token_id()` and `eos_token_id()` are preserved
- `target_language_token(Language::Japanese)` returns the Japanese token id

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt-pipeline generation_config_uses_default_limit_and_language_tokens`

Expected: compile failure because the generation config types do not exist.

### Task 2: Implement Typed Config

**Files:**
- Modify: `crates/localmt-pipeline/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 3: Add generation config types**

Add:
- `DEFAULT_MAX_NEW_TOKENS`
- `MaxNewTokens`
- `GenerationSpecialTokens`
- `LanguageTokenIds`
- `GenerationConfig`
- `GenerationConfigError`

- [x] **Step 4: Add validation tests**

Add tests for zero/oversized `MaxNewTokens`, duplicate BOS/EOS tokens, duplicate language tokens, and language tokens colliding with special tokens.

- [x] **Step 5: Run GREEN**

Run: `cargo test -p localmt-pipeline generation_config`

Expected: generation config tests pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-generation-config.md`

- [x] **Step 6: Update docs**

Document that typed generation config exists but file parsing and decoder use are future work.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-pipeline generation_config
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-pipeline` and `crates/localmt`.

- [x] **Step 8: Commit**

Commit the config types, tests, facade re-export, and docs with the Lore commit protocol.
