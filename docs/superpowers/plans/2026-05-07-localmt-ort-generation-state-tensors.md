# ORT Generation State Tensors Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow ORT decoder tensor inputs to be rebuilt from the current decoder-loop state.

**Architecture:** Keep encoder input ids and encoder attention mask anchored in the immutable `OrtGenerationInputs`, but take decoder input ids from `OrtGenerationState`. This lets the runtime loop run the non-cached decoder repeatedly with the same encoder-side tensors and a growing decoder row.

**Tech Stack:** Rust, TDD unit tests, existing `OrtGenerationInputs`, `OrtGenerationState`, and `OrtGenerationTensorInputs`.

---

### Task 1: Add RED Unit Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add these tests near the existing generation tensor/state tests:

```rust
#[test]
fn generation_tensor_inputs_from_state_match_initial_inputs() {
    let inputs = raw_generation_inputs();
    let state = OrtGenerationState::new(inputs.clone());

    let tensor_inputs = OrtGenerationTensorInputs::from_generation_state(&inputs, &state);

    assert_eq!(tensor_inputs.encoder_input_ids().shape(), [1, 2]);
    assert_eq!(tensor_inputs.encoder_input_ids().values(), &[7, 8]);
    assert_eq!(tensor_inputs.encoder_attention_mask().shape(), [1, 2]);
    assert_eq!(tensor_inputs.encoder_attention_mask().values(), &[1, 1]);
    assert_eq!(tensor_inputs.decoder_input_ids().shape(), [1, 1]);
    assert_eq!(tensor_inputs.decoder_input_ids().values(), &[11]);
}

#[test]
fn generation_tensor_inputs_from_state_follow_decoder_growth()
-> Result<(), Box<dyn std::error::Error>> {
    let inputs = raw_generation_inputs();
    let mut state = OrtGenerationState::new(inputs.clone());
    state.accept_next_token(TokenId::new(21))?;
    state.accept_next_token(TokenId::new(22))?;

    let tensor_inputs = OrtGenerationTensorInputs::from_generation_state(&inputs, &state);

    assert_eq!(tensor_inputs.encoder_input_ids().values(), &[7, 8]);
    assert_eq!(tensor_inputs.encoder_attention_mask().values(), &[1, 1]);
    assert_eq!(tensor_inputs.decoder_input_ids().shape(), [1, 3]);
    assert_eq!(tensor_inputs.decoder_input_ids().values(), &[11, 21, 22]);
    Ok(())
}
```

Add this test helper:

```rust
fn raw_generation_inputs() -> OrtGenerationInputs {
    OrtGenerationInputs {
        encoder_input_ids: vec![7, 8],
        encoder_attention_mask: vec![1, 1],
        decoder_input_ids: vec![11],
        max_new_tokens: 3,
        eos_token_id: 1,
    }
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort generation_tensor_inputs_from_state
```

Expected: compile failure because `OrtGenerationTensorInputs::from_generation_state` does not exist yet.

### Task 2: Implement State-Aware Tensor Inputs

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Add constructor**

Add this public constructor to `impl OrtGenerationTensorInputs`:

```rust
pub fn from_generation_state(
    inputs: &OrtGenerationInputs,
    state: &OrtGenerationState,
) -> Self
```

Implementation:

```rust
Self {
    encoder_input_ids: OrtI64TensorInput::row(inputs.encoder_input_ids()),
    encoder_attention_mask: OrtI64TensorInput::row(inputs.encoder_attention_mask()),
    decoder_input_ids: OrtI64TensorInput::row(state.decoder_input_ids()),
}
```

- [x] **Step 2: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort generation_tensor_inputs_from_state
```

Expected: new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document state-aware tensor rebuilding**

Update the ORT runtime section to mention `OrtGenerationTensorInputs::from_generation_state` as the bridge that rebuilds decoder input rows after each accepted token.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort generation_tensor_inputs_from_state
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

Create a commit explaining why decoder tensors are rebuilt from explicit generation state instead of mutating tensor inputs in place.
