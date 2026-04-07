#!/usr/bin/env bash
# libressl-bundle/build.sh — Cross-compile LibreSSL for Linux ARM64 (LOCAL TESTING ONLY).
#
# NOTE: The Docker image builds LibreSSL natively in its libressl-builder stage.
# This script is retained for local debugging/experimentation only.
#
# KNOWN ISSUE: zig cross-compilation produces broken EC point arithmetic
# ("point is not on curve"). The Docker native build does not have this bug.
#
# Prerequisites:
#   brew install cmake zig
#
# Output:
#   libressl-bundle/lib/libcrypto.so
#   libressl-bundle/lib/libssl.so
#   libressl-bundle/lib/libtls.so

set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────

LIBRESSL_VERSION="3.3.6"
LIBRESSL_SHA256="87b1bbe36e9eec8d1bb2f4e033a91db69e555966"  # SHA-1 of tarball (for integrity)
LIBRESSL_URL="https://ftp.openbsd.org/pub/OpenBSD/LibreSSL/libressl-${LIBRESSL_VERSION}.tar.gz"

# Cross-compile target
TARGET_ARCH="aarch64"
TARGET_TRIPLE="aarch64-linux-gnu"

# ── Paths ──────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="/tmp/libressl-build"
SRC_DIR="${BUILD_DIR}/libressl-${LIBRESSL_VERSION}"
INSTALL_DIR="${BUILD_DIR}/install"
OUT_DIR="${SCRIPT_DIR}/lib"

# ── Preflight ──────────────────────────────────────────────────────

for cmd in cmake zig; do
    if ! command -v "$cmd" &>/dev/null; then
        echo "error: $cmd not found. Install with: brew install $cmd" >&2
        exit 1
    fi
done

echo "=== LibreSSL ${LIBRESSL_VERSION} cross-compile for ${TARGET_TRIPLE} ==="

# ── Download ───────────────────────────────────────────────────────

mkdir -p "${BUILD_DIR}"

TARBALL="${BUILD_DIR}/libressl-${LIBRESSL_VERSION}.tar.gz"
if [[ ! -f "${TARBALL}" ]]; then
    echo "Downloading LibreSSL ${LIBRESSL_VERSION}..."
    curl -fSL "${LIBRESSL_URL}" -o "${TARBALL}"
else
    echo "Using cached tarball: ${TARBALL}"
fi

# ── Extract ────────────────────────────────────────────────────────

if [[ ! -d "${SRC_DIR}" ]]; then
    echo "Extracting..."
    tar -xzf "${TARBALL}" -C "${BUILD_DIR}"
fi

# ── Create zig-based cross-compile wrapper scripts ─────────────────
# CMake needs CC/AR/RANLIB executables; zig provides all of these
# via "zig cc", "zig ar", "zig ranlib" with --target flag.

WRAPPER_DIR="${BUILD_DIR}/zig-wrappers"
mkdir -p "${WRAPPER_DIR}"

cat > "${WRAPPER_DIR}/cc" << 'WRAPPER_CC'
#!/bin/sh
exec zig cc -target aarch64-linux-gnu.2.36 "$@"
WRAPPER_CC
chmod +x "${WRAPPER_DIR}/cc"

cat > "${WRAPPER_DIR}/ar" << 'WRAPPER_AR'
#!/bin/sh
exec zig ar "$@"
WRAPPER_AR
chmod +x "${WRAPPER_DIR}/ar"

cat > "${WRAPPER_DIR}/ranlib" << 'WRAPPER_RANLIB'
#!/bin/sh
exec zig ranlib "$@"
WRAPPER_RANLIB
chmod +x "${WRAPPER_DIR}/ranlib"

# ── CMake toolchain file ──────────────────────────────────────────

TOOLCHAIN_FILE="${BUILD_DIR}/zig-toolchain.cmake"
cat > "${TOOLCHAIN_FILE}" << CMAKE_TC
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)
set(CMAKE_C_COMPILER "${WRAPPER_DIR}/cc")
set(CMAKE_AR "${WRAPPER_DIR}/ar")
set(CMAKE_RANLIB "${WRAPPER_DIR}/ranlib")
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
CMAKE_TC

# ── Build ──────────────────────────────────────────────────────────

CMAKE_BUILD_DIR="${BUILD_DIR}/build"
rm -rf "${CMAKE_BUILD_DIR}" "${INSTALL_DIR}"
mkdir -p "${CMAKE_BUILD_DIR}" "${INSTALL_DIR}"

echo "Configuring..."
cmake -S "${SRC_DIR}" -B "${CMAKE_BUILD_DIR}" \
    -DCMAKE_TOOLCHAIN_FILE="${TOOLCHAIN_FILE}" \
    -DCMAKE_INSTALL_PREFIX="${INSTALL_DIR}" \
    -DCMAKE_BUILD_TYPE=Release \
    -DBUILD_SHARED_LIBS=ON \
    -DLIBRESSL_APPS=OFF \
    -DLIBRESSL_TESTS=OFF \
    -DENABLE_ASM=OFF \
    -DCMAKE_C_FLAGS="-fPIC" \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    2>&1 | tail -5

echo "Building..."
cmake --build "${CMAKE_BUILD_DIR}" -j "$(sysctl -n hw.ncpu 2>/dev/null || nproc)" 2>&1 | tail -5

echo "Installing..."
cmake --install "${CMAKE_BUILD_DIR}" 2>&1 | tail -5

# ── Collect outputs ────────────────────────────────────────────────

mkdir -p "${OUT_DIR}"

# Copy the actual .so files (not symlinks)
for lib in libcrypto libssl libtls; do
    # Find the real file (e.g., libcrypto.so.50.0.1)
    REAL_FILE=$(find "${INSTALL_DIR}/lib" -name "${lib}.so.*.*.*" -type f 2>/dev/null | head -1)
    if [[ -z "${REAL_FILE}" ]]; then
        # Fallback: try .so.X.Y pattern
        REAL_FILE=$(find "${INSTALL_DIR}/lib" -name "${lib}.so.*.*" -type f 2>/dev/null | head -1)
    fi
    if [[ -z "${REAL_FILE}" ]]; then
        # Fallback: try plain .so
        REAL_FILE=$(find "${INSTALL_DIR}/lib" -name "${lib}.so" -type f 2>/dev/null | head -1)
    fi

    if [[ -n "${REAL_FILE}" ]]; then
        cp "${REAL_FILE}" "${OUT_DIR}/${lib}.so"
        echo "  ${lib}.so  ($(du -h "${OUT_DIR}/${lib}.so" | cut -f1))"
    else
        echo "warning: ${lib}.so not found in ${INSTALL_DIR}/lib" >&2
        ls -la "${INSTALL_DIR}/lib/" >&2
    fi
done

# ── Verify ─────────────────────────────────────────────────────────

echo ""
echo "=== Verification ==="
for lib in "${OUT_DIR}"/*.so; do
    ARCH=$(file "$lib" | grep -oE "ARM aarch64|x86-64" || echo "unknown")
    echo "  $(basename "$lib"): ${ARCH}"
done

echo ""
echo "=== Done ==="
echo "Output: ${OUT_DIR}/"
ls -lh "${OUT_DIR}/"
