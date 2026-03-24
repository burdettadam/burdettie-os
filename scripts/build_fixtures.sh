#!/usr/bin/env bash
# build_fixtures.sh — Compile all test fixture binaries.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$SCRIPT_DIR/.."
FIXTURES="$ROOT/fixtures"

echo "=== Building simple_cli ==="
cc -o "$FIXTURES/simple_cli/app" "$FIXTURES/simple_cli/main.c"

echo "=== Building dylib_chain ==="
cc -dynamiclib -o "$FIXTURES/dylib_chain/libchainb.dylib" \
   -install_name @rpath/libchainb.dylib \
   "$FIXTURES/dylib_chain/libb.c"

cc -dynamiclib -o "$FIXTURES/dylib_chain/libchaina.dylib" \
   -install_name @rpath/libchaina.dylib \
   -L"$FIXTURES/dylib_chain" -lchainb \
   -Wl,-rpath,@loader_path \
   "$FIXTURES/dylib_chain/liba.c"

cc -o "$FIXTURES/dylib_chain/app" \
   -L"$FIXTURES/dylib_chain" -lchaina \
   -Wl,-rpath,@executable_path \
   "$FIXTURES/dylib_chain/main.c"

echo "=== Building constructor_test ==="
cc -o "$FIXTURES/constructor_test/app" "$FIXTURES/constructor_test/main.c"

echo "=== Building objc_foundation ==="
cc -o "$FIXTURES/objc_foundation/app" \
   -framework Foundation \
   "$FIXTURES/objc_foundation/main.m"

echo "=== Building framework_style ==="
FW_DIR="$FIXTURES/framework_style/MyFramework.framework/Versions/A"
mkdir -p "$FW_DIR"
cc -dynamiclib -o "$FW_DIR/MyFramework" \
   -install_name @rpath/MyFramework.framework/Versions/A/MyFramework \
   "$FIXTURES/framework_style/MyFramework.c"
# Create standard symlinks
ln -sf A "$FIXTURES/framework_style/MyFramework.framework/Versions/Current"
ln -sf Versions/Current/MyFramework "$FIXTURES/framework_style/MyFramework.framework/MyFramework"
# Build the app
cc -o "$FIXTURES/framework_style/app" \
   -F"$FIXTURES/framework_style" \
   -framework MyFramework \
   -Wl,-rpath,@executable_path \
   "$FIXTURES/framework_style/main.c"

echo "=== Building return42 ==="
cc -arch arm64 -O0 -o "$FIXTURES/return42/return42" "$FIXTURES/return42/return42.c"

echo "=== Building tlv_basic ==="
cc -arch arm64 -O0 -o "$FIXTURES/tlv_basic/app" "$FIXTURES/tlv_basic/main.c"

echo "=== Building objc_classes ==="
cc -arch arm64 -O0 -o "$FIXTURES/objc_classes/app" \
   -framework Foundation \
   "$FIXTURES/objc_classes/main.m"

echo "=== Building fat_binary ==="
cc -arch arm64 -O0 -o "$FIXTURES/fat_binary/app_arm64" "$FIXTURES/fat_binary/main.c" 2>/dev/null || true
cc -arch x86_64 -O0 -o "$FIXTURES/fat_binary/app_x86_64" "$FIXTURES/fat_binary/main.c" 2>/dev/null || true
if [ -f "$FIXTURES/fat_binary/app_arm64" ] && [ -f "$FIXTURES/fat_binary/app_x86_64" ]; then
    lipo -create -output "$FIXTURES/fat_binary/app" \
        "$FIXTURES/fat_binary/app_arm64" "$FIXTURES/fat_binary/app_x86_64"
    rm -f "$FIXTURES/fat_binary/app_arm64" "$FIXTURES/fat_binary/app_x86_64"
fi

echo "=== Building weak_dylib ==="
cc -dynamiclib -o "$FIXTURES/weak_dylib/liboptional.dylib" \
   -install_name @rpath/liboptional.dylib \
   "$FIXTURES/weak_dylib/liboptional.c"
cc -o "$FIXTURES/weak_dylib/app" \
   -L"$FIXTURES/weak_dylib" -weak-loptional \
   -Wl,-rpath,@executable_path \
   "$FIXTURES/weak_dylib/main.c"

echo "=== Generating malformed fixtures ==="

echo "=== Building nsstring_ops ==="
cc -arch arm64 -O0 -o "$FIXTURES/nsstring_ops/app" \
   -framework Foundation -framework CoreFoundation \
   "$FIXTURES/nsstring_ops/main.m"

echo "=== Building nsdata_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsdata_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsdata_basics/main.m"

echo "=== Building nsstring_path ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsstring_path/app" \
   -framework Foundation \
   "$FIXTURES/nsstring_path/main.m"

echo "=== Building nsprocessinfo_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsprocessinfo_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsprocessinfo_basics/main.m"

echo "=== Building nserror_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nserror_basics/app" \
   -framework Foundation \
   "$FIXTURES/nserror_basics/main.m"

echo "=== Building nsbundle_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsbundle_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsbundle_basics/main.m"

echo "=== Building nsfilemanager_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsfilemanager_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsfilemanager_basics/main.m"

echo "=== Building exception_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/exception_basics/app" \
   -framework Foundation \
   "$FIXTURES/exception_basics/main.m"

echo "=== Building real_cli ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/real_cli/app" \
   -framework Foundation \
   "$FIXTURES/real_cli/main.m"

echo "=== Building collection_enum ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/collection_enum/app" \
   -framework Foundation \
   "$FIXTURES/collection_enum/main.m"

echo "=== Building nsdate_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsdate_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsdate_basics/main.m"

echo "=== Building nsuserdefaults_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsuserdefaults_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsuserdefaults_basics/main.m"

echo "=== Building nsset_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsset_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsset_basics/main.m"

echo "=== Building nsurl_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsurl_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsurl_basics/main.m"

echo "=== Building nsnull_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsnull_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsnull_basics/main.m"

echo "=== Building nsdateformatter_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsdateformatter_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsdateformatter_basics/main.m"

echo "=== Building nscharacterset_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nscharacterset_basics/app" \
   -framework Foundation \
   "$FIXTURES/nscharacterset_basics/main.m"

echo "=== Building nsnotification_basics ==="
cc -arch arm64 -fno-objc-arc -o "$FIXTURES/nsnotification_basics/app" \
   -framework Foundation \
   "$FIXTURES/nsnotification_basics/main.m"

echo "=== Generating malformed fixtures ==="
# bad_magic: just 4 bytes of garbage
printf '\xDE\xAD\xBE\xEF' > "$FIXTURES/malformed/bad_magic"

# truncated_header: copy first 16 bytes from simple_cli
if [ -f "$FIXTURES/simple_cli/app" ]; then
    dd if="$FIXTURES/simple_cli/app" of="$FIXTURES/malformed/truncated_header" bs=1 count=16 2>/dev/null
fi

echo "=== All fixtures built ==="
