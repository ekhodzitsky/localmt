# Manifest Authoring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a type-safe way to create current-schema model-pack manifests and serialize them to JSON.

**Architecture:** Keep authoring in `localmt-models` beside parsing and verification so one crate owns the model-pack contract. Reuse existing validated newtypes (`ModelId`, `ModelRelativePath`, `Sha256Digest`, `ModelFileRole`) and reject duplicate languages/file roles before serialization.

**Tech Stack:** Rust workspace, `localmt-models`, existing `serde`/`serde_json` dependencies only.

---

### Task 1: Lock Authoring Behavior

**Files:**
- Modify: `crates/localmt-models/src/lib.rs`

- [x] **Step 1: Write failing tests**

Add tests for:
- `manifest_authoring_round_trips_through_discovery`
- `manifest_authoring_rejects_duplicate_file_roles`
- `manifest_authoring_rejects_duplicate_languages`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-models manifest_authoring
```

Expected: fail because `ModelManifest::new_current` and `to_json_string_pretty` do not exist yet.

### Task 2: Implement Authoring API

**Files:**
- Modify: `crates/localmt-models/src/lib.rs`

- [x] **Step 3: Add constructor and validation helpers**

Add:
- `ModelManifest::new_current(...) -> Result<Self, ModelPackError>`
- duplicate-language validation for typed `Language` values
- duplicate/empty-file validation for typed `ModelFile` values

- [x] **Step 4: Add JSON serialization**

Add:
- `ModelManifest::to_json_string_pretty(&self) -> Result<String, ModelPackError>`
- private serializable manifest/file structs
- `ModelPackError::SerializeManifest(serde_json::Error)`

- [x] **Step 5: Run GREEN**

Run:

```bash
cargo test -p localmt-models manifest_authoring
```

Expected: pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-manifest-authoring.md`

- [x] **Step 6: Document typed manifest authoring**

Mention that SDK/tooling code can build a `ModelManifest` with validated file roles, safe relative paths, and SHA-256 values, then serialize it to `manifest.json`.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-models manifest_authoring
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-models`.

- [x] **Step 8: Commit**

Commit the authoring API, tests, and docs with the Lore commit protocol.
