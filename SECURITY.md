# Security Policy

## Supported Versions

`localmt` is pre-1.0. Security fixes target the current `main` branch.

## Reporting a Vulnerability

Please report security issues privately by opening a GitHub security advisory
for this repository. Do not file public issues for vulnerabilities.

Include:

- affected commit or version
- impact and attack surface
- reproduction steps or proof of concept
- whether model-pack contents, FFI inputs, or Android packaging are involved

## Security Boundaries

- Model-pack manifests and paths are untrusted input.
- `.localmt-trust.json` is a local hot-start optimization, not a remote trust
  root.
- FFI callers own all input/output buffers and must treat non-zero status codes
  as hard failures.
- Model files are not committed to this repository.
- The configured `libllama` dynamic library and GGUF model files are local
  trusted artifacts, not arbitrary remote inputs. Production builds must pin a
  patched llama.cpp revision and record its provenance; as of May 10, 2026,
  upstream advisories list fixes for token-to-piece vocabulary overflow in
  `b5662` and GGUF tensor-size overflow in `b7824`.
