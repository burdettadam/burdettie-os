# Alternate-layout libc++ build

The Linux ARM64 runtime uses LLVM libc++ with
`_LIBCPP_ABI_ALTERNATE_STRING_LAYOUT` for guest `std::string` compatibility.
This directory contains the build recipe and upstream notices. No prebuilt
library or extracted Apple SDK is needed by the Docker build.

## Source and configuration

| Input | Pin |
| --- | --- |
| Upstream | [llvm/llvm-project](https://github.com/llvm/llvm-project) |
| Release | `llvmorg-14.0.6` |
| Verified commit | `f28c006a5895fc0e329fe15fead81e37457cb1d1` |
| libc++ ABI version | `1` |
| Additional ABI define | `_LIBCPP_ABI_ALTERNATE_STRING_LAYOUT` |

`build.sh` checks the downloaded release against the pinned commit before
building. It uses LLVM's [runtimes build](https://github.com/llvm/llvm-project/blob/f28c006a5895fc0e329fe15fead81e37457cb1d1/libcxx/docs/BuildingLibcxx.rst)
and the upstream [`LIBCXX_ABI_DEFINES` option](https://github.com/llvm/llvm-project/blob/f28c006a5895fc0e329fe15fead81e37457cb1d1/libcxx/CMakeLists.txt).
The release matches the version recorded for the previous local prebuilt;
the new recipe makes that dependency reviewable and rebuildable.

## Build

`docker/Dockerfile.burdettie` runs this recipe for `linux/arm64`. Other Docker
architectures keep their system libc++ behavior. The recipe compiles a native
library inside the selected target container, so an ARM64 build on another
host requires Docker's usual cross-platform support.

To build separately on Linux, install Git, CMake, Ninja, Python 3 and a C/C++
toolchain, then run:

```sh
./docker/apple-libcxx/build.sh /tmp/burdettie-libcxx
```

The default compilers are `clang` and `clang++`. `CC`, `CXX`, and `BUILD_JOBS`
can select another supported compiler or parallelism; the default is two
build jobs. The output contains `libc++.so.1.0`, a source record and the two
upstream license notices. The runtime image supplies `libc++abi.so.1` from
Debian. No installation into the development host's system library directory
is performed.

## Validation scope

Before copying the result, the recipe compiles and runs a small program
against the installed headers and libraries. It checks the alternate short
string layout and exercises long-string append, substring, erase and shrink
operations. A failure stops the build.

This smoke test checks the selected library configuration. Imported C++ guest
applications still need their own compatibility tests, and results from the
old prebuilt do not establish results for a newly compiled library.

## Notices

The upstream licenses are retained as [LICENSE-libcxx.txt](LICENSE-libcxx.txt)
and [LICENSE-libcxxabi.txt](LICENSE-libcxxabi.txt). The Docker stage also copies
the corresponding notices from the pinned upstream checkout beside its
generated library.
