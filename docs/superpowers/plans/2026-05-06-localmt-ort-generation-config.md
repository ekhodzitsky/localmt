# ORT Generation Config Plan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `OrtGeneratorPlan` parse an optional verified `generation_config` asset into typed `GenerationConfig`.

**Architecture:** Keep parsing as an explicit `OrtGeneratorPlan` method so session loading and decoder execution remain separate. Return `Ok(None)` when a pack has no `generation_config` role and surface parser failures through `OrtEngineError`.

**Tech Stack:** Rust workspace, existing `localmt-engine-ort`, `localmt-pipeline`, and `localmt-tokenizer` crates. No new external dependencies.

---

### Task 1: Lock Optional Config Parsing

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing test**

Add a test that creates a verified ONNX pack with a valid `generation_config` role, builds `OrtGeneratorPlan`, calls `parse_generation_config`, and asserts the parsed max-new-token/BOS/EOS ids.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt-engine-ort generator_plan_parses_optional_generation_config`

Expected: compile failure because `parse_generation_config` does not exist.

### Task 2: Implement Plan Method

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 3: Add error mapping**

Add `OrtEngineError::GenerationConfig(GenerationConfigParseError)` with display/source handling.

- [x] **Step 4: Add parser method**

Add `OrtGeneratorPlan::parse_generation_config(&self) -> Result<Option<GenerationConfig>, OrtEngineError>` that returns `Ok(None)` without a config path and parses the file when present.

- [x] **Step 5: Add no-config test**

Add a test that verifies packs without `generation_config` return `Ok(None)`.

- [x] **Step 6: Run GREEN**

Run: `cargo test -p localmt-engine-ort generator_plan`

Expected: generator-plan tests pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-ort-generation-config.md`

- [x] **Step 7: Update docs**

Document that ORT generator plans can parse config explicitly, but `OrtTokenGenerator::load` still does not execute decoder inference.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort generator_plan
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt-engine-ort --features ort-runtime
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-engine-ort` and `crates/localmt`.

- [x] **Step 9: Commit**

Commit parser hook, tests, and docs with the Lore commit protocol.
