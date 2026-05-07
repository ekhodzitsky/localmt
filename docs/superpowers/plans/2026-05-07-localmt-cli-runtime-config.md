# CLI Runtime Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a no-inference CLI gate that validates the strict ORT runtime config for a verified model pack.

**Architecture:** Route `localmt model runtime-config PACK` through the facade plan and `OrtGeneratorPlan::parse_runtime_config`. The command prints stable text with generation limit and selected ORT tensor names, without loading tokenizer backends or ONNX sessions.

**Tech Stack:** Rust CLI, existing model-pack verification, existing ORT runtime config parser, TDD.

---

### Task 1: Add CLI Tests

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing tests**

Add tests for:

```rust
cli_prints_model_help
cli_reports_ort_runtime_config_for_verified_pack
```

- [x] **Step 2: Verify RED**

Run: `cargo test -p localmt-cli runtime_config`

Expected: failure because the command is not routed and help text does not list it.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Route command**

Add `runtime-config` under `run_model`.

- [x] **Step 2: Implement formatter**

Verify the pack, build `OfflineTranslatorPlan`, call `plan.generator().parse_runtime_config()`, and print:

```text
runtime_config: ok
max_new_tokens: 32
encoder_input_ids: encoder_input_ids
decoder_logits: decoder_logits
```

- [x] **Step 3: Verify GREEN**

Run: `cargo test -p localmt-cli runtime_config`

Expected: runtime-config tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`
- Modify: `docs/android-build.md`

- [x] **Step 1: Update docs**

Mention `model runtime-config` as the strict no-inference gate for future ORT generation.

- [x] **Step 2: Verify**

Run:

```bash
cargo fmt --check
cargo test -p localmt-cli runtime_config
cargo test -p localmt-cli
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

### Task 4: Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run Kimi**

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 2: Commit with Lore protocol**

Create a commit explaining why strict ORT runtime readiness is now CLI-visible before real generation.
