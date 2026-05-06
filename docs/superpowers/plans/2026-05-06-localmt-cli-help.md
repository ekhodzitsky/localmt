# CLI Help Implementation Plan

**Goal:** Add explicit development CLI help text for translation, model-pack,
and benchmark commands.

**Architecture:** Keep the CLI dependency-free and return static help strings
from the existing `run` path. This is a development tool, so the help text
should document exactly the supported command surface without introducing a CLI
framework dependency.

**Tech Stack:** Rust workspace, existing `localmt-cli` crate. No new
dependencies.

---

### Task 1: Lock Help Behavior

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing tests**

Add tests for:
- `localmt --help`
- `localmt model help`
- `localmt bench --help`

- [x] **Step 2: Run RED**

Run: `cargo test -p localmt-cli cli_prints_help`

Expected: failures because help routing is not implemented.

### Task 2: Implement Help

**Files:**
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add static help text**

Add top-level, model, and benchmark help strings.

- [x] **Step 4: Route help commands**

Route `--help` / `help` at the top level, under `model`, and under `bench`.

- [x] **Step 5: Run GREEN**

Run: `cargo test -p localmt-cli cli_prints_help`

Expected: help tests pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-cli-help.md`

- [x] **Step 6: Update docs**

Document the help commands.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-cli cli_prints_help
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo check -p localmt --features ort-runtime
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-cli`.

- [x] **Step 8: Commit**

Commit the CLI help route, tests, and docs with the Lore commit protocol.
