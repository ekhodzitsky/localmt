# Performance Notes

`localmt` treats performance as a release contract, not a marketing claim. All
numbers below are local baselines for detecting regressions; Xiaomi 17
measurements still need to be captured on the physical Android target.

## Current Real-Model Baseline

Command:

```bash
ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib \
  cargo run --locked --release -p localmt-cli \
  --features hf-tokenizers,ort-runtime \
  -- ffi ort-translate-bench-trusted \
  /private/tmp/localmt-nllb-pack en ru "hello world" 3
```

Model pack:

- Model: `nllb-200-distilled-600m-onnx`
- Runtime: ONNX Runtime via dynamic library
- Trust artifact: schema `1`, manifest hash plus file identity, byte length,
  and modified timestamp
- Source/target: English to Russian

Observed on the macOS development machine:

```text
ffi_abi: 13
model_pack_summary: ok
ort_translator_open: ok
ort_translate_bench: ok
runs: 3
model_pack_summary_ms: 1
ort_translator_open_ms: 12426
first_translate_ms: 1117
translate_total_ms: 2152
translate_avg_ms: 717
total_ms: 14580
translation: Привет, мир.
```

## Interpretation

The trusted startup path is no longer dominated by model-pack verification:
large-file hashing is isolated to `model trust` / install-update time. The
remaining cold-start cost is ORT session loading and graph initialization.

Warm translation is real offline inference through the Android-facing FFI
translator handle. The current NLLB pack has no `decoder_with_past` graph, so
generation still uses the non-cached decoder loop. The highest-impact next
performance work is to ship a pack with cached decoder support or a smaller
mobile-tuned model and then measure on Xiaomi 17 with the production
`libonnxruntime.so`.
