#!/usr/bin/env bash
# Build the alternate-string-layout libc++ used by the Linux ARM64 guest lane.
# No Apple SDK, extracted dylib, or prebuilt library is required.
set -euo pipefail

OUTPUT_DIR="${1:?usage: build.sh OUTPUT_DIRECTORY}"
LLVM_TAG=llvmorg-14.0.6
LLVM_COMMIT=f28c006a5895fc0e329fe15fead81e37457cb1d1
BUILD_JOBS="${BUILD_JOBS:-2}"

for tool in git cmake ninja "${CC:-clang}" "${CXX:-clang++}"; do
    command -v "$tool" >/dev/null || { echo "error: $tool is required" >&2; exit 1; }
done

mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

# The release tag is useful to humans; the commit check is the immutable pin.
# Sparse checkout limits the download to the runtime and CMake sources.
git -c advice.detachedHead=false clone --depth 1 --filter=blob:none --sparse \
    --branch "$LLVM_TAG" https://github.com/llvm/llvm-project.git "$WORK_DIR/source"
test "$(git -C "$WORK_DIR/source" rev-parse HEAD)" = "$LLVM_COMMIT"
git -C "$WORK_DIR/source" sparse-checkout set \
    libcxx libcxxabi runtimes cmake llvm/cmake third-party

cmake -G Ninja -S "$WORK_DIR/source/runtimes" -B "$WORK_DIR/build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$WORK_DIR/install" \
    -DCMAKE_C_COMPILER="${CC:-clang}" \
    -DCMAKE_CXX_COMPILER="${CXX:-clang++}" \
    '-DLLVM_ENABLE_RUNTIMES=libcxx;libcxxabi' \
    -DLLVM_INCLUDE_TESTS=OFF \
    -DLLVM_INCLUDE_DOCS=OFF \
    -DLIBCXX_INCLUDE_TESTS=OFF \
    -DLIBCXXABI_INCLUDE_TESTS=OFF \
    -DLIBCXX_ENABLE_SHARED=ON \
    -DLIBCXX_ENABLE_STATIC=OFF \
    -DLIBCXX_ENABLE_EXPERIMENTAL_LIBRARY=OFF \
    -DLIBCXXABI_ENABLE_SHARED=ON \
    -DLIBCXXABI_ENABLE_STATIC=OFF \
    -DLIBCXX_ABI_VERSION=1 \
    -DLIBCXX_ABI_DEFINES=_LIBCPP_ABI_ALTERNATE_STRING_LAYOUT
cmake --build "$WORK_DIR/build" --target cxx cxxabi --parallel "$BUILD_JOBS"
cmake --build "$WORK_DIR/build" --target install-cxx install-cxxabi

# Exercise both the installed headers and out-of-line string operations in the
# new library. This is an ABI smoke test, not proof of arbitrary guest support.
cat > "$WORK_DIR/string-layout.cpp" <<'CPP'
#include <cassert>
#include <cstring>
#include <string>
#ifndef _LIBCPP_ABI_ALTERNATE_STRING_LAYOUT
#error Alternate string layout was not configured
#endif
int main() {
    const std::string small("hello");
    assert(std::memcmp(&small, "hello", 5) == 0);
    std::string large(100, 'x');
    large.append(100, 'y');
    assert(large.size() == 200);
    assert(large.substr(0, 100) == std::string(100, 'x'));
    assert(large.substr(100) == std::string(100, 'y'));
    large.erase(0, 198);
    large.shrink_to_fit();
    assert(large == "yy");
}
CPP
"${CXX:-clang++}" -std=c++17 -nostdinc++ -nodefaultlibs \
    -I"$WORK_DIR/install/include/c++/v1" "$WORK_DIR/string-layout.cpp" \
    -L"$WORK_DIR/install/lib" -Wl,-rpath,"$WORK_DIR/install/lib" \
    -lc++ -lc++abi -lpthread -lm -lc -lgcc_s -lgcc \
    -o "$WORK_DIR/string-layout"
"$WORK_DIR/string-layout"

# The runtime image provides Debian's matching libc++abi.so.1. Only libc++
# needs the alternate string layout. Do not replace a host system library.
install -m755 "$WORK_DIR/install/lib/libc++.so.1.0" "$OUTPUT_DIR/libc++.so.1.0"
install -m644 "$WORK_DIR/source/libcxx/LICENSE.TXT" "$OUTPUT_DIR/LICENSE-libcxx.txt"
install -m644 "$WORK_DIR/source/libcxxabi/LICENSE.TXT" "$OUTPUT_DIR/LICENSE-libcxxabi.txt"
printf 'LLVM tag: %s\nLLVM commit: %s\nABI defines: %s\n' \
    "$LLVM_TAG" "$LLVM_COMMIT" _LIBCPP_ABI_ALTERNATE_STRING_LAYOUT \
    > "$OUTPUT_DIR/SOURCE.txt"
