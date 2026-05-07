# ORT Generation Step Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pure ORT generation-step helper that applies decoder logits output to `OrtGenerationState`.

**Architecture:** Keep the helper in `localmt-engine-ort` because it connects ORT decoder tensor interpretation to the ORT decoder-loop state. `OrtGenerationStep` selects the next token with `OrtDecoderLogits::select_next_token`, appends it through `OrtGenerationState::accept_next_token`, returns the accepted `TokenId`, and keeps logits/state errors distinct.

**Tech Stack:** Rust, TDD unit tests, existing `OrtDecoderLogits`, existing `OrtGenerationState`.

---

### Task 1: Add RED Unit Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add these tests near the existing decoder logits and generation-state tests:

```rust
#[test]
fn generation_step_accepts_decoder_output_into_state() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = generation_state(3);
    let output = OrtFloatTensorOutput {
        shape: vec![1, 1, 3],
        values: vec![0.1, 0.2, 5.0],
    };

    let token = OrtGenerationStep::accept_decoder_output(&mut state, &output)?;

    assert_eq!(token, TokenId::new(2));
    assert_eq!(state.generated_token_ids(), &[TokenId::new(2)]);
    assert_eq!(state.decoder_input_ids(), &[11, 2]);
    assert!(!state.is_finished());
    Ok(())
}

#[test]
fn generation_step_marks_state_finished_on_eos() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = generation_state(3);
    let output = OrtFloatTensorOutput {
        shape: vec![1, 1, 3],
        values: vec![0.1, 9.0, 0.2],
    };

    let token = OrtGenerationStep::accept_decoder_output(&mut state, &output)?;

    assert_eq!(token, TokenId::new(1));
    assert_eq!(state.generated_token_ids(), &[TokenId::new(1)]);
    assert!(state.is_finished());
    Ok(())
}

#[test]
fn generation_step_propagates_logits_error_without_state_mutation() {
    let mut state = generation_state(3);
    let before = state.clone();
    let output = OrtFloatTensorOutput {
        shape: vec![1, 3],
        values: vec![0.1, 0.2, 0.3],
    };

    let token = OrtGenerationStep::accept_decoder_output(&mut state, &output);

    assert!(matches!(
        token,
        Err(OrtGenerationStepError::DecoderLogits(
            OrtDecoderLogitsError::InvalidRank { rank: 2 }
        ))
    ));
    assert_eq!(state, before);
}

#[test]
fn generation_step_rejects_finished_state_without_extra_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = generation_state(1);
    let output = OrtFloatTensorOutput {
        shape: vec![1, 1, 3],
        values: vec![0.1, 0.2, 5.0],
    };
    OrtGenerationStep::accept_decoder_output(&mut state, &output)?;
    let before = state.clone();

    let token = OrtGenerationStep::accept_decoder_output(&mut state, &output);

    assert!(matches!(
        token,
        Err(OrtGenerationStepError::State(
            OrtGenerationStateError::AlreadyFinished
        ))
    ));
    assert_eq!(state, before);
    Ok(())
}
```

Add this test helper:

```rust
fn generation_state(max_new_tokens: usize) -> OrtGenerationState {
    OrtGenerationState {
        decoder_input_ids: vec![11],
        generated_token_ids: Vec::new(),
        max_new_tokens,
        eos_token_id: 1,
        finished: false,
    }
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort generation_step
```

Expected: compile failure because `OrtGenerationStep` and `OrtGenerationStepError` do not exist yet.

### Task 2: Implement Generation Step Helper

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add error enum**

Add:

```rust
pub enum OrtGenerationStepError {
    DecoderLogits(OrtDecoderLogitsError),
    State(OrtGenerationStateError),
}
```

Implement `Display` and `std::error::Error::source`.

- [x] **Step 2: Add helper type**

Add:

```rust
pub struct OrtGenerationStep;
```

Expose:

```rust
pub fn accept_decoder_output(
    state: &mut OrtGenerationState,
    output: &OrtFloatTensorOutput,
) -> Result<TokenId, OrtGenerationStepError>
```

Implementation must select with `OrtDecoderLogits::select_next_token(output)`, append with `state.accept_next_token(token)`, and return the accepted token.

- [x] **Step 3: Re-export from facade**

Add `OrtGenerationStep` and `OrtGenerationStepError` to the `localmt` facade exports.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort generation_step
```

Expected: new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document generation step**

Update the ORT runtime section so it says selected decoder tokens can now be applied to `OrtGenerationState` through `OrtGenerationStep::accept_decoder_output`; the remaining runtime gap is repeatedly feeding updated decoder ids through the ORT decoder loop and detokenizing real model outputs.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort generation_step
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

Create a commit explaining why decoder-output state mutation is isolated from ORT session execution.
