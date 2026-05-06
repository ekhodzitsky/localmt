# CLI HF Tokenizer Smoke Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a local developer smoke command that verifies a model pack, loads its Hugging Face `tokenizer.json`, encodes text, and decodes the token ids back without running ONNX inference.

**Architecture:** Add `localmt-cli/hf-tokenizers` and forward it to `localmt/hf-tokenizers`. Add `localmt model tokenize PACK FROM TO TEXT`. Default builds still parse and plan the pack, then return a clear feature-disabled error; feature builds construct `HfTokenizer` from the verified tokenizer path and use the existing `TokenizerEngine` contract.

**Tech Stack:** Rust CLI, existing model pack verification, optional `HfTokenizer`, no new external dependencies.

---

### Task 1: Lock CLI Contract

**Files:**
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Add failing tests**

Add tests that prove:
- help text lists `localmt model tokenize PACK FROM TO TEXT`
- default builds return a feature-disabled error after pack planning
- feature builds load a valid tokenizer JSON, encode `hello offline` to `1, 2`, and decode it back
- feature builds map invalid tokenizer JSON to a tokenizer load error

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-cli tokenizer
```

Expected: fail because the model subcommand, feature, and CLI error path do not exist yet.

### Task 2: Implement CLI Tokenizer Smoke

**Files:**
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`

- [x] **Step 3: Add CLI feature and error**

Add:
- `localmt-cli/hf-tokenizers`
- `CliError::TokenizerFeatureDisabled`
- feature-gated `CliError::Tokenizer(localmt::TokenizerError)`

- [x] **Step 4: Add `model tokenize` command**

Implement:

```bash
localmt model tokenize PACK FROM TO TEXT
```

Output:

```text
tokenizer: loaded
tokens: 1, 2
decoded: hello offline
```

- [x] **Step 5: Update help and README**

Document the smoke command and clarify that it proves tokenizer loading only.

- [x] **Step 6: Run GREEN**

Run:

```bash
cargo test -p localmt-cli tokenizer
cargo test -p localmt-cli --features hf-tokenizers tokenizer
```

Expected: both pass.

### Task 3: Verify And Commit

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli tokenizer
cargo test -p localmt-cli --features hf-tokenizers tokenizer
cargo test -p localmt-cli
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt-cli --features hf-tokenizers
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the CLI tokenizer smoke command, tests, docs, and plan with the Lore commit protocol.
