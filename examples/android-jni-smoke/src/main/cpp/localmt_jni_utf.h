#ifndef LOCALMT_JNI_SMOKE_UTF_H_
#define LOCALMT_JNI_SMOKE_UTF_H_

#include <jni.h>

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace localmt_jni_smoke {

inline bool append_utf8_code_point(uint32_t code_point, std::string *output) {
  if (output == nullptr) {
    return false;
  }
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

inline bool utf16_to_utf8(const jchar *input, size_t len,
                          std::string *output) {
  if (output == nullptr || (input == nullptr && len != 0)) {
    return false;
  }

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

inline bool append_utf16_code_point(uint32_t code_point,
                                    std::vector<jchar> *output) {
  if (output == nullptr) {
    return false;
  }
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

inline bool is_utf8_continuation(uint8_t byte) {
  return (byte & 0xC0u) == 0x80u;
}

inline bool utf8_to_utf16(const uint8_t *input, size_t len,
                          std::vector<jchar> *output) {
  if (output == nullptr || (input == nullptr && len != 0)) {
    return false;
  }

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

} // namespace localmt_jni_smoke

#endif // LOCALMT_JNI_SMOKE_UTF_H_
