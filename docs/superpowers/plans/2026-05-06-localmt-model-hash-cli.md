# Model Hash CLI Implementation Plan

**Goal:** Let users compute manifest-ready SHA-256 digests for local model-pack
files through the public model layer and development CLI.

**Architecture:** Expose file hashing on `Sha256Digest` in `localmt-models` so
the CLI and future tooling use the same digest implementation as verification.
Add `localmt model hash <file>` as a narrow authoring helper.

**Tech Stack:** Rust workspace, existing `localmt-models` and `localmt-cli`
crates. No new dependencies.

---

### Task 1: Lock Public Hashing

**Files:**
- Modify: `crates/localmt-models/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing model-layer test**

Add a test for `Sha256Digest::from_file` on a local fixture file.

- [x] **Step 2: Write failing CLI test**

Add a test for `localmt model hash <file>`.

- [x] **Step 3: Run RED**

Run:

```bash
cargo test -p localmt-models sha256_digest_hashes_local_file
cargo test -p localmt-cli cli_hashes_model_pack_file_for_manifest
```

Expected: compile failures because the public hashing API and CLI command do
not exist.

### Task 2: Implement Hash Helper

**Files:**
- Modify: `crates/localmt-models/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 4: Expose file hashing**

Add `Sha256Digest::from_file(path)` that delegates to the existing verifier
hash implementation.

- [x] **Step 5: Add `model hash` routing**

Route `localmt model hash <file>` and print a compact digest line.

- [x] **Step 6: Run GREEN**

Run the two targeted tests from Step 3.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-05-06-localmt-library-design.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-model-hash-cli.md`

- [x] **Step 7: Update docs**

Document `localmt model hash` as a manifest authoring helper.

- [x] **Step 8: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-models sha256_digest_hashes_local_file
cargo test -p localmt-cli cli_hashes_model_pack_file_for_manifest
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-models` and `crates/localmt-cli`.

- [x] **Step 9: Commit**

Commit the public hash helper, CLI command, tests, and docs with the Lore
commit protocol.
