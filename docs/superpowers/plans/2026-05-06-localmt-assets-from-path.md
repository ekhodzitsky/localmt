# Offline Translator Assets From Path Implementation Plan

**Goal:** Let SDK users prepare offline translator assets directly from a local
model-pack directory path.

**Architecture:** Keep `ModelPack<Discovered>::discover` and verification in
`localmt-models`, but add a facade convenience entry point that performs
discover -> verify -> prepare. Surface failures through a facade-owned error so
mobile adapters can depend on `localmt` without manually wiring lower-level
crate errors.

**Tech Stack:** Rust workspace, existing `localmt` and `localmt-cli` crates. No
new dependencies.

---

### Task 1: Lock Path Preparation Behavior

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing test**

Add a facade test that creates a valid local model pack directory and calls
`OfflineTranslatorAssets::from_model_pack_path`.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt offline_translator_assets_prepare_from_model_pack_path`

Expected: compile failure because the path preparation method does not exist.

### Task 2: Implement Facade Entry Point

**Files:**
- Modify: `crates/localmt/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add facade error**

Add `OfflineTranslatorAssetsError` with model-pack and planning variants.

- [x] **Step 4: Add path constructor**

Add `OfflineTranslatorAssets::from_model_pack_path` that discovers, verifies,
and prepares assets.

- [x] **Step 5: Route CLI through path constructor**

Update `localmt model plan` to use the facade path constructor.

- [x] **Step 6: Run GREEN**

Run: `cargo test -p localmt offline_translator_assets_prepare_from_model_pack_path`

Expected: test passes.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-assets-from-path.md`

- [x] **Step 7: Update docs**

Document direct path preparation as the SDK-level model-pack loading entry
point.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt offline_translator_assets_prepare_from_model_pack_path
cargo test -p localmt-cli cli_plans_verified_model_pack_assets
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt` and `crates/localmt-cli`.

- [x] **Step 9: Commit**

Commit the facade path constructor, CLI route update, tests, and docs with the
Lore commit protocol.
