#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ABI="arm64-v8a"
OUT_DIR="${ROOT_DIR}/examples/android-jni-smoke/src/main/jniLibs"
LIB_PATH="${OUT_DIR}/${ABI}/liblocalmt_ffi.so"
FEATURES="hf-tokenizers ort-runtime"

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 1
  fi
}

find_ndk_root() {
  local ndk_root
  for ndk_root in \
    "${ANDROID_NDK_HOME:-}" \
    "${ANDROID_NDK_ROOT:-}" \
    /opt/homebrew/Caskroom/android-ndk/*/*.app/Contents/NDK \
    "${HOME}/Library/Android/sdk/ndk"/*; do
    if [[ -n "${ndk_root}" && -d "${ndk_root}/toolchains/llvm/prebuilt" ]]; then
      printf '%s\n' "${ndk_root}"
      return 0
    fi
  done

  echo "missing Android NDK; set ANDROID_NDK_HOME or install the NDK" >&2
  return 1
}

find_llvm_nm() {
  if [[ -n "${LOCALMT_ANDROID_NM:-}" ]]; then
    printf '%s\n' "${LOCALMT_ANDROID_NM}"
    return 0
  fi

  if command -v llvm-nm >/dev/null 2>&1; then
    command -v llvm-nm
    return 0
  fi

  local candidate
  for candidate in \
    "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm" \
    "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-nm" \
    "${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/darwin-arm64/bin/llvm-nm"; do
    if [[ -x "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  echo "missing llvm-nm; set LOCALMT_ANDROID_NM or ANDROID_NDK_HOME" >&2
  return 1
}

require_symbol() {
  local nm_tool="$1"
  local symbol="$2"
  if ! "${nm_tool}" --defined-only "${LIB_PATH}" | grep -q "[[:space:]]${symbol}$"; then
    echo "missing exported symbol in ${LIB_PATH}: ${symbol}" >&2
    exit 1
  fi
}

require_command cargo
ANDROID_NDK_HOME="$(find_ndk_root)"
export ANDROID_NDK_HOME

if ! cargo ndk --version >/dev/null 2>&1; then
  echo "missing cargo-ndk; install it with: cargo install cargo-ndk" >&2
  exit 1
fi

cd "${ROOT_DIR}"

cargo ndk -t "${ABI}" -o "${OUT_DIR}" build \
  -p localmt-ffi \
  --release \
  --features "${FEATURES}"

if [[ ! -f "${LIB_PATH}" ]]; then
  echo "expected Android FFI artifact was not created: ${LIB_PATH}" >&2
  exit 1
fi

NM_TOOL="$(find_llvm_nm)"
require_symbol "${NM_TOOL}" "localmt_ffi_abi_version"
require_symbol "${NM_TOOL}" "localmt_ffi_model_pack_trust_schema_version"
require_symbol "${NM_TOOL}" "localmt_ffi_gguf_model_pack_summary"
require_symbol "${NM_TOOL}" "localmt_ffi_llama_runtime_enabled"
require_symbol "${NM_TOOL}" "localmt_ffi_llama_translator_open"
require_symbol "${NM_TOOL}" "localmt_ffi_llama_translate"
require_symbol "${NM_TOOL}" "localmt_ffi_ort_translator_open_trusted"
require_symbol "${NM_TOOL}" "localmt_ffi_ort_translate"

echo "packaged ${LIB_PATH}"
