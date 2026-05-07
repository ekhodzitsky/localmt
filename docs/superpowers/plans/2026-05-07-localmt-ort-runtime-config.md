# ORT Runtime Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a strict typed runtime config for future ORT token generation, requiring both decoder-loop settings and ONNX tensor-name bindings.

**Architecture:** Keep the strict config in `localmt-engine-ort`. Planning and facade summaries can remain diagnostic, but `OrtGeneratorRuntimeConfig` is the contract that real `OrtTokenGenerator::load` and later `generate` will consume.

**Tech Stack:** Rust `localmt-engine-ort`, existing `GenerationConfig`, existing `OrtIoConfig`, TDD.

---

### Task 1: Add Runtime Config Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests for:

```rust
generator_plan_parses_runtime_config
generator_plan_requires_generation_config_for_runtime_config
generator_plan_requires_ort_io_config_for_runtime_config
```

- [x] **Step 2: Verify RED**

Run: `cargo test -p localmt-engine-ort runtime_config`

Expected: compile failure because `OrtGeneratorRuntimeConfig` and `parse_runtime_config` do not exist.

### Task 2: Implement Strict Runtime Config

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Add the config type**

Implement `OrtGeneratorRuntimeConfig` with `generation_config()` and `ort_io_config()` accessors.

- [x] **Step 2: Add missing-config errors**

Add `MissingGenerationConfig` and `MissingOrtIoConfig` to `OrtEngineError`.

- [x] **Step 3: Add parser**

Implement `OrtGeneratorPlan::parse_runtime_config()` by requiring parsed `generation_config` and `ort_io` values.

- [x] **Step 4: Verify GREEN**

Run: `cargo test -p localmt-engine-ort runtime_config`

Expected: the new runtime-config tests pass.

### Task 3: Wire Runtime Load And Docs

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

- [x] **Step 1: Store runtime config in feature-enabled generator**

Have `OrtTokenGenerator::load` parse and store `OrtGeneratorRuntimeConfig` before creating sessions.

- [x] **Step 2: Document the strict boundary**

Document that diagnostic preflight can report absent/missing, but runtime load requires parsed generation and ORT I/O config.

- [x] **Step 3: Verify**

Run:

```bash
cargo fmt --check
cargo test -p localmt-engine-ort runtime_config
cargo test -p localmt-engine-ort
cargo check -p localmt-engine-ort --features ort-runtime
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

### Task 4: Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run Kimi**

Run `cargo kimi check` from `crates/localmt-engine-ort`.

- [ ] **Step 2: Commit with Lore protocol**

Create a commit explaining why runtime load now has a strict config gate before generation logic exists.
