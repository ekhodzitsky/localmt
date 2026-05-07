# ORT Session Slot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an interior-mutability boundary for feature-gated ORT sessions so future `OrtTokenGenerator::generate(&self)` can run mutable sessions without changing the shared `TokenGenerator` trait.

**Architecture:** Wrap each loaded `OrtEngine` in an `OrtEngineSlot` backed by `std::sync::Mutex`. The slot owns the role for stable diagnostics, exposes lightweight inspection helpers, and provides a `with_mut` method for future encoder/decoder execution. `OrtTokenGenerator` stores slots instead of raw engines under `ort-runtime`; default builds remain unchanged.

**Tech Stack:** Rust standard library `Mutex`, feature-gated `localmt-engine-ort`, existing `OrtEngine`, compile-focused TDD.

---

### Task 1: Add Compile-Focused RED Test

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing ignored test**

Add a feature-gated ignored test named:

```rust
token_generator_exposes_locked_sessions_for_generate_boundary
```

The test body should bind this signature:

```rust
fn assert_signature(generator: &OrtTokenGenerator) {
    let _encoder: &OrtEngineSlot = generator.encoder();
    let _decoder: &OrtEngineSlot = generator.decoder();
    let _past: Option<&OrtEngineSlot> = generator.decoder_with_past();
}

let _signature: fn(&OrtTokenGenerator) = assert_signature;
```

Mark it ignored because constructing a feature-enabled `OrtTokenGenerator` requires real ONNX model assets; the test still compiles under `ort-runtime`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime token_generator_exposes_locked_sessions_for_generate_boundary
```

Expected: compile failure because `OrtEngineSlot` does not exist and `OrtTokenGenerator` accessors still expose raw `OrtEngine`.

### Task 2: Implement Session Slot

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add lock error**

Extend `OrtEngineError` with:

```rust
OrtSessionLock(OrtModelRole)
```

The display text should include the role whose session lock was poisoned.

- [x] **Step 2: Add `OrtEngineSlot`**

Behind `ort-runtime`, add:

```rust
#[derive(Debug)]
pub struct OrtEngineSlot {
    role: OrtModelRole,
    engine: std::sync::Mutex<OrtEngine>,
}
```

Expose:

```rust
pub fn new(engine: OrtEngine) -> Self
pub const fn role(&self) -> OrtModelRole
pub fn with_mut<T>(&self, operation: impl FnOnce(&mut OrtEngine) -> Result<T, OrtEngineError>) -> Result<T, OrtEngineError>
pub fn input_count(&self) -> Result<usize, OrtEngineError>
pub fn output_count(&self) -> Result<usize, OrtEngineError>
```

- [x] **Step 3: Store slots in `OrtTokenGenerator`**

Change feature-enabled `OrtTokenGenerator` fields:

```rust
encoder: OrtEngineSlot,
decoder: OrtEngineSlot,
decoder_with_past: Option<OrtEngineSlot>,
```

Wrap loaded sessions with `OrtEngineSlot::new`.

- [x] **Step 4: Update accessors**

Change feature-enabled accessors to return `&OrtEngineSlot` and `Option<&OrtEngineSlot>`.

- [x] **Step 5: Re-export slot from facade**

Add `OrtEngineSlot` to the `localmt` facade exports.

- [x] **Step 6: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime token_generator_exposes_locked_sessions_for_generate_boundary
cargo check -p localmt-engine-ort --features ort-runtime
```

Expected: ignored signature test compiles and the feature build succeeds.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document mutability boundary**

Mention that runtime-enabled `OrtTokenGenerator` stores encoder/decoder sessions in `OrtEngineSlot` so future generation can borrow mutable ORT sessions behind the trait's shared-reference API.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort --features ort-runtime token_generator_exposes_locked_sessions_for_generate_boundary
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

- [x] **Step 2: Commit with Lore protocol**

Create a commit explaining why session mutability is isolated before wiring `OrtTokenGenerator::generate`.
