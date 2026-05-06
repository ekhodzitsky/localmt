# localmt Initial Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the first verified Rust workspace for the `localmt` library.

**Architecture:** Start with std-only core types and a mock engine. Keep all
future ONNX Runtime integration behind `TranslatorEngine` so model inference can
be added without changing the public facade.

**Tech Stack:** Rust 1.95, Cargo workspace resolver 3, Kimi Rust guidelines,
std-only first slice.

---

## Files

- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `README.md`
- Copy and specialize: `AGENTS.md`
- Create: `crates/localmt-core/Cargo.toml`
- Create: `crates/localmt-core/src/lib.rs`
- Create: `crates/localmt-engine/Cargo.toml`
- Create: `crates/localmt-engine/src/lib.rs`
- Create: `crates/localmt/Cargo.toml`
- Create: `crates/localmt/src/lib.rs`
- Create: `crates/localmt-cli/Cargo.toml`
- Create: `crates/localmt-cli/src/main.rs`

### Task 1: Repository Contract

- [x] **Step 1: Initialize git repository**

Run: `git init` in `/Users/ekhodzitsky/Documents/personal/localmt`.

- [x] **Step 2: Rename default branch**

Run: `git branch -m main`.

- [x] **Step 3: Add Rust Kimi guidelines**

Copy `kimi-dotfiles/templates/rust/rust-only/AGENTS.md` into `AGENTS.md` and
add project-specific model-pack and backend-boundary rules.

### Task 2: Core Types

- [x] **Step 1: Create `localmt-core`**

Define `Language`, `LanguagePair`, `NonEmptyText`, `TranslateRequest`, and
`Translation`.

- [x] **Step 2: Add invariant tests**

Test same-language rejection, empty text rejection, code parsing, and request
construction.

### Task 3: Engine Boundary

- [x] **Step 1: Create `TranslatorEngine`**

The trait accepts `TranslateRequest` and returns `Translation`.

- [x] **Step 2: Create `MockEngine`**

Return a deterministic string containing source code, target code, and input.

### Task 4: Facade And CLI

- [x] **Step 1: Create `Translator<E>` facade**

Wrap any engine implementing `TranslatorEngine`.

- [x] **Step 2: Create development CLI**

Accept `FROM TO TEXT` and route through `MockEngine`.

### Task 5: Verification

- [x] **Step 1: Format**

Run: `cargo fmt --check`.

- [x] **Step 2: Test**

Run: `cargo test`.

- [x] **Step 3: Lint**

Run: `cargo clippy --all-targets --all-features -- -D warnings`.

- [x] **Step 4: Docs**

Run: `cargo doc --no-deps`.

- [ ] **Step 5: Commit**

Commit with a Lore-style message that records the std-only first-slice
constraint and the planned ONNX Runtime follow-up.
