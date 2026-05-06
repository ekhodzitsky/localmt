# CLI Plan Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a development CLI smoke command that verifies a model pack, builds `OfflineTranslatorPlan`, and parses optional `generation_config` without running inference.

**Architecture:** Extend the existing `localmt model` command surface with `plan`. The command stays in `localmt-cli` and uses facade types so it tests public SDK composition rather than lower-level crates directly.

**Tech Stack:** Rust workspace, existing `localmt`, `localmt-models`, and `localmt-cli` crates. No new dependencies.

---

### Task 1: Lock CLI Smoke Output

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing test**

Add a test for `localmt model plan <pack>` using a verified pack with tokenizer, encoder, decoder, and generation config. Assert output contains planned model id, asset role paths, `generation_config: parsed`, and `max_new_tokens: 32`.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt-cli cli_plans_verified_model_pack_assets`

Expected: failure because `model plan` is not a known command.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add `model plan` routing**

Route `localmt model plan <pack>` to a new `plan_model` function.

- [x] **Step 4: Build public facade plan**

`plan_model` should verify the pack, call `OfflineTranslatorPlan::from_pack`, call `plan.generator().parse_generation_config()`, and return a compact text summary.

- [x] **Step 5: Add CLI error variants**

Add `OfflinePlan` and `OrtPlan` error variants to surface planning/config errors.

- [x] **Step 6: Run GREEN**

Run: `cargo test -p localmt-cli cli_plans_verified_model_pack_assets`

Expected: test passes.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-cli-plan-smoke.md`

- [x] **Step 7: Update docs**

Document `localmt model plan` as a no-inference smoke command.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli cli_plans_verified_model_pack_assets
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli` and `crates/localmt`.

- [x] **Step 9: Commit**

Commit the CLI smoke command, tests, and docs with the Lore commit protocol.
