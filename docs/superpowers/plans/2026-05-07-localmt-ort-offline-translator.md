# ORT Offline Translator Facade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a feature-gated facade translator that combines the real HF tokenizer with `OrtTokenGenerator`.

**Architecture:** Keep the type in the `localmt` facade next to `HfMockOfflineTranslator`. `OrtOfflineTranslator` is compiled only when both `hf-tokenizers` and `ort-runtime` are enabled, owns prepared `OfflineTranslatorAssets`, builds `HfTokenizer` from the verified tokenizer path, loads `OrtTokenGenerator` from the verified generator plan, and delegates translation to `TranslationPipeline`.

**Tech Stack:** Rust, facade unit tests, existing `HfTokenizer`, `OrtTokenGenerator`, `TranslationPipeline`.

---

### Task 1: Add RED Unit Tests

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add these tests in the facade test module:

```rust
#[test]
#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
fn ort_offline_translator_maps_missing_pack_to_assets_error()
-> Result<(), Box<dyn std::error::Error>> {
    let path = create_temp_dir()?.join("missing-pack");

    let translator = OrtOfflineTranslator::from_model_pack_path(&path);

    assert!(matches!(
        translator,
        Err(OrtOfflineTranslatorError::Assets(_))
    ));
    Ok(())
}

#[test]
#[ignore = "compile-only signature guard; construction requires tokenizer and ONNX model assets"]
#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]
fn ort_offline_translator_exposes_translate_boundary() {
    fn assert_signature(translator: &OrtOfflineTranslator, request: &TranslateRequest) {
        let result: Result<Translation, TranslationError> = translator.translate(request);
        let _ = result;
    }

    let _signature: fn(&OrtOfflineTranslator, &TranslateRequest) = assert_signature;
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p localmt --all-features ort_offline_translator
```

Expected: compile failure because `OrtOfflineTranslator` and `OrtOfflineTranslatorError` do not exist yet.

### Task 2: Implement Facade Translator

**Files:**
- Modify: `crates/localmt/src/lib.rs`

- [x] **Step 1: Add translator type**

Add under `#[cfg(all(feature = "hf-tokenizers", feature = "ort-runtime"))]`:

```rust
pub struct OrtOfflineTranslator {
    assets: OfflineTranslatorAssets,
    pipeline: TranslationPipeline<HfTokenizer, OrtTokenGenerator>,
}
```

Implement:

```rust
pub fn from_model_pack_path(path: impl AsRef<Path>) -> Result<Self, OrtOfflineTranslatorError>
pub fn from_assets(assets: OfflineTranslatorAssets) -> Result<Self, OrtOfflineTranslatorError>
pub const fn assets(&self) -> &OfflineTranslatorAssets
pub fn translate(&self, request: &TranslateRequest) -> Result<Translation, TranslationError>
```

Also implement `TranslatorEngine` by delegating to `self.pipeline.translate(request)`.

- [x] **Step 2: Add error enum**

Add:

```rust
pub enum OrtOfflineTranslatorError {
    Assets(OfflineTranslatorAssetsError),
    Tokenizer(TokenizerError),
    Generator(OrtEngineError),
}
```

Implement `Display` and `std::error::Error::source`.

- [x] **Step 3: Verify GREEN**

Run:

```bash
cargo test -p localmt --all-features ort_offline_translator
```

Expected: the missing-pack test passes and the compile-only boundary test is ignored.

### Task 3: Docs And Verification

**Files:**
- Modify: `README.md`

- [x] **Step 1: Document ORT offline translator**

Update README to describe `OrtOfflineTranslator` as the SDK-level real tokenizer plus ORT generator facade behind `hf-tokenizers + ort-runtime`, with real model asset execution still requiring local ONNX Runtime and compatible model files.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt --all-features ort_offline_translator
cargo test -p localmt --all-features
cargo check -p localmt --features "hf-tokenizers ort-runtime"
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
```

### Task 4: Commit

**Files:**
- Commit all touched files.

- [x] **Step 1: Run Kimi**

Run `cargo kimi check` from `crates/localmt`.

- [x] **Step 2: Commit with Lore protocol**

Create a commit explaining why the real tokenizer plus ORT generator translator is feature-gated as a facade type before adding FFI translation handles.
