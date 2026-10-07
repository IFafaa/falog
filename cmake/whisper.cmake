# Included by CMake after every project() call while whisper-rs-sys builds whisper.cpp (voice input),
# through CMAKE_PROJECT_INCLUDE in .cargo/config.toml. Cargo cannot set environment variables per
# target, so compiler-specific settings live here, behind compiler checks.
#
# Two MSVC pitfalls make transcription 30-100x slower if left alone:
#
# - the `cmake` crate replaces CMAKE_<LANG>_FLAGS_RELEASE with flags that have no /O2, so the C++ code
#   is built unoptimized;
# - ggml's GGML_NATIVE cannot detect CPU features under MSVC and leaves SIMD off. Target AVX2
#   explicitly: any x86-64 CPU from 2013 on.
#
# GCC and Clang (Linux, macOS) keep ggml's defaults: GGML_NATIVE tunes the build for the CPU it is
# built on, which is what a from-source install wants.
if(MSVC)
    set(CMAKE_C_FLAGS_RELEASE "/O2 /Ob2 /DNDEBUG -nologo -MD -Brepro -W0")
    set(CMAKE_CXX_FLAGS_RELEASE "/O2 /Ob2 /DNDEBUG /utf-8 -nologo -MD -Brepro -W0")
    set(GGML_NATIVE OFF CACHE BOOL "" FORCE)
    set(GGML_AVX ON CACHE BOOL "" FORCE)
    set(GGML_AVX2 ON CACHE BOOL "" FORCE)
    set(GGML_FMA ON CACHE BOOL "" FORCE)
    set(GGML_F16C ON CACHE BOOL "" FORCE)
endif()
