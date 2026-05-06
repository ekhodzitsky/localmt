# Project Guidelines

> Generated from kimi-dotfiles/templates/rust/rust-only
> Includes: base@v1.3.0 + rust@v1.3.0
>
> <!-- Strictness: standard -->

## Base Rules

- **Types prove invariants** — encode constraints in Newtype / Phantom / Typestate
- **Functions have contracts** — Hoare triple in every doc comment
- **No unwrap/expect/panic** without compile-time proof
- **Property tests** for algebraic axioms
- **Standard patterns** — no custom DSLs

## Rust-Specific Rules

### Types

- Newtype for every semantic distinction: `struct Price(u64)` not `u64`
- Phantom types for dimensions: `Quantity<Meters>`
- Typestate for state machines: `Socket<Connected>`
- `NonZeroU32`, `NonZeroU64` where applicable

### Functions

- Hoare triple in every doc comment:
```rust
/// { !items.is_empty() }
/// fn average(items: &[f64]) -> f64
/// { ret == sum(items) / items.len() }
```
- `debug_assert!` for preconditions not in types
- Max 40 lines
- No nesting > 3 levels

### Algebraic Structures

```rust
pub trait Semigroup: Clone + PartialEq {
    fn combine(&self, other: &Self) -> Self;
}

pub trait Monoid: Semigroup {
    fn identity() -> Self;
}
```

Property tests MUST verify all axioms.

### Unsafe

Every `unsafe` block requires `// SAFETY:` proof + Miri check.

### Testing

- `#[cfg(test)]` in same file
- `proptest` for property verification
- Doc tests for examples

### Automation

Clippy configuration is installed based on your chosen strictness level:
- `relaxed` — warnings only, no CI breaks
- `standard` — deny unwrap/panic, warn on others (default)
- `strict` — deny everything

Run: `cargo test`, `cargo clippy -- -D warnings`, `cargo doc --no-deps`

### Mechanized Checks

Use `cargo kimi check` to verify contracts are present.

Use `cargo kimi verify` to machine-check contracts with Kani (optional but recommended for critical functions).

---

## Project-Specific Rules

- This repository is library-first. Keep app/mobile shells as adapters around the Rust API.
- Model files are not committed to the Rust crates. Load them through verified model packs.
- The first supported device profile is Xiaomi 17 / Android arm64-v8a.
- Keep inference backends behind traits so ONNX Runtime, Candle, or future engines can be swapped.
