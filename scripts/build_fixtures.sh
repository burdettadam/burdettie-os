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
        dylib_chain|framework_style|fat_binary|weak_dylib|objc_category|objc_dylib_chain)
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
"$CC" "${COMMON[@]}" "${OBJC[@]}" "$dir/main.m" "$dir/greeter.m" "$dir/extras.m" \
    -lobjc -o "$dir/objc_category"
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
