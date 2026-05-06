# CLI Manifest Writer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a development CLI command that prints a ready `manifest.json` for a standard local model-pack directory by hashing local files.

**Architecture:** Keep the command in `localmt-cli` and delegate manifest construction to `localmt-models` through the facade re-exports. The command should scan standard file names, require `encoder.onnx`, `decoder.onnx`, and `tokenizer.json`, include optional known roles when present, and print JSON to stdout without writing files.

**Tech Stack:** Rust workspace, `localmt-cli`, `localmt-models` authoring API, std-only argument parsing.

---

### Task 1: Lock CLI Behavior

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing tests**

Add tests for:
- `cli_write_manifest_outputs_standard_model_files`
- `cli_write_manifest_rejects_missing_required_standard_file`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli cli_write_manifest
```

Expected: fail because `model write-manifest` is not routed yet.

### Task 2: Implement Command

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Refactor model command argument parsing**

Keep existing one-path commands unchanged, but allow `write-manifest` to consume:

```text
PACK MODEL_ID VERSION ARCHITECTURE RUNTIME LICENSE
```

- [x] **Step 4: Build manifest from standard files**

Required files:
- `encoder.onnx`
- `decoder.onnx`
- `tokenizer.json`

Optional files:
- `decoder-with-past.onnx`
- `vocab.txt`
- `config.json`
- `generation.json`

Map optional files to `decoder_with_past`, `vocab`, `config`, and `generation_config`.

- [x] **Step 5: Run GREEN**

Run:

```bash
cargo test -p localmt-cli cli_write_manifest
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-cli-write-manifest.md`

- [x] **Step 6: Update help and README**

Add the command to top-level/model help and the Development CLI block.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli cli_write_manifest
cargo test -p localmt-cli cli_prints
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the CLI manifest writer, tests, and docs with the Lore commit protocol.
