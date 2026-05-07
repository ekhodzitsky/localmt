#include "localmt_jni_utf.h"

#include <cassert>
#include <cstdint>
#include <string>
#include <string_view>
#include <vector>

namespace {

std::vector<jchar> to_jchars(std::u16string_view input) {
  std::vector<jchar> output;
  output.reserve(input.size());
  for (char16_t value : input) {
    output.push_back(static_cast<jchar>(value));
  }
  return output;
}

std::vector<uint8_t> to_bytes(std::string_view input) {
  std::vector<uint8_t> output;
  output.reserve(input.size());
  for (char value : input) {
    output.push_back(static_cast<uint8_t>(value));
  }
  return output;
}

void assert_utf16_to_utf8(std::u16string_view input,
                          std::string_view expected) {
  std::vector<jchar> utf16 = to_jchars(input);
  std::string output;
  assert(localmt_jni_smoke::utf16_to_utf8(utf16.data(), utf16.size(), &output));
  assert(output == expected);
}

void assert_utf8_to_utf16(std::string_view input,
                          std::u16string_view expected) {
  std::vector<uint8_t> utf8 = to_bytes(input);
  std::vector<jchar> output;
  assert(localmt_jni_smoke::utf8_to_utf16(utf8.data(), utf8.size(), &output));
  assert(output == to_jchars(expected));
}

void assert_utf16_to_utf8_rejects(std::u16string_view input) {
  std::vector<jchar> utf16 = to_jchars(input);
  std::string output;
  assert(!localmt_jni_smoke::utf16_to_utf8(utf16.data(), utf16.size(), &output));
}

void assert_utf8_to_utf16_rejects(std::vector<uint8_t> input) {
  std::vector<jchar> output;
  assert(!localmt_jni_smoke::utf8_to_utf16(input.data(), input.size(), &output));
}

void assert_round_trip(std::u16string_view utf16, std::string_view utf8) {
  assert_utf16_to_utf8(utf16, utf8);
  assert_utf8_to_utf16(utf8, utf16);
}

} // namespace

int main() {
  assert_round_trip(u"", "");
  assert_round_trip(u"hello", "hello");
  assert_round_trip(u"Привет", u8"Привет");
  assert_round_trip(u"สวัสดี", u8"สวัสดี");
  assert_round_trip(u"こんにちは", u8"こんにちは");
  assert_round_trip(u"xin chào", u8"xin chào");

  std::u16string supplementary;
  supplementary.push_back(static_cast<char16_t>(0xD83D));
  supplementary.push_back(static_cast<char16_t>(0xDE00));
  assert_round_trip(supplementary, "\xF0\x9F\x98\x80");

  assert_utf16_to_utf8_rejects(std::u16string(1, static_cast<char16_t>(0xD800)));
  assert_utf16_to_utf8_rejects(std::u16string(1, static_cast<char16_t>(0xDC00)));

  std::u16string broken_pair;
  broken_pair.push_back(static_cast<char16_t>(0xD800));
  broken_pair.push_back(u'A');
  assert_utf16_to_utf8_rejects(broken_pair);

  assert_utf8_to_utf16_rejects({0x80u});
  assert_utf8_to_utf16_rejects({0xC0u, 0x80u});
  assert_utf8_to_utf16_rejects({0xE0u, 0x80u});
  assert_utf8_to_utf16_rejects({0xEDu, 0xA0u, 0x80u});
  assert_utf8_to_utf16_rejects({0xF4u, 0x90u, 0x80u, 0x80u});

  return 0;
}
