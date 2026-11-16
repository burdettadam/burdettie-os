#!/usr/bin/env bash
# Rebuild the project's Mach-O test inputs from their checked-in sources.
# Requires macOS and the Xcode command-line tools. Outputs stay local.
set -euo pipefail
export LC_ALL=C

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "error: build_fixtures.sh requires macOS and the Xcode command-line tools" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURES="$(cd "$SCRIPT_DIR/../fixtures" && pwd)"
CC="$(xcrun --sdk macosx --find clang)"
SDK="$(xcrun --sdk macosx --show-sdk-path)"
COMMON=(-isysroot "$SDK" -arch arm64 -O0)
OBJC=(-fno-objc-arc -fblocks)

# Compile every ordinary one-source fixture. Multi-image and architecture
# fixtures have explicit recipes below. Legacy output names remain compatible
# with the Rust/Swift test suites.
for source in "$FIXTURES"/*/main.c "$FIXTURES"/*/main.m; do
    [[ -f "$source" ]] || continue
    dir="$(dirname "$source")"
    name="$(basename "$dir")"
    output="$dir/app"
    flags=("${COMMON[@]}")
    case "$name" in
        dylib_chain|framework_style|fat_binary|weak_dylib|objc_category|objc_dylib_chain|m1_compiler_rt_probe)
            continue
            ;;
        arm64e_basic)
            echo "Building $name (arm64e)"
            "$CC" -isysroot "$SDK" -arch arm64e -O0 "$source" -o "$output"
            continue
            ;;
        dispatch_test|process_info)
            output="$dir/$name"
            ;;
        xcodebuild_simple_cli)
            # Match the checked-in Xcode project's Debug configuration.
            flags+=(-DHELLO_VALUE=41)
            ;;
        objc_minimal)
            output="$dir/objc_minimal"
            flags+=("${OBJC[@]}" -lobjc)
            ;;
        objc_inherit)
            flags+=("${OBJC[@]}" -lobjc)
            ;;
        block_basics)
            flags+=("${OBJC[@]}")
            ;;
        appkit_bootstrap|cocoa_window)
            flags+=("${OBJC[@]}" -framework Foundation -framework AppKit)
            ;;
        cocoa_controls|cocoa_draw)
            flags+=("${OBJC[@]}" -framework Foundation -framework AppKit -framework CoreGraphics)
            ;;
        *)
            if [[ "$source" == *.m ]]; then
                flags+=("${OBJC[@]}" -framework Foundation -framework CoreFoundation)
            fi
            ;;
    esac
    echo "Building $name"
    "$CC" "${flags[@]}" "$source" -o "$output"
    if [[ "$output" != "$dir/app" ]]; then
        ln -sfn "$name" "$dir/app"
    fi
done

echo "Building m1_compiler_rt_probe (arm64 + x86_64)"
dir="$FIXTURES/m1_compiler_rt_probe"
if [[ -d "$dir/app" ]]; then
    echo "error: compiler-runtime app destination is a directory: $dir/app" >&2
    exit 1
fi
M1_REFERENCE_WORK="$(mktemp -d "${TMPDIR:-/tmp}/burdettie-m1-compiler-rt.XXXXXX")"
# Quote includes must resolve against the generated header, even if the source
# directory contains an old, untracked vectors.h. Preserve failed work for diagnosis.
cp "$dir/main.c" "$dir/probe.c" "$dir/probe.h" "$M1_REFERENCE_WORK/"
python3 - "$SCRIPT_DIR" "$M1_REFERENCE_WORK/vectors.h" <<'PYM1'
import hashlib
import sys

sys.path.insert(0, sys.argv[1])
import m1_compiler_rt_vectors as vectors

data = vectors.header(vectors.corpus())
with open(sys.argv[2], 'xb') as output:
    output.write(data)
print(f"Compiler-runtime reference vectors: {len(data)} bytes, sha256 {hashlib.sha256(data).hexdigest()}")
PYM1
# The normal app output is consumed by ARM64 runtime compatibility tests. Retain
# that native slice as well as the x86_64 reference; do not route a thin x86_64
# executable into the native ARM64 entry path.
for arch in arm64 x86_64; do
    "$CC" -isysroot "$SDK" -arch "$arch" -std=c11 -O2 -Wall -Wextra -Werror \
        -fno-builtin -fno-stack-protector -fno-lto \
        "$M1_REFERENCE_WORK/main.c" "$M1_REFERENCE_WORK/probe.c" -lm \
        -o "$M1_REFERENCE_WORK/app_$arch"
done
xcrun lipo -create "$M1_REFERENCE_WORK/app_arm64" "$M1_REFERENCE_WORK/app_x86_64" \
    -output "$M1_REFERENCE_WORK/app"
xcrun lipo -verify_arch arm64 x86_64 "$M1_REFERENCE_WORK/app"
if [[ -d "$dir/app" ]]; then
    echo "error: compiler-runtime app destination is a directory: $dir/app" >&2
    exit 1
fi
mv -f "$M1_REFERENCE_WORK/app" "$dir/app"
rm -f "$M1_REFERENCE_WORK/main.c" "$M1_REFERENCE_WORK/probe.c" \
    "$M1_REFERENCE_WORK/probe.h" "$M1_REFERENCE_WORK/vectors.h" \
    "$M1_REFERENCE_WORK/app_arm64" "$M1_REFERENCE_WORK/app_x86_64"
rmdir "$M1_REFERENCE_WORK"

echo "Building return42"
"$CC" "${COMMON[@]}" "$FIXTURES/return42/return42.c" \
    -o "$FIXTURES/return42/return42"
ln -sfn return42 "$FIXTURES/return42/app"

echo "Building dylib_chain"
dir="$FIXTURES/dylib_chain"
"$CC" "${COMMON[@]}" -dynamiclib "$dir/libb.c" \
    -install_name @rpath/libchainb.dylib -o "$dir/libchainb.dylib"
"$CC" "${COMMON[@]}" -dynamiclib "$dir/liba.c" \
    -install_name @rpath/libchaina.dylib -L"$dir" -lchainb \
    -Wl,-rpath,@loader_path -o "$dir/libchaina.dylib"
"$CC" "${COMMON[@]}" "$dir/main.c" -L"$dir" -lchaina \
    -Wl,-rpath,@executable_path -o "$dir/app"

echo "Building objc_category"
dir="$FIXTURES/objc_category"
# Keep category records separate: modern linkers otherwise merge them into Greeter.
"$CC" "${COMMON[@]}" "${OBJC[@]}" "$dir/main.m" "$dir/greeter.m" "$dir/extras.m" \
    -Wl,-no_objc_category_merging -lobjc -o "$dir/objc_category"
ln -sfn objc_category "$dir/app"

echo "Building objc_dylib_chain"
dir="$FIXTURES/objc_dylib_chain"
"$CC" "${COMMON[@]}" "${OBJC[@]}" -dynamiclib "$dir/greeter.m" -lobjc \
    -install_name @rpath/libgreeter.dylib -o "$dir/libgreeter.dylib"
"$CC" "${COMMON[@]}" "${OBJC[@]}" -dynamiclib "$dir/extras.m" -lobjc \
    -install_name @rpath/libextras.dylib -L"$dir" -lgreeter \
    -Wl,-rpath,@loader_path -o "$dir/libextras.dylib"
"$CC" "${COMMON[@]}" "${OBJC[@]}" "$dir/main.m" -lobjc -L"$dir" -lgreeter -lextras \
    -Wl,-rpath,@executable_path -o "$dir/app"

echo "Building framework_style"
dir="$FIXTURES/framework_style"
bundle="$dir/MyFramework.framework"
mkdir -p "$bundle/Versions/A"
"$CC" "${COMMON[@]}" -dynamiclib "$dir/MyFramework.c" \
    -install_name @rpath/MyFramework.framework/Versions/A/MyFramework \
    -o "$bundle/Versions/A/MyFramework"
# -n avoids following an existing directory symlink on a repeated build.
ln -sfn A "$bundle/Versions/Current"
ln -sfn Versions/Current/MyFramework "$bundle/MyFramework"
"$CC" "${COMMON[@]}" "$dir/main.c" -F"$dir" -framework MyFramework \
    -Wl,-rpath,@executable_path -o "$dir/app"

echo "Building fat_binary"
dir="$FIXTURES/fat_binary"
"$CC" -isysroot "$SDK" -arch arm64 -O0 "$dir/main.c" -o "$dir/app_arm64"
"$CC" -isysroot "$SDK" -arch x86_64 -O0 "$dir/main.c" -o "$dir/app_x86_64"
xcrun lipo -create "$dir/app_arm64" "$dir/app_x86_64" -output "$dir/app"
rm -f "$dir/app_arm64" "$dir/app_x86_64"

echo "Building weak_dylib"
dir="$FIXTURES/weak_dylib"
"$CC" "${COMMON[@]}" -dynamiclib "$dir/liboptional.c" \
    -install_name @rpath/liboptional.dylib -o "$dir/liboptional.dylib"
"$CC" "${COMMON[@]}" "$dir/main.c" -L"$dir" -weak-loptional \
    -Wl,-rpath,@executable_path -o "$dir/app"

# malformed/bad_magic and malformed/truncated_header are deliberate, tiny
# parser inputs checked into source control. Preserve their exact bytes.
echo "All fixture binaries rebuilt from source."
