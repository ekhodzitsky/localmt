# Contributing

`localmt` is library-first Rust infrastructure for offline mobile translation.
Keep changes small, verified, and compatible with the Android-facing FFI
contract.

## Development

Use the repository toolchain:

```bash
rustup show
cargo fmt --all --check
cargo test --locked
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo check --locked -p localmt-ffi --target aarch64-linux-android
cargo kimi check --format json
```

For release-sensitive FFI changes, also build the release artifact and verify
exported symbols:

```bash
cargo build --locked --release -p localmt-ffi --features hf-tokenizers,ort-runtime
nm -gU target/release/liblocalmt_ffi.dylib
```

On Linux, use `nm -D --defined-only target/release/liblocalmt_ffi.so`.

## Pull Requests

- Add or update tests before changing behavior.
- Keep model files out of the repository.
- Bump `LOCALMT_FFI_ABI_VERSION` when adding exported C ABI symbols.
- Update `crates/localmt-ffi/include/localmt_ffi.h` with any FFI surface change.
- Update `docs/android-build.md` and `docs/performance.md` when build or latency
  contracts change.
