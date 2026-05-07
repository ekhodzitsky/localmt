#include <jni.h>

#include <cstdint>
#include <limits>
#include <string>
#include <vector>

#include "localmt_ffi.h"

namespace {

static_assert(sizeof(jlong) >= sizeof(void *),
              "jlong must be able to hold a native pointer");

bool append_utf8_code_point(uint32_t code_point, std::string *output) {
  if (code_point <= 0x7Fu) {
    output->push_back(static_cast<char>(code_point));
    return true;
  }
  if (code_point <= 0x7FFu) {
    output->push_back(static_cast<char>(0xC0u | (code_point >> 6u)));
    output->push_back(static_cast<char>(0x80u | (code_point & 0x3Fu)));
    return true;
  }
  if (code_point >= 0xD800u && code_point <= 0xDFFFu) {
    return false;
  }
  if (code_point <= 0xFFFFu) {
    output->push_back(static_cast<char>(0xE0u | (code_point >> 12u)));
    output->push_back(
        static_cast<char>(0x80u | ((code_point >> 6u) & 0x3Fu)));
    output->push_back(static_cast<char>(0x80u | (code_point & 0x3Fu)));
    return true;
  }
  if (code_point <= 0x10FFFFu) {
    output->push_back(static_cast<char>(0xF0u | (code_point >> 18u)));
    output->push_back(
        static_cast<char>(0x80u | ((code_point >> 12u) & 0x3Fu)));
    output->push_back(
        static_cast<char>(0x80u | ((code_point >> 6u) & 0x3Fu)));
    output->push_back(static_cast<char>(0x80u | (code_point & 0x3Fu)));
    return true;
  }
  return false;
}

bool utf16_to_utf8(const jchar *input, size_t len, std::string *output) {
  output->clear();
  for (size_t i = 0; i < len; ++i) {
    uint32_t code_point = input[i];
    if (code_point >= 0xD800u && code_point <= 0xDBFFu) {
      if (i + 1 >= len) {
        return false;
      }
      uint32_t low = input[++i];
      if (low < 0xDC00u || low > 0xDFFFu) {
        return false;
      }
      code_point =
          0x10000u + ((code_point - 0xD800u) << 10u) + (low - 0xDC00u);
    } else if (code_point >= 0xDC00u && code_point <= 0xDFFFu) {
      return false;
    }

    if (!append_utf8_code_point(code_point, output)) {
      return false;
    }
  }
  return true;
}

bool append_utf16_code_point(uint32_t code_point, std::vector<jchar> *output) {
  if (code_point <= 0xFFFFu) {
    if (code_point >= 0xD800u && code_point <= 0xDFFFu) {
      return false;
    }
    output->push_back(static_cast<jchar>(code_point));
    return true;
  }
  if (code_point <= 0x10FFFFu) {
    uint32_t adjusted = code_point - 0x10000u;
    output->push_back(static_cast<jchar>(0xD800u | (adjusted >> 10u)));
    output->push_back(static_cast<jchar>(0xDC00u | (adjusted & 0x3FFu)));
    return true;
  }
  return false;
}

bool is_utf8_continuation(uint8_t byte) { return (byte & 0xC0u) == 0x80u; }

bool utf8_to_utf16(const uint8_t *input, size_t len,
                   std::vector<jchar> *output) {
  output->clear();
  size_t offset = 0;
  while (offset < len) {
    uint8_t first = input[offset];
    uint32_t code_point = 0;
    size_t width = 0;
    uint32_t min_code_point = 0;

    if (first <= 0x7Fu) {
      code_point = first;
      width = 1;
    } else if ((first & 0xE0u) == 0xC0u) {
      code_point = first & 0x1Fu;
      width = 2;
      min_code_point = 0x80u;
    } else if ((first & 0xF0u) == 0xE0u) {
      code_point = first & 0x0Fu;
      width = 3;
      min_code_point = 0x800u;
    } else if ((first & 0xF8u) == 0xF0u) {
      code_point = first & 0x07u;
      width = 4;
      min_code_point = 0x10000u;
    } else {
      return false;
    }

    if (offset + width > len) {
      return false;
    }
    for (size_t i = 1; i < width; ++i) {
      uint8_t next = input[offset + i];
      if (!is_utf8_continuation(next)) {
        return false;
      }
      code_point = (code_point << 6u) | (next & 0x3Fu);
    }

    if (code_point < min_code_point ||
        (code_point >= 0xD800u && code_point <= 0xDFFFu) ||
        code_point > 0x10FFFFu) {
      return false;
    }
    if (!append_utf16_code_point(code_point, output)) {
      return false;
    }
    offset += width;
  }
  return true;
}

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
    ok_ = utf16_to_utf8(chars, static_cast<size_t>(length), &utf8_);
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

void throw_localmt(JNIEnv *env, int32_t status) {
  throw_java(env, "java/lang/IllegalStateException", status_message(status));
}

jstring new_java_string_from_utf8(JNIEnv *env, const uint8_t *input,
                                  size_t len) {
  std::vector<jchar> utf16;
  if (!utf8_to_utf16(input, len, &utf16)) {
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
