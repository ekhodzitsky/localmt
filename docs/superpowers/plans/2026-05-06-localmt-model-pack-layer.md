# localmt Model-Pack Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add a verified model-pack layer before real ONNX Runtime inference.

**Architecture:** Keep model assets behind `ModelPack<Discovered>` and
`ModelPack<Verified>`. Parse `manifest.json` with structured JSON, validate
manifest fields as newtypes, reject unsafe paths, and verify file SHA-256
digests before an inference backend can consume the pack.

**Tech Stack:** Rust 1.95, `serde`, `serde_json`, `sha2`, Kimi Rust guidelines.

---

## Files

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `crates/localmt-models/Cargo.toml`
- Create: `crates/localmt-models/src/lib.rs`
- Modify: `crates/localmt/Cargo.toml`
- Modify: `crates/localmt/src/lib.rs`
- Modify: `crates/localmt-cli/Cargo.toml`
- Modify: `crates/localmt-cli/src/main.rs`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`

### Task 1: Failing Model-Pack Tests

- [x] **Step 1: Add tests for complete pack verification**

`localmt-models` tests create a temporary pack with `manifest.json`,
`encoder.onnx`, and `tokenizer.json`; `ModelPack::<Discovered>::discover`
followed by `verify` must return `ModelPack<Verified>`.

- [x] **Step 2: Add tests for rejected packs**

Tests cover missing required language, checksum mismatch, and relative path
escape through `../encoder.onnx`.

### Task 2: Model-Pack Implementation

- [x] **Step 1: Add validated manifest types**

Implement `ModelManifest`, `ModelId`, `ModelPackVersion`, `ModelArchitecture`,
`ModelRuntime`, `ModelLicense`, `ModelFileKind`, `ModelRelativePath`, and
`Sha256Digest`.

- [x] **Step 2: Add typestate pack lifecycle**

Implement `ModelPack<Discovered>::discover` and
`ModelPack<Discovered>::verify`.

- [x] **Step 3: Verify checksums**

Read each declared file with std I/O and compute SHA-256 through `sha2`.

### Task 3: CLI And Facade

- [x] **Step 1: Re-export model types from `localmt`**

The facade crate exposes the stable model-pack API.

- [x] **Step 2: Add CLI model commands**

`localmt model inspect PATH` prints manifest summary.
`localmt model verify PATH` verifies files and prints the verified model id.

### Task 4: Verification

- [x] **Step 1: Run targeted tests**

Run: `cargo test -p localmt-models` and `cargo test -p localmt-cli`.

- [x] **Step 2: Run workspace checks**

Run: `cargo fmt --check`, `cargo test`,
`cargo clippy --all-targets --all-features -- -D warnings`, and
`cargo doc --no-deps`.

- [x] **Step 3: Run Kimi checks**

Run `cargo kimi check` in each member crate because root virtual-workspace mode
currently returns zero files.
