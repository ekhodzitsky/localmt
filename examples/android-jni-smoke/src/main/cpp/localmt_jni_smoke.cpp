#include <jni.h>

#include <cstdint>
#include <limits>
#include <string>
#include <vector>

#include "localmt_jni_utf.h"
#include "localmt_ffi.h"

namespace {

static_assert(sizeof(jlong) >= sizeof(void *),
              "jlong must be able to hold a native pointer");

class JniUtf8String {
public:
  JniUtf8String(JNIEnv *env, jstring value) : env_(env), value_(value) {
    if (value_ == nullptr) {
      return;
    }

    jsize length = env_->GetStringLength(value_);
    const jchar *chars = env_->GetStringChars(value_, nullptr);
    if (chars == nullptr) {
      return;
    }
    ok_ = localmt_jni_smoke::utf16_to_utf8(
        chars, static_cast<size_t>(length), &utf8_);
    env_->ReleaseStringChars(value_, chars);
  }

  bool ok() const { return ok_; }

  bool is_null() const { return value_ == nullptr; }

  const uint8_t *bytes() const {
    return reinterpret_cast<const uint8_t *>(utf8_.data());
  }

  size_t size() const { return utf8_.size(); }

private:
  JNIEnv *env_;
  jstring value_;
  bool ok_ = false;
  std::string utf8_;
};

void throw_java(JNIEnv *env, const char *class_name, const std::string &message) {
  jclass error_class = env->FindClass(class_name);
  if (error_class != nullptr) {
    env->ThrowNew(error_class, message.c_str());
  }
}

void throw_illegal_argument(JNIEnv *env, const std::string &message) {
  throw_java(env, "java/lang/IllegalArgumentException", message);
}

void throw_string_error(JNIEnv *env, const JniUtf8String &value,
                        const char *name) {
  if (env->ExceptionCheck()) {
    return;
  }
  if (value.is_null()) {
    throw_illegal_argument(env, std::string(name) + " must be non-null");
    return;
  }
  throw_illegal_argument(env,
                         std::string(name) + " must contain valid UTF-16");
}

std::string status_message(int32_t status) {
  size_t written_len = 0;
  int32_t probe = localmt_ffi_status_message(status, nullptr, 0, &written_len);
  if (probe != LOCALMT_FFI_BUFFER_TOO_SMALL && probe != LOCALMT_FFI_OK) {
    return "localmt status " + std::to_string(status);
  }

  std::string output(written_len, '\0');
  int32_t result = localmt_ffi_status_message(
      status, reinterpret_cast<uint8_t *>(output.data()), output.size(),
      &written_len);
  if (result != LOCALMT_FFI_OK) {
    return "localmt status " + std::to_string(status);
  }
  output.resize(written_len);
  return output;
}

void throw_illegal_status(JNIEnv *env, int32_t status) {
  throw_illegal_argument(env, status_message(status));
}

void throw_localmt(JNIEnv *env, int32_t status) {
  throw_java(env, "java/lang/IllegalStateException", status_message(status));
}

jstring new_java_string_from_utf8(JNIEnv *env, const uint8_t *input,
                                  size_t len) {
  std::vector<jchar> utf16;
  if (!localmt_jni_smoke::utf8_to_utf16(input, len, &utf16)) {
    throw_java(env, "java/lang/IllegalStateException",
               "localmt returned invalid UTF-8");
    return nullptr;
  }
  if (utf16.size() >
      static_cast<size_t>(std::numeric_limits<jsize>::max())) {
    throw_java(env, "java/lang/IllegalStateException",
               "localmt output is too large for Java");
    return nullptr;
  }

  const jchar *data = utf16.empty() ? nullptr : utf16.data();
  return env->NewString(data, static_cast<jsize>(utf16.size()));
}

template <typename FfiCall> jstring call_buffer(JNIEnv *env, FfiCall call) {
  size_t written_len = 0;
  int32_t status = call(nullptr, 0, &written_len);
  if (status != LOCALMT_FFI_BUFFER_TOO_SMALL && status != LOCALMT_FFI_OK) {
    throw_localmt(env, status);
    return nullptr;
  }

  std::vector<uint8_t> output(written_len);
  status = call(output.data(), output.size(), &written_len);
  if (status != LOCALMT_FFI_OK) {
    throw_localmt(env, status);
    return nullptr;
  }

  return new_java_string_from_utf8(env, output.data(), written_len);
}

bool ensure_startup_contract(JNIEnv *env) {
  if (localmt_ffi_abi_version() != LOCALMT_FFI_ABI_VERSION) {
    throw_java(env, "java/lang/IllegalStateException",
               "localmt FFI ABI version mismatch");
    return false;
  }
  if (localmt_ffi_model_pack_trust_schema_version() !=
      LOCALMT_FFI_MODEL_PACK_TRUST_SCHEMA_VERSION) {
    throw_java(env, "java/lang/IllegalStateException",
               "localmt trust schema version mismatch");
    return false;
  }
  if (localmt_ffi_xiaomi17_android_abi_code() !=
      LOCALMT_FFI_ANDROID_ABI_ARM64_V8A) {
    throw_java(env, "java/lang/IllegalStateException",
               "localmt Android ABI mismatch");
    return false;
  }
  return true;
}

bool ensure_language_id(JNIEnv *env, jint language_id, const char *name) {
  if (language_id < 0 || language_id > 255) {
    throw_illegal_argument(env, std::string(name) + " must fit in uint8_t");
    return false;
  }
  return true;
}

bool ensure_supported_language_id(JNIEnv *env, jint language_id,
                                  const char *name) {
  if (!ensure_language_id(env, language_id, name)) {
    return false;
  }
  if (static_cast<size_t>(language_id) >=
      localmt_ffi_supported_language_count()) {
    throw_illegal_argument(env, std::string(name) + " is unsupported");
    return false;
  }
  return true;
}

jlong to_java_handle(LocalmtFfiOrtTranslator *translator) {
  return static_cast<jlong>(reinterpret_cast<std::intptr_t>(translator));
}

LocalmtFfiOrtTranslator *from_java_handle(jlong handle) {
  return reinterpret_cast<LocalmtFfiOrtTranslator *>(
      static_cast<std::intptr_t>(handle));
}

} // namespace

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_startupSummary(JNIEnv *env, jclass) {
  return call_buffer(env, [](uint8_t *output, size_t capacity, size_t *written) {
    return localmt_ffi_startup_summary(output, capacity, written);
  });
}

extern "C" JNIEXPORT jint JNICALL
Java_dev_localmt_smoke_LocalmtNative_supportedLanguageCount(JNIEnv *, jclass) {
  return static_cast<jint>(localmt_ffi_supported_language_count());
}

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_languageCode(JNIEnv *env, jclass,
                                                 jint language_id) {
  if (!ensure_supported_language_id(env, language_id, "languageId")) {
    return nullptr;
  }

  LocalmtFfiLanguageCode code =
      localmt_ffi_language_code(static_cast<uint8_t>(language_id));
  uint8_t output[2] = {code.first, code.second};
  return new_java_string_from_utf8(env, output, sizeof(output));
}

extern "C" JNIEXPORT void JNICALL
Java_dev_localmt_smoke_LocalmtNative_validateLanguagePair(
    JNIEnv *env, jclass, jint source_language_id, jint target_language_id) {
  if (!ensure_language_id(env, source_language_id, "sourceLanguageId") ||
      !ensure_language_id(env, target_language_id, "targetLanguageId")) {
    return;
  }

  int32_t status = localmt_ffi_validate_language_pair(
      static_cast<uint8_t>(source_language_id),
      static_cast<uint8_t>(target_language_id));
  if (status != LOCALMT_FFI_OK) {
    throw_illegal_status(env, status);
  }
}

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_trustedSummary(JNIEnv *env, jclass,
                                                   jstring model_pack_path) {
  if (!ensure_startup_contract(env)) {
    return nullptr;
  }

  JniUtf8String path(env, model_pack_path);
  if (!path.ok()) {
    throw_string_error(env, path, "modelPackPath");
    return nullptr;
  }

  return call_buffer(env, [&](uint8_t *output, size_t capacity, size_t *written) {
    return localmt_ffi_model_pack_trusted_summary(path.bytes(), path.size(),
                                                 output, capacity, written);
  });
}

extern "C" JNIEXPORT void JNICALL
Java_dev_localmt_smoke_LocalmtNative_configureOrtRuntime(JNIEnv *env, jclass,
                                                        jstring library_path) {
  JniUtf8String path(env, library_path);
  if (!path.ok()) {
    throw_string_error(env, path, "absoluteLibraryPath");
    return;
  }

  int32_t status = localmt_ffi_ort_runtime_configure(path.bytes(), path.size());
  if (status != LOCALMT_FFI_OK) {
    throw_localmt(env, status);
  }
}

extern "C" JNIEXPORT jlong JNICALL
Java_dev_localmt_smoke_LocalmtNative_openTrusted(JNIEnv *env, jclass,
                                                jstring model_pack_path) {
  if (!ensure_startup_contract(env)) {
    return 0;
  }

  JniUtf8String path(env, model_pack_path);
  if (!path.ok()) {
    throw_string_error(env, path, "modelPackPath");
    return 0;
  }

  LocalmtFfiOrtTranslator *raw_translator = nullptr;
  int32_t status = localmt_ffi_ort_translator_open_trusted(
      path.bytes(), path.size(), &raw_translator);
  if (status != LOCALMT_FFI_OK) {
    throw_localmt(env, status);
    return 0;
  }

  return to_java_handle(raw_translator);
}

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_translate(JNIEnv *env, jclass,
                                              jlong translator_handle,
                                              jint source_language_id,
                                              jint target_language_id,
                                              jstring text) {
  if (translator_handle == 0) {
    throw_illegal_argument(env, "nativeTranslatorHandle must be non-zero");
    return nullptr;
  }

  JniUtf8String input(env, text);
  if (!input.ok()) {
    throw_string_error(env, input, "text");
    return nullptr;
  }
  if (!ensure_language_id(env, source_language_id, "sourceLanguageId") ||
      !ensure_language_id(env, target_language_id, "targetLanguageId")) {
    return nullptr;
  }

  LocalmtFfiOrtTranslator *translator = from_java_handle(translator_handle);

  return call_buffer(env, [&](uint8_t *output, size_t capacity, size_t *written) {
    return localmt_ffi_ort_translate(
        translator, static_cast<uint8_t>(source_language_id),
        static_cast<uint8_t>(target_language_id), input.bytes(), input.size(),
        output, capacity, written);
  });
}

extern "C" JNIEXPORT void JNICALL
Java_dev_localmt_smoke_LocalmtNative_close(JNIEnv *, jclass,
                                          jlong translator_handle) {
  if (translator_handle != 0) {
    localmt_ffi_ort_translator_close(from_java_handle(translator_handle));
  }
}
