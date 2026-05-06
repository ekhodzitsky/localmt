# Generation Config Parser Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parse a local `generation_config` JSON file into the typed `GenerationConfig` boundary.

**Architecture:** Keep parsing in `localmt-pipeline` because the output is a generation policy, not an ORT session or tokenizer primitive. Use existing workspace `serde/serde_json` dependencies. The parser accepts a narrow local schema with `bos_token_id`, `eos_token_id`, optional `max_new_tokens`, and `language_token_ids` for `en`, `ru`, `th`, `vi`, and `ja`.

**Tech Stack:** Rust workspace, `serde`, `serde_json`, existing `localmt-core`, `localmt-tokenizer`, and `localmt-pipeline` crates.

---

### Task 1: Lock Parser Shape

**Files:**
- Modify: `crates/localmt-pipeline/src/lib.rs`
- Modify: `crates/localmt-pipeline/Cargo.toml`

- [x] **Step 1: Write failing test**

Add a test that writes `generation.json` with BOS/EOS, max_new_tokens, and language tokens, then calls `GenerationConfig::from_json_file(&path)` and asserts all values are preserved.

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt-pipeline generation_config_parses_json_file`

Expected: compile failure because `from_json_file` and parser errors do not exist.

### Task 2: Implement Parser

**Files:**
- Modify: `crates/localmt-pipeline/Cargo.toml`
- Modify: `crates/localmt-pipeline/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 3: Add serde dependencies**

Add `serde.workspace = true` and `serde_json.workspace = true` to `localmt-pipeline`.

- [x] **Step 4: Add parser API**

Add:
- `GenerationConfig::from_json_file(&Path)`
- `GenerationConfig::from_json_str(&str)`
- `GenerationConfigParseError`

- [x] **Step 5: Add parser validation tests**

Add tests for omitted `max_new_tokens` using default, malformed JSON, missing required language token, and validation errors flowing through parser error.

- [x] **Step 6: Run GREEN**

Run: `cargo test -p localmt-pipeline generation_config`

Expected: generation config parser and validation tests pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-generation-config-parser.md`

- [x] **Step 7: Update docs**

Document the accepted JSON schema and clarify that parsing still does not run decoder inference.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-pipeline generation_config
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-pipeline` and `crates/localmt`.

- [x] **Step 9: Commit**

Commit parser, tests, facade re-export, and docs with the Lore commit protocol.
