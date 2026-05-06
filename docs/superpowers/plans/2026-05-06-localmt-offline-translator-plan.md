# Offline Translator Plan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a facade-level plan that turns one verified model pack into the tokenizer and ONNX generator plans needed to construct a future offline translator.

**Architecture:** Keep the composition in `crates/localmt` because it is a public SDK convenience over existing lower-level crates. The new type owns `TokenizerAssetPlan` and `OrtGeneratorPlan`; it does not load sessions, parse tokenizer files, or run inference.

**Tech Stack:** Rust workspace, existing `localmt-models`, `localmt-tokenizer`, and `localmt-engine-ort` crates. No new dependencies.

---

### Task 1: Lock Facade Plan Behavior

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests that call `OfflineTranslatorPlan::from_pack(&pack)` on verified model packs. The success case should assert that tokenizer and ORT generator paths are preserved. Error cases should assert missing tokenizer and missing decoder are surfaced through facade-owned errors.

- [x] **Step 2: Run RED test**

Run: `cargo test -p localmt offline_translator_plan`

Expected: compile failure because `OfflineTranslatorPlan` and `OfflineTranslatorPlanError` do not exist yet.

### Task 2: Implement The Facade Plan

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 3: Add public plan type**

Add:

```rust
pub struct OfflineTranslatorPlan {
    tokenizer: TokenizerAssetPlan,
    generator: OrtGeneratorPlan,
}
```

with `from_pack`, `tokenizer`, and `generator` methods.

- [x] **Step 4: Add public error type**

Add `OfflineTranslatorPlanError::{Tokenizer, Generator}` with `Display` and `Error` impls.

- [x] **Step 5: Run GREEN tests**

Run: `cargo test -p localmt offline_translator_plan`

Expected: tests pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-offline-translator-plan.md`

- [x] **Step 6: Update docs**

Document that `OfflineTranslatorPlan` is a facade-level planning object and explicitly does not load ORT sessions or run translation.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt offline_translator_plan
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

For `cargo kimi check`, run it from `crates/localmt` because root virtual-workspace checking is not reliable here.

- [x] **Step 8: Commit**

Commit the facade plan, tests, and docs with the Lore commit protocol.
