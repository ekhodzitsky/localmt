# ORT Generation Inputs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add typed ORT generation inputs that bridge tokenizer output and strict runtime config before actual ONNX execution.

**Architecture:** Keep the type in `localmt-engine-ort` because it is ONNX input-shape preparation, not tokenizer or facade behavior, and re-export it from `localmt` for adapters. `OrtGenerationInputs::from_tokenizer_output` converts semantic `TokenId` values to ONNX-friendly `i64` vectors, builds an all-ones encoder attention mask, starts decoder input with the configured target-language token, and carries max-new-token plus EOS limits from `GenerationConfig`. This remains no-inference code and does not touch ONNX Runtime sessions.

**Tech Stack:** Rust `localmt-engine-ort`, existing `TokenizerOutput`, existing `GenerationConfig`, existing language/token invariants, TDD.

---

### Task 1: Add Failing Facade Tests

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests:

```rust
generation_inputs_prepare_encoder_and_decoder_seed
generation_inputs_preserve_source_token_order
```

The first test should build `TokenizerOutput` for `English -> Japanese` with source tokens `[7, 8]` and a `GenerationConfig` where Japanese maps to token `14`, max-new-token is `3`, and EOS is `1`. It should assert:

```rust
assert_eq!(inputs.encoder_input_ids(), &[7, 8]);
assert_eq!(inputs.encoder_attention_mask(), &[1, 1]);
assert_eq!(inputs.decoder_input_ids(), &[14]);
assert_eq!(inputs.max_new_tokens(), 3);
assert_eq!(inputs.eos_token_id(), 1);
```

The second test should use source tokens `[42, 7, 42]` and assert the encoder input ids preserve duplicates and order.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt generation_inputs
```

Expected: compile failure because `OrtGenerationInputs` is not re-exported.

### Task 2: Implement Typed Inputs

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add `OrtGenerationInputs`**

Add:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrtGenerationInputs {
    encoder_input_ids: Vec<i64>,
    encoder_attention_mask: Vec<i64>,
    decoder_input_ids: Vec<i64>,
    max_new_tokens: usize,
    eos_token_id: i64,
}
```

- [x] **Step 2: Add constructor**

Implement:

```rust
pub fn from_tokenizer_output(input: &TokenizerOutput, config: GenerationConfig) -> Self
```

Mapping rules:

- `encoder_input_ids`: each source token id converted to `i64`.
- `encoder_attention_mask`: one `1_i64` for each source token.
- `decoder_input_ids`: one element, the configured `target_language_token(input.target())`.
- `max_new_tokens`: `config.max_new_tokens().value()`.
- `eos_token_id`: `config.eos_token_id()`.

- [x] **Step 3: Add accessors**

Add `encoder_input_ids`, `encoder_attention_mask`, `decoder_input_ids`, `max_new_tokens`, and `eos_token_id` accessors with Hoare-style docs.

- [x] **Step 4: Re-export from facade**

Add `OrtGenerationInputs` to the `pub use localmt_engine_ort::{ ... }` list in `crates/localmt/src/lib.rs`.

- [x] **Step 5: Verify GREEN**

Run:

```bash
cargo test -p localmt generation_inputs
```

Expected: new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document the boundary**

Mention that `OrtGenerationInputs` now prepares ONNX-friendly token id vectors and masks, but `OrtTokenGenerator::generate` still returns the explicit unavailable-backend error until encoder/decoder session execution is wired.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt generation_inputs
cargo test -p localmt-engine-ort
cargo test -p localmt
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

Create a commit explaining why ONNX generation input preparation is separated from actual ORT session execution.
