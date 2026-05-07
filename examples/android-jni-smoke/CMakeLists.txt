cmake_minimum_required(VERSION 3.22)

project(localmt_jni_smoke LANGUAGES CXX)

add_library(localmt_ffi SHARED IMPORTED)
set_target_properties(localmt_ffi PROPERTIES
  IMPORTED_LOCATION
    "${CMAKE_CURRENT_LIST_DIR}/src/main/jniLibs/${ANDROID_ABI}/liblocalmt_ffi.so"
  IMPORTED_NO_SONAME TRUE
)

add_library(localmt_jni_smoke SHARED
  src/main/cpp/localmt_jni_smoke.cpp
  src/main/cpp/localmt_jni_utf.h
)

target_compile_features(localmt_jni_smoke PRIVATE cxx_std_17)
target_include_directories(localmt_jni_smoke PRIVATE
  "${CMAKE_CURRENT_LIST_DIR}/src/main/cpp"
  "${CMAKE_CURRENT_LIST_DIR}/../../crates/localmt-ffi/include"
)
target_link_libraries(localmt_jni_smoke PRIVATE localmt_ffi)
