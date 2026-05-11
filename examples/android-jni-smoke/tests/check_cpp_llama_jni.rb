#!/usr/bin/env ruby
# frozen_string_literal: true

cpp_path = ARGV.fetch(0)
source = File.read(cpp_path)

required_snippets = [
  'Java_dev_localmt_smoke_LocalmtNative_ggufModelPackSummary',
  'Java_dev_localmt_smoke_LocalmtNative_configureLlamaRuntime',
  'Java_dev_localmt_smoke_LocalmtNative_openGgufNative',
  'Java_dev_localmt_smoke_LocalmtNative_translateGgufNative',
  'Java_dev_localmt_smoke_LocalmtNative_closeGgufNative',
  'localmt_ffi_gguf_model_pack_summary',
  'localmt_ffi_llama_runtime_configure',
  'localmt_ffi_llama_translator_open',
  'localmt_ffi_llama_translate',
  'localmt_ffi_llama_translator_close'
]

missing = required_snippets.reject { |snippet| source.include?(snippet) }
if missing.any?
  warn "missing llama JNI snippets:"
  missing.each { |snippet| warn "  #{snippet}" }
  exit 1
end
