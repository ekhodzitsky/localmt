#!/usr/bin/env ruby
# frozen_string_literal: true

java_path = ARGV.fetch(0)
source = File.read(java_path)

required_snippets = [
  'public static native String ggufModelPackSummary(String modelPackPath);',
  'public static native void configureLlamaRuntime(String absoluteLibraryPath);',
  'public static GgufTranslator openGgufTranslator(String modelPackPath)',
  'public static String translateGguf(',
  'private static native long openGgufNative(String modelPackPath);',
  'private static native String translateGgufNative(',
  'private static native void closeGgufNative(long nativeTranslatorHandle);',
  'public static final class GgufTranslator implements AutoCloseable'
]

missing = required_snippets.reject { |snippet| source.include?(snippet) }
if missing.any?
  warn "missing llama Java API snippets:"
  missing.each { |snippet| warn "  #{snippet}" }
  exit 1
end
