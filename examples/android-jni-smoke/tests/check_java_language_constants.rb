# frozen_string_literal: true

java_path, startup_path = ARGV
abort("usage: check_java_language_constants.rb LocalmtNative.java startup.txt") unless java_path && startup_path

java_source = File.read(java_path)
startup_summary = File.read(startup_path)

languages_line = startup_summary.lines.find { |line| line.start_with?("languages: ") }
abort("startup summary is missing languages line") unless languages_line

language_names = {
  "en" => "ENGLISH",
  "ru" => "RUSSIAN",
  "th" => "THAI",
  "vi" => "VIETNAMESE",
  "ja" => "JAPANESE"
}

language_codes = languages_line.delete_prefix("languages: ").strip.split(", ")
expected_constants = language_codes.each_with_index.to_h do |code, index|
  name = language_names.fetch(code) { abort("unsupported Java constant name for language code #{code}") }
  ["LANGUAGE_#{name}", index]
end

actual_constants = java_source
                   .scan(/public static final int (LANGUAGE_[A-Z_]+) = ([0-9]+);/)
                   .to_h { |name, value| [name, Integer(value, 10)] }

abort("Java language constants drifted: expected #{expected_constants}, got #{actual_constants}") unless actual_constants == expected_constants

puts "Java language constants match localmt ffi startup"
