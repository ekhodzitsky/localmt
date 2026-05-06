# Xiaomi 17 Profile Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Xiaomi 17 benchmark profile expose target ABI, RAM class, and preferred runtime metadata.

**Architecture:** Keep metadata on `DeviceProfile` in `localmt-bench` because benchmark interpretation belongs with the device profile. Surface the same metadata in `BenchReport`/CLI output so benchmark smoke runs describe the target class, not only the profile id.

**Tech Stack:** Rust workspace, `localmt-bench`, `localmt-cli`, no new dependencies.

---

### Task 1: Lock Metadata Behavior

**Files:**
- Modify: `crates/localmt-bench/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 1: Write failing tests**

Add tests that assert:
- `DeviceProfile::Xiaomi17.android_abi() == "arm64-v8a"`
- `DeviceProfile::Xiaomi17.ram_class_gib() == 12`
- `DeviceProfile::Xiaomi17.preferred_runtime() == "onnx-runtime-mobile-xnnpack"`
- benchmark CLI output includes `android_abi`, `ram_class_gib`, and `preferred_runtime`

- [x] **Step 2: Run RED**

Run:

```bash
cargo test -p localmt-bench device_profile_parses_xiaomi17
cargo test -p localmt-cli cli_runs_mock_pipeline_benchmark_for_verified_pack
```

Expected: fail because metadata accessors and CLI output are not implemented.

### Task 2: Implement Metadata

**Files:**
- Modify: `crates/localmt-bench/src/lib.rs`
- Modify: `crates/localmt-cli/src/main.rs`

- [x] **Step 3: Add DeviceProfile accessors**

Add:
- `android_abi`
- `ram_class_gib`
- `preferred_runtime`

- [x] **Step 4: Surface metadata through BenchReport and CLI**

Add corresponding `BenchReport` methods and output lines.

- [x] **Step 5: Run GREEN**

Run the RED commands again and expect pass.

### Task 3: Document And Verify

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/plans/2026-05-06-localmt-xiaomi17-profile-metadata.md`

- [x] **Step 6: Update README**

Mention the Xiaomi 17 profile metadata in the benchmark section.

- [x] **Step 7: Run verification**

Run:

```bash
cargo fmt --check
git diff --check
cargo test -p localmt-bench device_profile
cargo test -p localmt-cli cli_runs_mock_pipeline_benchmark_for_verified_pack
cargo test
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo kimi check
```

Run `cargo kimi check` from `crates/localmt-bench`.

- [x] **Step 8: Commit**

Commit the profile metadata, CLI output, and docs with the Lore commit protocol.
