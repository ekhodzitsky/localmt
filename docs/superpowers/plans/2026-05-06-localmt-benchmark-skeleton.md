# localmt Benchmark Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a Xiaomi 17 benchmark command before real ONNX Runtime inference.

**Architecture:** Introduce `localmt-bench` with device profile parsing,
fixed smoke scenarios, and a mock-runtime report. Keep the report honest:
the command verifies model-pack assets and measures mock translation only,
without pretending to have ONNX load time or device RSS metrics.

**Tech Stack:** Rust 1.95, std timing, existing `localmt-core`,
`localmt-engine`, and `localmt-models`.

---

## Files

- Modify: `Cargo.toml`
- Create: `crates/localmt-bench/Cargo.toml`
- Create: `crates/localmt-bench/src/lib.rs`
- Modify: `crates/localmt/Cargo.toml`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Benchmark Tests

- [x] **Step 1: Add crate-level tests**

Add tests proving `DeviceProfile::parse("xiaomi17")`, unknown profile
rejection, and mock benchmark execution across 10 scenarios.

- [x] **Step 2: Add CLI test**

Add a CLI test for
`localmt bench --profile xiaomi17 --model-pack PATH`.

### Task 2: Benchmark Implementation

- [x] **Step 1: Add `DeviceProfile`**

Implement the first profile id: `xiaomi17`.

- [x] **Step 2: Add `MockBenchmarkRunner`**

Run 10 fixed language-pair scenarios through `MockEngine` using a
`ModelPack<Verified>`.

- [x] **Step 3: Add `BenchReport`**

Expose profile, runtime, model id, scenario count, translation count, total ms,
and warm translate ms.

### Task 3: CLI And Docs

- [x] **Step 1: Wire CLI command**

Parse `--profile` and `--model-pack`, verify the pack, run the mock benchmark,
and print a plain text report.

- [x] **Step 2: Update README and design spec**

Document that the benchmark is currently mock-runtime only.

### Task 4: Verification

- [x] **Step 1: Run workspace checks**

Run `cargo fmt --check`, `cargo test`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 2: Run Kimi checks**

Run `cargo kimi check` in each member crate.
