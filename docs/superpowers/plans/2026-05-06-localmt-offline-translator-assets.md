# Offline Translator Assets Implementation Plan

**Goal:** Add a facade-owned prepared asset object that combines
`OfflineTranslatorPlan` with parsed optional generation config while still
avoiding tokenizer parsing, ONNX Runtime session loading, and decoder execution.

**Architecture:** Keep `OfflineTranslatorPlan` as the raw verified path plan.
Add `OfflineTranslatorAssets` as the next no-inference preparation layer for
CLI/mobile adapters. It owns the plan and the parsed config so adapters do not
repeat pack verification, planning, and config parsing.

**Tech Stack:** Rust workspace, existing `localmt` and `localmt-cli` crates. No
new dependencies.

---

### Task 1: Lock Prepared Assets Behavior

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing test**

Add a facade test that builds a verified pack with tokenizer, encoder, decoder,
and valid generation config, then calls `OfflineTranslatorAssets::from_pack`.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt offline_translator_assets_prepare_plan_and_generation_config`

Expected: compile failure because `OfflineTranslatorAssets` does not exist.

### Task 2: Implement Prepared Assets

**Files:**
- Modify: `crates/localmt/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add `OfflineTranslatorAssets`**

Add a facade struct with `from_pack`, `from_plan`, `plan`, and
`generation_config` accessors.

- [x] **Step 4: Route CLI through prepared assets**

Update `localmt model plan` to build `OfflineTranslatorAssets` and read the
summary data from it.

- [x] **Step 5: Run GREEN**

Run: `cargo test -p localmt offline_translator_assets_prepare_plan_and_generation_config`

Expected: test passes.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-offline-translator-assets.md`

- [x] **Step 6: Update docs**

Document prepared assets as the no-inference layer above raw planning.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt offline_translator_assets_prepare_plan_and_generation_config
cargo test -p localmt-cli cli_plans_verified_model_pack_assets
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt` and `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the prepared-assets facade, CLI route update, tests, and docs with the
Lore commit protocol.
