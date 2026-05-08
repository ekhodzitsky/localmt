#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOCALMT_LIB="${ROOT_DIR}/examples/android-jni-smoke/src/main/jniLibs/arm64-v8a/liblocalmt_ffi.so"
REMOTE_DIR="/data/local/tmp/localmt-smoke"
DEVICE_SERIAL=""
ORT_RUNTIME=""
MODEL_PACK=""

usage() {
  cat <<'USAGE'
usage: scripts/android-device-preflight.sh [options]

Checks a connected Android device before running the JNI smoke adapter.

Options:
  --device SERIAL       adb device serial to use when multiple devices exist
  --remote-dir PATH     remote staging directory (default: /data/local/tmp/localmt-smoke)
  --ort-runtime PATH    local libonnxruntime.so to push beside liblocalmt_ffi.so
  --model-pack PATH     local model-pack directory to push for translation smoke
  -h, --help            show this help

Run scripts/package-android-ffi.sh first so liblocalmt_ffi.so exists.
USAGE
}

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "missing required command: ${command_name}" >&2
    exit 1
  fi
}

take_value() {
  local option="$1"
  local value="${2:-}"
  if [[ -z "${value}" || "${value}" == --* ]]; then
    echo "${option} requires a value" >&2
    exit 1
  fi
  printf '%s\n' "${value}"
}

adb_cmd() {
  if [[ -n "${DEVICE_SERIAL}" ]]; then
    adb -s "${DEVICE_SERIAL}" "$@"
    return
  fi
  adb "$@"
}

validate_remote_dir() {
  local path="$1"
  if [[ ! "${path}" =~ ^/data/local/tmp/[A-Za-z0-9._/-]+$ ]]; then
    echo "remote dir must be under /data/local/tmp and use safe path characters: ${path}" >&2
    exit 1
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --device)
      DEVICE_SERIAL="$(take_value "$1" "${2:-}")"
      shift 2
      ;;
    --remote-dir)
      REMOTE_DIR="$(take_value "$1" "${2:-}")"
      shift 2
      ;;
    --ort-runtime)
      ORT_RUNTIME="$(take_value "$1" "${2:-}")"
      shift 2
      ;;
    --model-pack)
      MODEL_PACK="$(take_value "$1" "${2:-}")"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_command adb
validate_remote_dir "${REMOTE_DIR}"

if [[ ! -f "${LOCALMT_LIB}" ]]; then
  echo "missing ${LOCALMT_LIB}" >&2
  echo "run scripts/package-android-ffi.sh first" >&2
  exit 1
fi
if [[ -n "${ORT_RUNTIME}" && ! -f "${ORT_RUNTIME}" ]]; then
  echo "missing ORT runtime library: ${ORT_RUNTIME}" >&2
  exit 1
fi
if [[ -n "${MODEL_PACK}" && ! -d "${MODEL_PACK}" ]]; then
  echo "missing model-pack directory: ${MODEL_PACK}" >&2
  exit 1
fi

adb_cmd wait-for-device

DEVICE_STATE="$(adb_cmd get-state)"
if [[ "${DEVICE_STATE}" != "device" ]]; then
  echo "adb device is not ready: ${DEVICE_STATE}" >&2
  exit 1
fi

ABILIST="$(adb_cmd shell getprop ro.product.cpu.abilist | tr -d '\r')"
if [[ ",${ABILIST}," != *",arm64-v8a,"* ]]; then
  echo "connected device does not advertise arm64-v8a: ${ABILIST}" >&2
  exit 1
fi

MANUFACTURER="$(adb_cmd shell getprop ro.product.manufacturer | tr -d '\r')"
MODEL="$(adb_cmd shell getprop ro.product.model | tr -d '\r')"
ANDROID_RELEASE="$(adb_cmd shell getprop ro.build.version.release | tr -d '\r')"
ANDROID_SDK="$(adb_cmd shell getprop ro.build.version.sdk | tr -d '\r')"

echo "device: ${MANUFACTURER} ${MODEL}"
echo "android: ${ANDROID_RELEASE} (sdk ${ANDROID_SDK})"
echo "abis: ${ABILIST}"

adb_cmd shell mkdir -p "${REMOTE_DIR}/lib" "${REMOTE_DIR}/model-pack"
adb_cmd push "${LOCALMT_LIB}" "${REMOTE_DIR}/lib/liblocalmt_ffi.so" >/dev/null
echo "pushed ${REMOTE_DIR}/lib/liblocalmt_ffi.so"

if [[ -n "${ORT_RUNTIME}" ]]; then
  adb_cmd push "${ORT_RUNTIME}" "${REMOTE_DIR}/lib/libonnxruntime.so" >/dev/null
  echo "pushed ${REMOTE_DIR}/lib/libonnxruntime.so"
fi

if [[ -n "${MODEL_PACK}" ]]; then
  adb_cmd push "${MODEL_PACK}/." "${REMOTE_DIR}/model-pack" >/dev/null
  echo "pushed model pack to ${REMOTE_DIR}/model-pack"
fi

adb_cmd shell ls -l "${REMOTE_DIR}/lib"
echo "device preflight complete"
