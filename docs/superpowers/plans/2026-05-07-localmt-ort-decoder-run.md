# ORT Decoder Run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a feature-gated ORT decoder execution primitive that binds decoder inputs plus encoder hidden states and extracts named logits output.

**Architecture:** Keep the primitive on `OrtEngine` beside `run_encoder`. It accepts strict decoder tensor names, prepared generation row tensors, and the owned encoder hidden-state output. It constructs `i64` tensors for decoder input ids and encoder attention mask, constructs an `f32` tensor for encoder hidden states, runs the mutable decoder session, and copies named logits into `OrtFloatTensorOutput`. The cached decoder-with-past path remains future work.

**Tech Stack:** Rust, `ort 2.0.0-rc.12`, feature-gated `localmt-engine-ort`, existing tensor wrappers, compile-focused TDD.

---

### Task 1: Add Compile-Focused RED Test

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing ignored test**

Add a feature-gated ignored test named:

```rust
decoder_run_method_accepts_encoder_hidden_states
```

The test body should bind this signature:

```rust
fn assert_signature(
    engine: &mut OrtEngine,
    names: &OrtDecoderIoNames,
    inputs: &OrtGenerationTensorInputs,
    encoder_output: &OrtFloatTensorOutput,
) {
    let result: Result<OrtFloatTensorOutput, OrtEngineError> =
        engine.run_decoder(names, inputs, encoder_output);
    let _ = result;
}

let _signature: fn(
    &mut OrtEngine,
    &OrtDecoderIoNames,
    &OrtGenerationTensorInputs,
    &OrtFloatTensorOutput,
) = assert_signature;
```

Mark it ignored because executing it requires real ONNX decoder model assets.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime decoder_run_method_accepts_encoder_hidden_states
```

Expected: compile failure because `OrtEngine::run_decoder` does not exist yet.

### Task 2: Implement Decoder Run Primitive

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Add `ort_f32_tensor` helper**

Behind `ort-runtime`, add a helper that converts `OrtFloatTensorOutput` into `ort::value::Tensor<f32>` with:

```rust
ort::value::Tensor::from_array((input.shape().to_vec(), input.values().to_vec()))
```

Map ORT construction errors into `OrtEngineError::Ort`.

- [x] **Step 2: Add `OrtEngine::run_decoder`**

Behind `ort-runtime`, implement:

```rust
pub fn run_decoder(
    &mut self,
    names: &OrtDecoderIoNames,
    inputs: &OrtGenerationTensorInputs,
    encoder_output: &OrtFloatTensorOutput,
) -> Result<OrtFloatTensorOutput, OrtEngineError>
```

The method should:

1. Convert `inputs.decoder_input_ids()` into an `i64` tensor for `names.input_ids()`.
2. Convert `inputs.encoder_attention_mask()` into an `i64` tensor for `names.encoder_attention_mask()`.
3. Convert `encoder_output` into an `f32` tensor for `names.encoder_hidden_states()`.
4. Run the session with named inputs.
5. Fetch and extract `names.logits()` as an `f32` tensor.
6. Return `OrtFloatTensorOutput`.

- [x] **Step 3: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime decoder_run_method_accepts_encoder_hidden_states
cargo check -p localmt-engine-ort --features ort-runtime
```

Expected: ignored signature test compiles and feature build succeeds.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document decoder primitive**

Mention that `OrtEngine::run_decoder` exists for the non-cached decoder graph, while logits slicing and cached decoder execution remain future work.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort --features ort-runtime decoder_run_method_accepts_encoder_hidden_states
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

Create a commit explaining why decoder session execution is separated from logits selection and cached decoding.
