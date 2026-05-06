# localmt Pipeline CLI And Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development for behavior changes. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Route the development CLI and Xiaomi 17 benchmark through the
pipeline skeleton instead of the older direct mock engine.

**Architecture:** Keep `MockEngine` available for narrow engine unit tests, but
make user-facing smoke paths use `TranslationPipeline<MockTokenizer,
MockTokenGenerator>`. Benchmark reports label this path as `mock-pipeline`.

**Tech Stack:** Rust 1.95, existing `localmt-pipeline`, `localmt-tokenizer`, and
`localmt-engine`; no new external dependencies.

---

## Files

- Modify: `crates/localmt-bench/Cargo.toml`
- Modify: `crates/localmt-bench/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Smoke Tests

- [x] **Step 1: CLI pipeline output**

Update CLI translate tests to expect the pipeline echo output rather than the
old direct mock-engine prefix.

- [x] **Step 2: Benchmark runtime label**

Update benchmark tests to expect `runtime: mock-pipeline`.

### Task 2: Implementation

- [x] **Step 1: Switch benchmark runner**

Run benchmark scenarios through `TranslationPipeline<MockTokenizer,
MockTokenGenerator>`.

- [x] **Step 2: Switch CLI translate path**

Use the same pipeline through the existing `Translator` facade in the CLI.

### Task 3: Documentation

- [x] **Step 1: Update README**

Document the `mock-pipeline` benchmark path.

- [x] **Step 2: Update design spec**

Record that CLI and benchmark smoke paths now exercise the pipeline shape.

### Task 4: Verification

- [x] **Step 1: Run workspace checks**

Run `cargo fmt --check`, `cargo test`, `cargo test --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 2: Run Kimi checks**

Run `cargo kimi check` in each member crate.
