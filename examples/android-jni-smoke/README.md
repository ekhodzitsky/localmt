# Android JNI Smoke Harness

This directory is a minimal Android adapter surface for `localmt-ffi`. It is not
a full app module; copy or include these files from an Android app that already
owns Gradle, UI, permissions, and model-pack storage.

## Files

- `src/main/cpp/localmt_jni_smoke.cpp` - JNI bridge over the C ABI.
- `src/main/cpp/localmt_jni_utf.h` - strict UTF-16/UTF-8 conversion helpers.
- `src/main/java/dev/localmt/smoke/LocalmtNative.java` - Java declarations.
- `tests/localmt_jni_utf_test.cpp` - native conversion regression tests.
- `tests/check_java_language_constants.rb` - CI drift check for Java language ids.
- `CMakeLists.txt` - NDK build snippet that links `liblocalmt_ffi.so`.

The bridge converts Java UTF-16 strings to standard UTF-8 for Rust and converts
Rust UTF-8 output back to Java UTF-16. It does not rely on JNI modified UTF-8.
CI also compares the Java language constants against `localmt ffi startup`.
CI runs the conversion tests against ASCII, Russian, Thai, Japanese,
Vietnamese, supplementary-plane characters, malformed UTF-16, and malformed
UTF-8. CI also links the JNI bridge against a release `localmt-ffi` artifact on
the host runner, so missing C ABI symbols fail before device testing.

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
String sourceCode = LocalmtNative.languageCode(LocalmtNative.LANGUAGE_ENGLISH);
String targetCode = LocalmtNative.languageCode(LocalmtNative.LANGUAGE_RUSSIAN);
LocalmtNative.validateLanguagePair(
    LocalmtNative.LANGUAGE_ENGLISH, LocalmtNative.LANGUAGE_RUSSIAN);
try (LocalmtNative.Translator translator =
         LocalmtNative.openTrustedTranslator(modelPackDir.getAbsolutePath())) {
    String translated = translator.translate(
        LocalmtNative.LANGUAGE_ENGLISH,
        LocalmtNative.LANGUAGE_RUSSIAN,
        "hello world");
}
```

Language ids are exposed as Java constants and remain the stable FFI order from
`localmt_ffi_language_code()`:

- `0` - English
- `1` - Russian
- `2` - Thai
- `3` - Vietnamese
- `4` - Japanese

Call `localmt model trust <pack>` or `localmt_ffi_model_pack_trust()` after
installing or updating a model pack. Trusted hot-start calls reject stale trust
artifacts.
