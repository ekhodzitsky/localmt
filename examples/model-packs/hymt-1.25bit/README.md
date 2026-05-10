# Hy-MT1.5 1.25-bit GGUF Model Pack Example

This directory documents the expected local model-pack shape for
Hy-MT1.5-1.8B-1.25bit GGUF assets.

Model weights are intentionally not committed. Create a local pack by copying
`manifest.example.json` to `manifest.json`, placing the downloaded GGUF model
beside it, and replacing every example SHA-256 value with `localmt model hash`
output for the matching local file.

Expected local files:

```text
manifest.json
hymt.gguf
chat-template.jinja
llama-runtime.json
```

Only `hymt.gguf` is required by the current verifier. The chat template and
runtime config roles are optional metadata that the llama.cpp backend can use
once runtime loading is implemented.
