#include <jni.h>

#include <cstdint>
#include <string>
#include <vector>

#include "localmt_ffi.h"

namespace {

class JniUtf8String {
public:
  JniUtf8String(JNIEnv *env, jstring value) : env_(env), value_(value) {
    if (value_ != nullptr) {
      chars_ = env_->GetStringUTFChars(value_, nullptr);
    }
  }

  JniUtf8String(const JniUtf8String &) = delete;
  JniUtf8String &operator=(const JniUtf8String &) = delete;

  ~JniUtf8String() {
    if (chars_ != nullptr) {
      env_->ReleaseStringUTFChars(value_, chars_);
    }
  }

  bool ok() const { return value_ != nullptr && chars_ != nullptr; }

  const uint8_t *bytes() const {
    return reinterpret_cast<const uint8_t *>(chars_);
  }

  size_t size() const {
    return static_cast<size_t>(env_->GetStringUTFLength(value_));
  }

private:
  JNIEnv *env_;
  jstring value_;
  const char *chars_ = nullptr;
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

void throw_localmt(JNIEnv *env, int32_t status) {
  throw_java(env, "java/lang/IllegalStateException", status_message(status));
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

  return env->NewStringUTF(
      std::string(reinterpret_cast<const char *>(output.data()), written_len)
          .c_str());
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

class OrtTranslator {
public:
  explicit OrtTranslator(LocalmtFfiOrtTranslator *handle) : handle_(handle) {}

  OrtTranslator(const OrtTranslator &) = delete;
  OrtTranslator &operator=(const OrtTranslator &) = delete;

  ~OrtTranslator() { localmt_ffi_ort_translator_close(handle_); }

  const LocalmtFfiOrtTranslator *get() const { return handle_; }

private:
  LocalmtFfiOrtTranslator *handle_;
};

} // namespace

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_startupSummary(JNIEnv *env, jclass) {
  return call_buffer(env, [](uint8_t *output, size_t capacity, size_t *written) {
    return localmt_ffi_startup_summary(output, capacity, written);
  });
}

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_trustedSummary(JNIEnv *env, jclass,
                                                   jstring model_pack_path) {
  if (!ensure_startup_contract(env)) {
    return nullptr;
  }

  JniUtf8String path(env, model_pack_path);
  if (!path.ok()) {
    throw_illegal_argument(env, "modelPackPath must be a non-null UTF-8 string");
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
    throw_illegal_argument(env,
                           "absoluteLibraryPath must be a non-null UTF-8 string");
    return;
  }

  int32_t status = localmt_ffi_ort_runtime_configure(path.bytes(), path.size());
  if (status != LOCALMT_FFI_OK) {
    throw_localmt(env, status);
  }
}

extern "C" JNIEXPORT jstring JNICALL
Java_dev_localmt_smoke_LocalmtNative_translateTrusted(
    JNIEnv *env, jclass, jstring model_pack_path, jint source_language_id,
    jint target_language_id, jstring text) {
  if (!ensure_startup_contract(env)) {
    return nullptr;
  }

  JniUtf8String path(env, model_pack_path);
  JniUtf8String input(env, text);
  if (!path.ok()) {
    throw_illegal_argument(env, "modelPackPath must be a non-null UTF-8 string");
    return nullptr;
  }
  if (!input.ok()) {
    throw_illegal_argument(env, "text must be a non-null UTF-8 string");
    return nullptr;
  }
  if (!ensure_language_id(env, source_language_id, "sourceLanguageId") ||
      !ensure_language_id(env, target_language_id, "targetLanguageId")) {
    return nullptr;
  }

  LocalmtFfiOrtTranslator *raw_translator = nullptr;
  int32_t status = localmt_ffi_ort_translator_open_trusted(
      path.bytes(), path.size(), &raw_translator);
  if (status != LOCALMT_FFI_OK) {
    throw_localmt(env, status);
    return nullptr;
  }
  OrtTranslator translator(raw_translator);

  return call_buffer(env, [&](uint8_t *output, size_t capacity, size_t *written) {
    return localmt_ffi_ort_translate(
        translator.get(), static_cast<uint8_t>(source_language_id),
        static_cast<uint8_t>(target_language_id), input.bytes(), input.size(),
        output, capacity, written);
  });
}
