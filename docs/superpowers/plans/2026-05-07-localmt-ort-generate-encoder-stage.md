# ORT Generate Encoder Stage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make feature-enabled `OrtTokenGenerator::generate` execute the encoder stage before returning the explicit decoder-unimplemented boundary.

**Architecture:** Keep the shared `TokenGenerator` trait unchanged. Under `ort-runtime`, `generate(&self)` prepares `OrtGenerationInputs`, converts them into row tensors, runs `OrtEngine::run_encoder` through `OrtEngineSlot::with_mut`, and then returns `BackendUnavailable` for the decoder stage. Default builds keep the existing immediate unavailable-backend behavior.

**Tech Stack:** Rust, `localmt-engine-ort`, existing `OrtEngineSlot`, existing `OrtGenerationTensorInputs`, compile-focused TDD.

---

### Task 1: Add Compile-Focused RED Test

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing ignored test**

Add a feature-gated ignored test named:

```rust
token_generator_encoder_stage_accepts_tokenizer_output
```

The test body should bind this signature:

```rust
fn assert_signature(generator: &OrtTokenGenerator, input: &TokenizerOutput) {
    let result: Result<OrtFloatTensorOutput, OrtEngineError> =
        generator.run_encoder_for_input(input);
    let _ = result;
}

let _signature: fn(&OrtTokenGenerator, &TokenizerOutput) = assert_signature;
```

Mark it ignored because constructing a loaded feature-enabled generator requires real ONNX model assets.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime token_generator_encoder_stage_accepts_tokenizer_output
```

Expected: compile failure because `run_encoder_for_input` does not exist yet.

### Task 2: Implement Encoder Stage In Generator

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Add decoder boundary constant**

Add:

```rust
const DECODER_LOOP_UNIMPLEMENTED: &str = "ONNX decoder generation loop is not implemented";
```

- [x] **Step 2: Add `run_encoder_for_input`**

Behind `ort-runtime`, implement:

```rust
fn run_encoder_for_input(
    &self,
    input: &TokenizerOutput,
) -> Result<OrtFloatTensorOutput, OrtEngineError>
```

The helper should use:

```rust
let generation_inputs =
    OrtGenerationInputs::from_tokenizer_output(input, self.runtime_config.generation_config());
let tensor_inputs = OrtGenerationTensorInputs::from_generation_inputs(&generation_inputs);
self.encoder.with_mut(|engine| {
    engine.run_encoder(self.runtime_config.ort_io_config().encoder(), &tensor_inputs)
})
```

- [x] **Step 3: Update `TokenGenerator::generate`**

Under `ort-runtime`, call `self.run_encoder_for_input(input)` and map `OrtEngineError` into:

```rust
TokenGeneratorError::BackendUnavailable(format!("ORT encoder execution failed: {error}"))
```

If encoder execution succeeds, return:

```rust
TokenGeneratorError::BackendUnavailable(DECODER_LOOP_UNIMPLEMENTED.to_owned())
```

Under default builds, preserve the existing `GENERATION_LOOP_UNIMPLEMENTED` behavior.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort --features ort-runtime token_generator_encoder_stage_accepts_tokenizer_output
cargo check -p localmt-engine-ort --features ort-runtime
```

Expected: ignored signature test compiles and the feature build succeeds.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document generate behavior**

Mention that runtime-enabled generation now reaches the encoder stage before stopping at the decoder-unimplemented boundary.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort --features ort-runtime token_generator_encoder_stage_accepts_tokenizer_output
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

Create a commit explaining why generation now reaches encoder execution while decoder generation remains explicit future work.
