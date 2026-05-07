# ORT Non-Cached Generation Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the non-cached ORT decoder generation loop boundary and wire feature-enabled `OrtTokenGenerator::generate` through it.

**Architecture:** Add a pure `OrtGenerationLoop::run_non_cached` helper that owns loop control while accepting a decoder callback. The helper rebuilds tensor inputs from immutable `OrtGenerationInputs` plus `OrtGenerationState`, applies decoder outputs through `OrtGenerationStep`, and returns a bounded `TokenSequence`; feature-enabled `OrtTokenGenerator::generate` supplies the real ORT decoder callback.

**Tech Stack:** Rust, TDD unit tests, existing ORT state/logits/tensor helpers, feature-gated ORT session execution.

---

### Task 1: Add RED Unit Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add these tests near the generation step tests:

```rust
#[test]
fn generation_loop_runs_until_eos() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = raw_generation_inputs();
    let mut calls = 0_usize;
    let mut decoder_rows = Vec::<Vec<i64>>::new();

    let tokens = OrtGenerationLoop::run_non_cached(&inputs, |tensor_inputs| {
        calls += 1;
        decoder_rows.push(tensor_inputs.decoder_input_ids().values().to_vec());
        let output = if calls == 1 {
            decoder_logits([0.1, 0.2, 5.0])
        } else {
            decoder_logits([0.1, 9.0, 0.2])
        };

        Ok(output)
    })?;

    assert_eq!(tokens.as_slice(), &[TokenId::new(2), TokenId::new(1)]);
    assert_eq!(decoder_rows, vec![vec![11], vec![11, 2]]);
    assert_eq!(calls, 2);
    Ok(())
}

#[test]
fn generation_loop_stops_at_max_new_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = raw_generation_inputs_with_limit(2);
    let mut calls = 0_usize;

    let tokens = OrtGenerationLoop::run_non_cached(&inputs, |_tensor_inputs| {
        calls += 1;
        Ok(decoder_logits([0.1, 0.2, 5.0]))
    })?;

    assert_eq!(tokens.as_slice(), &[TokenId::new(2), TokenId::new(2)]);
    assert_eq!(calls, 2);
    Ok(())
}

#[test]
fn generation_loop_propagates_decoder_error() {
    let inputs = raw_generation_inputs();

    let tokens = OrtGenerationLoop::run_non_cached(&inputs, |_tensor_inputs| {
        Err(OrtGenerationLoopError::Decoder(
            "synthetic decoder failure".to_owned(),
        ))
    });

    assert!(matches!(
        tokens,
        Err(OrtGenerationLoopError::Decoder(ref reason))
            if reason == "synthetic decoder failure"
    ));
}
```

Add or update these helpers:

```rust
fn raw_generation_inputs() -> OrtGenerationInputs {
    raw_generation_inputs_with_limit(3)
}

fn raw_generation_inputs_with_limit(max_new_tokens: usize) -> OrtGenerationInputs {
    OrtGenerationInputs {
        encoder_input_ids: vec![7, 8],
        encoder_attention_mask: vec![1, 1],
        decoder_input_ids: vec![11],
        max_new_tokens,
        eos_token_id: 1,
    }
}

fn decoder_logits(values: [f32; 3]) -> OrtFloatTensorOutput {
    OrtFloatTensorOutput {
        shape: vec![1, 1, 3],
        values: values.to_vec(),
    }
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort generation_loop
```

Expected: compile failure because `OrtGenerationLoop` and `OrtGenerationLoopError` do not exist yet.

### Task 2: Implement Pure Loop

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add loop error enum**

Add:

```rust
pub enum OrtGenerationLoopError {
    Decoder(String),
    Step(OrtGenerationStepError),
    InvalidGeneratedTokens(String),
}
```

Implement `Display` and `std::error::Error::source`, with `source` returning the wrapped step error for `Step`.

- [x] **Step 2: Add pure loop helper**

Add:

```rust
pub struct OrtGenerationLoop;

pub fn run_non_cached<F>(
    inputs: &OrtGenerationInputs,
    decode_next: F,
) -> Result<TokenSequence, OrtGenerationLoopError>
where
    F: FnMut(&OrtGenerationTensorInputs) -> Result<OrtFloatTensorOutput, OrtGenerationLoopError>
```

The loop must:
- create `OrtGenerationState::new(inputs.clone())`
- while state is not finished, build `OrtGenerationTensorInputs::from_generation_state(inputs, &state)`
- call `decode_next`
- apply output through `OrtGenerationStep::accept_decoder_output`
- return `TokenSequence::new(state.generated_token_ids().to_vec())`

- [x] **Step 3: Re-export from facade**

Add `OrtGenerationLoop` and `OrtGenerationLoopError` to the `localmt` facade exports.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort generation_loop
```

Expected: new tests pass.

### Task 3: Wire Feature Generate

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `README.md`

- [x] **Step 1: Replace feature-gated decoder-unimplemented return**

In `#[cfg(feature = "ort-runtime")] impl TokenGenerator for OrtTokenGenerator`, replace the final `DECODER_LOOP_UNIMPLEMENTED` error with `OrtGenerationLoop::run_non_cached` using `self.decoder.with_mut(...)` and `OrtEngine::run_decoder(...)`.

Map loop errors to `TokenGeneratorError`:

```rust
fn map_ort_generation_loop_error(error: OrtGenerationLoopError) -> TokenGeneratorError
```

Use `BackendUnavailable` for decoder/step failures and `InvalidGeneratedTokens` for invalid generated token sequences.

- [x] **Step 2: Document non-cached runtime loop**

Update README so the ORT runtime section says feature-enabled generation now runs encoder plus the non-cached decoder loop; cached decoder-with-past and real model asset validation remain future runtime checks.

- [x] **Step 3: Verify feature build**

Run:

```bash
cargo check -p localmt-engine-ort --features ort-runtime
```

Expected: feature build compiles with the wired decoder loop.

### Task 4: Full Verification And Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort generation_loop
cargo test -p localmt-engine-ort
cargo check -p localmt-engine-ort --features ort-runtime
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

- [x] **Step 2: Run Kimi**

Run `cargo kimi check` from `crates/localmt-engine-ort`.

- [x] **Step 3: Commit with Lore protocol**

Create a commit explaining why the ORT decoder loop is callback-driven and non-cached first.
