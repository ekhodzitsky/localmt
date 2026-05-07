# ORT Generation Tensor Inputs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a typed, testable tensor-input binding layer that prepares ORT-ready row-shaped `i64` buffers before session execution.

**Architecture:** Keep tensor-shape preparation in `localmt-engine-ort` because it is part of the ONNX Runtime boundary, not tokenizer semantics. `OrtI64TensorInput` owns one `[1, N]` row tensor payload, and `OrtGenerationTensorInputs` maps `OrtGenerationInputs` into encoder input ids, encoder attention mask, and decoder seed tensors. Actual `ort::value::Tensor` construction and session execution remain a later feature-gated step.

**Tech Stack:** Rust, `localmt-engine-ort`, `localmt` facade re-export, existing `OrtGenerationInputs`, TDD.

---

### Task 1: Add Failing Facade Tests

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests named:

```rust
generation_tensor_inputs_prepare_row_shapes
generation_tensor_input_row_preserves_decoder_growth
```

The first test should build existing `OrtGenerationInputs` from source tokens `[7, 8]` for `English -> Japanese`, convert it with `OrtGenerationTensorInputs::from_generation_inputs(&inputs)`, and assert:

```rust
assert_eq!(tensor_inputs.encoder_input_ids().shape(), [1, 2]);
assert_eq!(tensor_inputs.encoder_input_ids().values(), &[7, 8]);
assert_eq!(tensor_inputs.encoder_attention_mask().shape(), [1, 2]);
assert_eq!(tensor_inputs.encoder_attention_mask().values(), &[1, 1]);
assert_eq!(tensor_inputs.decoder_input_ids().shape(), [1, 1]);
assert_eq!(tensor_inputs.decoder_input_ids().values(), &[14]);
```

The second test should assert `OrtI64TensorInput::row(&[14, 21])` preserves decoder-loop growth as `shape() == [1, 2]` and `values() == &[14, 21]`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt generation_tensor
```

Expected: compile failure because the new tensor-input types are not implemented and not re-exported.

### Task 2: Implement Tensor Input Types

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add `OrtI64TensorInput`**

Add an owned public type:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtI64TensorInput {
    shape: [usize; 2],
    values: Vec<i64>,
}
```

Expose `row(values: &[i64]) -> Self`, `shape(&self) -> [usize; 2]`, and `values(&self) -> &[i64]`.

- [x] **Step 2: Add `OrtGenerationTensorInputs`**

Add an owned public type with fields for encoder input ids, encoder attention mask, and decoder input ids. Implement:

```rust
pub fn from_generation_inputs(inputs: &OrtGenerationInputs) -> Self
```

Each field must call `OrtI64TensorInput::row(...)`, producing `[1, len]` shapes.

- [x] **Step 3: Re-export from facade**

Add `OrtGenerationTensorInputs` and `OrtI64TensorInput` to the `localmt` facade export list.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt generation_tensor
```

Expected: both new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document the boundary**

Mention that `OrtGenerationTensorInputs` now prepares `[1, N]` `i64` row tensors for future `ort::value::Tensor::from_array` calls.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt generation_tensor
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

- [x] **Step 2: Commit with Lore protocol**

Create a commit explaining why row-shaped ORT tensor input binding is separated from direct session execution.
