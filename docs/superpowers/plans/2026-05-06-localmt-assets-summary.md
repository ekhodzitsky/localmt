# Offline Translator Assets Summary Implementation Plan

**Goal:** Expose a structured preflight summary for prepared offline translator
assets so adapters do not parse CLI text or reach through nested plans.

**Architecture:** Keep `OfflineTranslatorAssets` as the prepared no-inference
object. Add an owned `OfflineTranslatorAssetsSummary` built from it, containing
model id, tokenizer/generator paths, optional cached decoder path, and optional
generation config.

**Tech Stack:** Rust workspace, existing `localmt` and `localmt-cli` crates. No
new dependencies.

---

### Task 1: Lock Summary Behavior

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing test**

Add a facade test that prepares assets from a verified pack with tokenizer,
encoder, decoder, cached decoder, and generation config, then asserts summary
fields.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt offline_translator_assets_summary_reports_preflight_paths`

Expected: compile failure because `OfflineTranslatorAssets::summary` does not
exist.

### Task 2: Implement Summary

**Files:**
- Modify: `crates/localmt/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add summary type**

Add `OfflineTranslatorAssetsSummary` with accessors for model id, tokenizer
path, encoder path, decoder path, optional cached decoder path, and generation
config.

- [x] **Step 4: Route CLI through summary**

Update `localmt model plan` formatting to use `assets.summary()`.

- [x] **Step 5: Run GREEN**

Run: `cargo test -p localmt offline_translator_assets_summary_reports_preflight_paths`

Expected: test passes.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-assets-summary.md`

- [x] **Step 6: Update docs**

Document structured preflight summary.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt offline_translator_assets_summary_reports_preflight_paths
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

Commit the summary API, CLI route update, tests, and docs with the Lore commit
protocol.
