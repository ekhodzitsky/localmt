# ORT Decoder Logits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pure decoder-logits helper that extracts the final generated-position vocabulary row from decoder output and selects the next token.

**Architecture:** Keep the helper in `localmt-engine-ort` because it interprets ONNX decoder tensor shape. `OrtDecoderLogits` treats decoder logits as `[batch, sequence, vocabulary]`, accepts only batch `1`, validates element counts, returns the final sequence row, and delegates argmax token selection to `OrtNextTokenSelector`.

**Tech Stack:** Rust, pure unit tests, existing `OrtFloatTensorOutput`, existing `OrtNextTokenSelector`.

---

### Task 1: Add RED Unit Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests:

```rust
decoder_logits_extract_final_token_row
decoder_logits_select_next_token_from_final_row
decoder_logits_rejects_non_three_dimensional_output
decoder_logits_rejects_mismatched_element_count
```

Use `OrtFloatTensorOutput { shape: vec![1, 2, 3], values: vec![0.1, 0.2, 0.3, 4.0, 2.0, 1.0] }`. The final row must be `[4.0, 2.0, 1.0]`, and selected token must be `TokenId::new(0)`.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort decoder_logits
```

Expected: compile failure because `OrtDecoderLogits` and `OrtDecoderLogitsError` do not exist yet.

### Task 2: Implement Decoder Logits Helper

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add error enum**

Add:

```rust
pub enum OrtDecoderLogitsError {
    InvalidRank { rank: usize },
    InvalidBatch { batch: usize },
    EmptySequence,
    EmptyVocabulary,
    ElementCountOverflow,
    ElementCountMismatch { expected: usize, actual: usize },
    Selection(OrtNextTokenSelectionError),
}
```

Implement `Display` and `std::error::Error`.

- [x] **Step 2: Add helper type**

Add:

```rust
pub struct OrtDecoderLogits;
```

Expose:

```rust
pub fn final_token_logits(output: &OrtFloatTensorOutput) -> Result<&[f32], OrtDecoderLogitsError>
pub fn select_next_token(output: &OrtFloatTensorOutput) -> Result<TokenId, OrtDecoderLogitsError>
```

- [x] **Step 3: Re-export from facade**

Add `OrtDecoderLogits` and `OrtDecoderLogitsError` to the `localmt` facade exports.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort decoder_logits
```

Expected: new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document logits helper**

Mention that decoder logits row extraction and argmax token selection are now isolated before generator state mutation.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort decoder_logits
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

Create a commit explaining why logits row extraction is separated from decoder execution and state mutation.
