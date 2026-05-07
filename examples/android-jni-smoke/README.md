# Android JNI Smoke Harness

This directory is a minimal Android adapter surface for `localmt-ffi`. It is not
a full app module; copy or include these files from an Android app that already
owns Gradle, UI, permissions, and model-pack storage.

## Files

- `src/main/cpp/localmt_jni_smoke.cpp` - JNI bridge over the C ABI.
- `src/main/java/dev/localmt/smoke/LocalmtNative.java` - Java declarations.
- `CMakeLists.txt` - NDK build snippet that links `liblocalmt_ffi.so`.

## Packaging

Build the Rust library for Android:

```bash
cargo ndk -t arm64-v8a -o examples/android-jni-smoke/src/main/jniLibs \
  build -p localmt-ffi --release --features "hf-tokenizers ort-runtime"
```

The expected artifact path is:

```text
examples/android-jni-smoke/src/main/jniLibs/arm64-v8a/liblocalmt_ffi.so
```

Your Android app must also package the matching ONNX Runtime Mobile
`libonnxruntime.so` beside `liblocalmt_ffi.so`.

## App Integration

Point the Android module's external native build at this `CMakeLists.txt`, or
copy the C++ source into the app's native build. At startup:

```java
String startup = LocalmtNative.startupSummary();
LocalmtNative.configureOrtRuntime(
    context.getApplicationInfo().nativeLibraryDir + "/libonnxruntime.so");
String ready = LocalmtNative.trustedSummary(modelPackDir.getAbsolutePath());
String translated = LocalmtNative.translateTrusted(
    modelPackDir.getAbsolutePath(), 0, 1, "hello world");
```

Language ids are the stable FFI order from `localmt_ffi_language_code()`:

- `0` - English
- `1` - Russian
- `2` - Thai
- `3` - Vietnamese
- `4` - Japanese

Call `localmt model trust <pack>` or `localmt_ffi_model_pack_trust()` after
installing or updating a model pack. Trusted hot-start calls reject stale trust
artifacts.
