# Hy-MT1.5 1.25-bit GGUF Model Pack Example

This directory documents the expected local model-pack shape for
Hy-MT1.5-1.8B-1.25bit GGUF assets.

Model weights are intentionally not committed. Create a local pack by copying
`manifest.example.json` to `manifest.json`, copying
`llama-runtime.example.json` to `llama-runtime.json`, placing the downloaded
GGUF model beside them, and replacing every example SHA-256 value with
`localmt model hash` output for the matching local file.

Expected local files:

```text
manifest.json
hymt.gguf
chat-template.jinja
llama-runtime.json
```

Only `hymt.gguf` is required by the model-pack verifier. The
`gguf-translate-smoke` readiness path also validates `llama-runtime.json` when
the manifest declares it.
