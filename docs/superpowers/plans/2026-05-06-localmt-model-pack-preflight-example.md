# Model Pack Preflight Example Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an SDK example that verifies a local model pack and prints the prepared offline-translator asset summary without running inference.

**Architecture:** Keep the example inside the public facade crate so it demonstrates the intended library-first API. Reuse `OfflineTranslatorAssets::from_model_pack_path` and `OfflineTranslatorAssetsSummary`; do not duplicate model-pack verification logic or add dependencies.

**Tech Stack:** Rust workspace, `crates/localmt` example target, std-only CLI parsing.

---

### Task 1: Lock Example Formatting

**Files:**
- Create: `crates/localmt/examples/model_pack_preflight.rs`

- [x] **Step 1: Write the failing test**

Add a unit test inside the example target that creates a temporary verified pack, runs the example formatting path, and asserts the output includes:
- `planned: m2m100-418m-int8`
- `tokenizer:`
- `encoder:`
- `decoder:`
- `generation_config: parsed`
- `max_new_tokens: 32`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt --example model_pack_preflight preflight_example_prints_asset_summary
```

Expected: fail because the example target or its `run` function does not exist yet.

### Task 2: Implement The Example

**Files:**
- Create: `crates/localmt/examples/model_pack_preflight.rs`

- [x] **Step 3: Add std-only example binary**

Implement:
- `main() -> ExitCode`
- `run(args: impl Iterator<Item = String>) -> Result<String, PreflightError>`
- `format_summary(summary: &OfflineTranslatorAssetsSummary) -> String`
- `PreflightError` with Display/Error impls

The example must accept exactly one model-pack path and print the no-inference summary.

- [x] **Step 4: Run GREEN**

Run:

```bash
cargo test -p localmt --example model_pack_preflight preflight_example_prints_asset_summary
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-model-pack-preflight-example.md`

- [x] **Step 5: Document the example**

Add the cargo command to the Facade Planning or Model Packs README section:

```bash
cargo run -p localmt --example model_pack_preflight -- ./models/m2m100-418m-int8
```

- [x] **Step 6: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt --example model_pack_preflight
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
```

- [x] **Step 7: Commit**

Commit the example and docs with the Lore commit protocol.
