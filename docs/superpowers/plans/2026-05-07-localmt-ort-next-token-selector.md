# ORT Next Token Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic next-token selection from decoder logits before wiring real ORT decoder execution.

**Architecture:** Keep logits selection in `localmt-engine-ort` as a small no-inference primitive. `OrtNextTokenSelector::select_argmax` accepts one vocabulary logits row, rejects empty or non-finite logits, and returns the vocabulary index as `TokenId` using a stable first-index tie policy. It will later feed `OrtGenerationState::accept_next_token`.

**Tech Stack:** Rust `localmt-engine-ort`, existing `TokenId`, TDD.

---

### Task 1: Add Failing Tests

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests:

```rust
next_token_selector_selects_highest_logit
next_token_selector_keeps_first_index_on_tie
next_token_selector_rejects_empty_logits
next_token_selector_rejects_non_finite_logits
```

Expected assertions:

```rust
assert_eq!(
    OrtNextTokenSelector::select_argmax(&[0.1, 2.5, 1.3])?,
    TokenId::new(1)
);
assert_eq!(
    OrtNextTokenSelector::select_argmax(&[2.0, 2.0, 1.0])?,
    TokenId::new(0)
);
assert!(matches!(
    OrtNextTokenSelector::select_argmax(&[]),
    Err(OrtNextTokenSelectionError::EmptyLogits)
));
assert!(matches!(
    OrtNextTokenSelector::select_argmax(&[1.0, f32::NAN]),
    Err(OrtNextTokenSelectionError::NonFiniteLogit { index: 1 })
));
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt-engine-ort next_token_selector
```

Expected: compile failure because selector types do not exist.

### Task 2: Implement Selector

**Files:**
- Modify: `crates/localmt-engine-ort/src/lib.rs`

- [x] **Step 1: Add error type**

Add:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrtNextTokenSelectionError {
    EmptyLogits,
    NonFiniteLogit { index: usize },
    VocabularyIndexTooLarge { index: usize },
}
```

with `Display` and `std::error::Error`.

- [x] **Step 2: Add selector type**

Add `pub struct OrtNextTokenSelector;`.

- [x] **Step 3: Implement argmax**

Implement:

```rust
pub fn select_argmax(logits: &[f32]) -> Result<TokenId, OrtNextTokenSelectionError>
```

Rules:

- Empty logits return `EmptyLogits`.
- Any `NaN`, `+inf`, or `-inf` returns `NonFiniteLogit { index }`.
- Highest value wins.
- Ties keep the first index because the comparison uses `>`, not `>=`.
- The selected index converts to `u32` with `u32::try_from`, returning `VocabularyIndexTooLarge` on overflow.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p localmt-engine-ort next_token_selector
```

Expected: new tests pass.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document the boundary**

Mention that logits argmax selection is now deterministic and separate from ORT tensor execution.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-engine-ort next_token_selector
cargo test -p localmt-engine-ort
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

Create a commit explaining why logits selection is isolated from ORT session execution and loop state mutation.
