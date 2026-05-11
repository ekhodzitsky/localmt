#!/usr/bin/env ruby
# frozen_string_literal: true

package_script = File.read(ARGV.fetch(0))
preflight_script = File.read(ARGV.fetch(1))

requirements = {
  'package script enables llama-runtime feature' =>
    package_script.include?('llama-runtime'),
  'package script checks llama close symbol' =>
    package_script.include?('localmt_ffi_llama_translator_close'),
  'preflight accepts --llama-runtime' =>
    preflight_script.include?('--llama-runtime PATH'),
  'preflight validates llama runtime file' =>
    preflight_script.include?('LLAMA_RUNTIME'),
  'preflight pushes libllama.so' =>
    preflight_script.include?('libllama.so')
}

failed = requirements.reject { |_name, ok| ok }.keys
if failed.any?
  warn 'missing Android llama script support:'
  failed.each { |name| warn "  #{name}" }
  exit 1
end
