# ORT I/O Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parse a verified model pack `config` file into typed ONNX tensor names needed by the future ORT token generation loop.

**Architecture:** Keep ORT-specific tensor naming in `localmt-engine-ort`, not in the generic pipeline crate. `OrtGeneratorPlan` records the optional `config` role path and exposes explicit parsing that returns `Ok(None)` when the pack has no ORT I/O contract.

**Tech Stack:** Rust workspace, `localmt-models` verified pack roles, existing workspace `serde` and `serde_json`, TDD.

---

### Task 1: Add ORT I/O Contract Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests for:

```rust
ort_io_config_parses_tensor_names_from_json_str
ort_io_config_rejects_empty_tensor_names
generator_plan_parses_optional_ort_io_config
generator_plan_returns_none_without_ort_io_config
```

- [x] **Step 2: Verify RED**

Run: `cargo test -p localmt-engine-ort ort_io_config`

Expected: compile failure because `OrtIoConfig`, accessors, and `parse_ort_io_config` do not exist yet.

### Task 2: Implement ORT I/O Parsing

**Files:**
- Modify: `crates/localmt-engine-ort/Cargo.toml`
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Add existing workspace JSON dependencies**

Use:

```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

- [x] **Step 2: Add typed config structs and validation**

Implement `OrtIoConfig`, `OrtEncoderIoNames`, `OrtDecoderIoNames`, `OrtIoConfigError`, and `OrtIoConfigParseError`. Parse the `ort_io` JSON object and reject empty or whitespace-only tensor names.

- [x] **Step 3: Wire `config` role into `OrtGeneratorPlan`**

Add `config_path`, `config_path()`, and `parse_ort_io_config()`. The parser returns `Ok(None)` when the pack has no `config` role and `Ok(Some(config))` when `ort_io` is present and valid.

- [x] **Step 4: Verify GREEN**

Run: `cargo test -p localmt-engine-ort ort_io_config`

Expected: the new tests pass.

### Task 3: Document the Contract

**Files:**
- Modify: `README.md`
- Modify: `docs/android-build.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

- [x] **Step 1: Update docs**

Document that `config.json` may include an `ort_io` object with encoder and decoder tensor names.

- [x] **Step 2: Verify**

Run:

```bash
cargo fmt --check
cargo test -p localmt-engine-ort
cargo check -p localmt-engine-ort --features ort-runtime
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

### Task 4: Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run final verification**

Run:

```bash
cargo test
cargo test --all-features
cargo kimi check
```

Note: root `cargo kimi check` reported all contracts, clippy, and tests passing,
then hit a cargo-kimi internal `attempt to divide by zero` panic. Crate-level
`cargo kimi check` passed from `crates/localmt-engine-ort` and
`crates/localmt-cli`.

- [ ] **Step 2: Commit with Lore protocol**

Create a commit describing why the ORT generation boundary now has a typed I/O contract before implementing token generation.
