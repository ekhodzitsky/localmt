# Facade Generation Config Implementation Plan

**Goal:** Let SDK users parse optional generation config from
`OfflineTranslatorPlan` without reaching into the ORT-specific generator plan.

**Architecture:** Keep ORT-owned parsing as the lower-level implementation, but
expose a facade convenience method that maps errors through
`OfflineTranslatorPlanError`. This keeps CLI and future mobile adapters on the
public SDK surface.

**Tech Stack:** Rust workspace, existing `localmt` and `localmt-cli` crates. No
new dependencies.

---

### Task 1: Lock Facade Behavior

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing test**

Add a facade test that builds a verified pack with tokenizer, encoder, decoder,
and valid `generation_config`, then calls
`OfflineTranslatorPlan::parse_generation_config`.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt offline_translator_plan_parses_generation_config`

Expected: compile failure because the facade method does not exist.

### Task 2: Implement Facade Method

**Files:**
- Modify: `crates/localmt/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add facade parser method**

Add `OfflineTranslatorPlan::parse_generation_config` that delegates to the
generator plan and maps `OrtEngineError` through `OfflineTranslatorPlanError`.

- [x] **Step 4: Route CLI through facade**

Update `localmt model plan` to call the new facade method instead of reaching
through `plan.generator()`.

- [x] **Step 5: Run GREEN**

Run: `cargo test -p localmt offline_translator_plan_parses_generation_config`

Expected: test passes.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-facade-generation-config.md`

- [x] **Step 6: Update docs**

Document facade-owned generation-config parsing.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt offline_translator_plan_parses_generation_config
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

Commit the facade parser method, CLI route update, tests, and docs with the
Lore commit protocol.
