# ORT Encoder Run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first feature-gated ORT execution primitive: run the encoder session from prepared row tensors and extract `last_hidden_state` as owned `f32` output.

**Architecture:** Keep the primitive on `OrtEngine` behind `ort-runtime` because `ort::Session::run` requires a mutable session handle. The method accepts strict encoder tensor names plus `OrtGenerationTensorInputs`, constructs owned `ort::value::Tensor<i64>` inputs, runs the session with named inputs, and copies the named `last_hidden_state` output into an `OrtFloatTensorOutput`. Real full translation remains blocked until decoder execution and the `TokenGenerator` mutability boundary are resolved.

**Tech Stack:** Rust, `ort 2.0.0-rc.12`, feature-gated `localmt-engine-ort`, compile-focused TDD, existing ORT I/O config.

---

### Task 1: Add Compile-Focused RED Test

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing ignored test**

Add a feature-gated ignored test named:

```rust
encoder_run_method_accepts_tensor_inputs
```

The test body should only bind the signature:

```rust
fn assert_signature(
    engine: &mut OrtEngine,
    names: &OrtEncoderIoNames,
    inputs: &OrtGenerationTensorInputs,
) {
    let result: Result<OrtFloatTensorOutput, OrtEngineError> =
        engine.run_encoder(names, inputs);
    let _ = result;
}

let _signature: fn(&mut OrtEngine, &OrtEncoderIoNames, &OrtGenerationTensorInputs) =
    assert_signature;
```

Mark it ignored because executing it requires a real ONNX encoder model and runtime library; the test still compiles under `ort-runtime`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime encoder_run_method_accepts_tensor_inputs
```

Expected: compile failure because `OrtFloatTensorOutput` and `OrtEngine::run_encoder` do not exist yet.

### Task 2: Implement Encoder Run Primitive

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add output type**

Add:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct OrtFloatTensorOutput {
    shape: Vec<usize>,
    values: Vec<f32>,
}
```

Expose `shape(&self) -> &[usize]` and `values(&self) -> &[f32]`.

- [x] **Step 2: Add explicit output errors**

Extend `OrtEngineError` with:

```rust
MissingOrtOutput(String)
InvalidOrtOutputShape { output: String, dimension: i64 }
```

Display messages must include the output name and invalid dimension where relevant.

- [x] **Step 3: Add `OrtEngine::run_encoder`**

Behind `ort-runtime`, implement:

```rust
pub fn run_encoder(
    &mut self,
    names: &OrtEncoderIoNames,
    inputs: &OrtGenerationTensorInputs,
) -> Result<OrtFloatTensorOutput, OrtEngineError>
```

The method should:

1. Convert `inputs.encoder_input_ids()` and `inputs.encoder_attention_mask()` into `ort::value::Tensor<i64>` using `Tensor::from_array((shape, values.to_vec()))`.
2. Call `self.session.run(ort::inputs! { names.input_ids() => input_ids, names.attention_mask() => attention_mask })`.
3. Fetch `names.last_hidden_state()` with `SessionOutputs::get`.
4. Extract `f32` tensor data with `try_extract_tensor::<f32>()`.
5. Copy shape dimensions to `usize`, rejecting negative dimensions through `InvalidOrtOutputShape`.
6. Copy values into `OrtFloatTensorOutput`.

- [x] **Step 4: Re-export output type from facade**

Add `OrtFloatTensorOutput` to the `localmt` facade exports.

- [x] **Step 5: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime encoder_run_method_accepts_tensor_inputs
cargo check -p localmt-engine-ort --features ort-runtime
```

Expected: the ignored signature test compiles and the feature build succeeds.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document encoder execution boundary**

Mention that `OrtEngine::run_encoder` is the first feature-gated execution primitive, while full translation still needs decoder execution and generator mutability wiring.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort --features ort-runtime encoder_run_method_accepts_tensor_inputs
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

Create a commit explaining why encoder session execution is introduced before decoder generation.
