# ORT Generation State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic decoder-loop state machine for future ORT token generation.

**Architecture:** Keep loop state in `localmt-engine-ort` and re-export it through `localmt`. `OrtGenerationState` owns the decoder input id sequence, generated token ids, EOS id, and max-new-token limit prepared by `OrtGenerationInputs`; future ORT execution will feed logits into this state one token at a time. This remains no-inference code and does not call ONNX Runtime.

**Tech Stack:** Rust `localmt-engine-ort`, existing `OrtGenerationInputs`, existing `TokenId`, facade tests, TDD.

---

### Task 1: Add Failing Facade Tests

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests:

```rust
generation_state_appends_tokens_until_eos
generation_state_stops_at_max_new_tokens
generation_state_rejects_tokens_after_finish
```

The EOS test should prepare `OrtGenerationInputs` for `English -> Japanese`, construct `OrtGenerationState::new(inputs)`, append `TokenId::new(21)`, then append EOS `TokenId::new(1)`, and assert:

```rust
assert_eq!(state.decoder_input_ids(), &[14, 21, 1]);
assert_eq!(state.generated_token_ids(), &[TokenId::new(21), TokenId::new(1)]);
assert!(state.is_finished());
```

The max-new-token test should use max `2`, append two non-EOS tokens, and assert the state is finished. The after-finish test should use max `1`, append once, then assert the next append returns `OrtGenerationStateError::AlreadyFinished` and does not mutate state.

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt generation_state
```

Expected: compile failure because `OrtGenerationState` and `OrtGenerationStateError` are not re-exported.

### Task 2: Implement State Machine

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add error type**

Add:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtGenerationStateError {
    AlreadyFinished,
}
```

with `Display` and `std::error::Error`.

- [x] **Step 2: Add `OrtGenerationState`**

Add fields:

```rust
decoder_input_ids: Vec<i64>,
generated_token_ids: Vec<TokenId>,
max_new_tokens: usize,
eos_token_id: i64,
finished: bool,
```

- [x] **Step 3: Add constructor and accessors**

`new(inputs: OrtGenerationInputs)` copies decoder seed ids and loop limits. Add accessors for `decoder_input_ids`, `generated_token_ids`, and `is_finished`.

- [x] **Step 4: Add append behavior**

Implement:

```rust
pub fn accept_next_token(&mut self, token: TokenId) -> Result<(), OrtGenerationStateError>
```

It appends the token to generated ids and decoder input ids, then sets `finished` when token equals EOS or generated length reaches `max_new_tokens`. If already finished, return `AlreadyFinished` without mutating.

- [x] **Step 5: Re-export from facade**

Add `OrtGenerationState` and `OrtGenerationStateError` to the `localmt` re-export list.

- [x] **Step 6: Verify GREEN**

Run:

```bash
cargo test -p localmt generation_state
```

Expected: new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document the boundary**

Mention that `OrtGenerationState` now handles EOS/max-token stopping, while logits selection and ORT execution remain future work.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt generation_state
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

Create a commit explaining why decoder-loop state is separated from logits selection and ORT execution.
