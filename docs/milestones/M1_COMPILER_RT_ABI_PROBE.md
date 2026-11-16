# M1 compiler-runtime arithmetic and calling-convention fixture

This change adds executable test code, rather than another assertion that an
archive exists. It prepares the compiler-runtime part of the M1 runtime-ABI
lane. **It does not close the selected PureDarwin build or guest-execution gate.**

## What runs

`fixtures/m1_compiler_rt_probe/probe.c` calls these six helpers through volatile
function pointers, preserving actual external calls under optimization:

| C symbol | Cases | Behavior exercised |
| --- | ---: | --- |
| `__divti3` | 2,197 | Signed 128-bit quotient, truncated toward zero |
| `__modti3` | 2,197 | Signed 128-bit remainder, including negative dividends |
| `__udivti3` | 1,058 | Unsigned 128-bit quotient |
| `__umodti3` | 1,058 | Unsigned 128-bit remainder |
| `__fixdfti` | 204 | Finite, in-range binary64 to signed 128-bit integer |
| `__floattidf` | 294 | Signed 128-bit integer to binary64, ties-to-even |

There are **7,008 comparisons**, not 7,008 separately discovered unit tests.
Cases include cross-products of integer boundaries, high/low 64-bit word
transitions, deterministic SHA-256-derived inputs, fractional and subnormal
binary64 values, both signed zeros, and rounding ties. Division by zero,
`INT128_MIN / -1`, NaN/infinity conversions, and out-of-range floating-to-integer
conversions are excluded because this fixture does not assign results to them.

`scripts/m1_compiler_rt_vectors.py` computes the expected values with Python
arbitrary-precision integer arithmetic and binary64 conversion, independently of
the tested C helpers. The generated header and complete logical corpus are
retained with each operation, rather than checked in as large generated files.
The freestanding core needs no SDK headers or libc. `main.c` is the separate
stdio reporter and refuses a non-nearest floating-point rounding mode.

A compiler emitting these six calls is not by itself proof of a working target
ABI. The **host** execution checks their behavior and parameter/return handling
on Linux x86_64 only. It does not exercise the Darwin calling environment,
selected libSystem initialization, dyld, guest kernel, or Burdettie loader.

## Default: compile the Darwin object, never execute it

```sh
python3 scripts/probe_m1_compiler_rt_abi.py \
  --clang /absolute/path/to/installed/clang \
  --output-dir /existing/private/parent/new-arithmetic-check
```

The object is built for `x86_64-apple-macosx26.5`, with `-ffreestanding`,
`-nostdinc`, `-fno-builtin`, `-fno-stack-protector`, and `-fno-lto`. Its inspector
requires baseline x86_64, one `_m1_compiler_rt_probe` definition, and exactly the
six Mach-O-prefixed imports. Unexpected libc/helper dependencies fail the check.
It retains macOS minimum version **26.5.0** and SDK field **0.0.0**: no SDK was
used. Do not relabel that field as selected-SDK provenance or strip it to placate
a linker. The success status is `darwin_object_checked_host_not_run`.

The M1 workflow adds this command to its existing job, using the already selected
LLVM 18 tools. It keeps its runner, permissions, time limits and existing artifact
retention. No new job, tool installation or cloud resource is introduced.

## Native macOS fixture integration

The existing `scripts/build_fixtures.sh` reference build needs both C sources and
generated vectors; its ordinary one-source discovery is insufficient. An explicit
recipe copies the sources into a fresh temporary directory, generates `vectors.h`
beside them, and builds ARM64 and x86_64 slices with the existing Xcode clang/SDK.
It creates the normal `app` output and verifies both slices with `lipo`.

The universal output remains in ordinary runtime compatibility discovery. Its
ARM64 slice is required by that gate's native ARM64 entry path; a thin x86_64
`app` must not enter that path. The reporter labels ARM64 execution as `arm64`.
The explicit Linux x86_64 producer still rejects foreign architecture reports.
Generation, compilation, linking and slice verification failures stop the build.
Fresh source copies prevent an old untracked source-side `vectors.h` from taking
precedence over newly generated inputs. Only successful temporary work is cleaned.

Linux routing regressions drive the real shell builder through controlled
macOS-discovery and compiler adapters, compile the arithmetic sources with a real
host compiler, execute 7,008 comparisons, and retain generator/linker failure
controls. They do not prove Mach-O slices or macOS SDK/runtime behavior; the
unchanged native CI jobs must establish those results. This SDK-backed macOS
reference is separate from the freestanding SDK-0.0.0 object check and does not
qualify selected PureDarwin compiler-rt/libSystem, guest execution or M1 completion.

## Explicit Linux host execution with an identified builtins archive

On an existing trusted Linux x86_64 test host, select the installed host
compiler-rt archive and its observed hash, then use:

```sh
python3 scripts/probe_m1_compiler_rt_abi.py \
  --clang "$INSTALLED_CLANG" \
  --run-host \
  --host-builtins "$HOST_BUILTINS_ARCHIVE" \
  --host-builtins-sha256 "$HOST_BUILTINS_SHA256" \
  --host-linker "$INSTALLED_GNU_LD" \
  --output-dir "$NEW_PRIVATE_EVIDENCE_DIRECTORY"
```

The runner snapshots the archive and fixture inputs. The native link uses
`-nodefaultlibs` plus the explicit archive, libc and libm, not an implicit libgcc
fallback. GNU ld's `--trace-symbol` records must locate exactly one definition of
each tested helper in the archived snapshot. A native linker map is retained.
The newly compiled output must be an x86_64 ELF executable before it is run.
The report must contain the exact six categories and all expected counts, zero
mismatches and a real zero process exit. A stale passing JSON body cannot rescue
a nonzero exit, signal, timeout, wrong platform or incomplete run.

`darwin_object_and_linux_arithmetic_checked` means both these preparatory checks
passed. All selected-source, selected-compiler-rt, selected-libSystem,
selected-runtime-ABI, Darwin execution, boot and security fields remain false or
`not_run`. The installed host compiler-rt version is separate from the selected
PureDarwin dependency; no equivalence is inferred from matching helper names.
The unit tests also run the fixture with the host C compiler's normal runtime as
a fixture-behavior test; that test is not the explicit compiler-rt archive lane.

## Failure and evidence

The runner uses the existing bounded child observer: no shell, ambient compiler
or loader overrides, automatic retry or fallback tool. It fingerprints tools,
records command arguments and exit status, retains bounded stdout/stderr, and
rechecks source/helper/archive snapshots across phases. Existing directories
are refused. Terminal publication failure is an operation failure. CLI SIGTERM
enters failure handling and cleanup of the operation-owned process group.

These checks assume trusted installed tools and an already trusted checkout.
They do not authenticate a hostile producer, contain detached hostile children,
or hash every host startup object/header/shared library. This is **not** a
qualified shared-AWS-host launcher. Do not run it against shared workloads as a
substitute for the existing resource/service preflight. Do not publish private
host paths or raw execution records in the public repository.

Negative regressions replace the signed-division implementation with an incorrect
helper and separately supply a non-nearest rounding-mode result. The first must
produce arithmetic failures and exit 1; the second must exit 2 without a passing
arithmetic report. Neither failure is promoted to a successful runtime result.

## Next selected-target integration

After the retained cross-toolchain/build evidence and current host safeguards
are qualified, regenerate the same vectors, recompile **both** core and reporter
with the verified selected compiler/SDK, and link against the selected libSystem
using observed command/input capture. Do not change the frozen fixture builder
or its recorded hashes implicitly. Bind the new fixture executable, selected
libraries and guest run to their exact build receipts, then run the reporter in
the selected Darwin guest with the existing guest-execution/persistence protocol.
Only that evidence can advance the selected runtime-ABI gate.

The previously observed local Swift LLD rejection has a specific source cause:
the installed tool's advertised revision
[`10999b6`](https://github.com/swiftlang/llvm-project/blob/10999b6d034fe318f3d56c83bddb6572593a8bb0/lld/MachO/InputFiles.cpp#L146-L154)
contains an unconditional rejection after detecting platform metadata. This
change does not patch that binary, strip metadata, lower the target, or replace
the selected native linker. It records the failure separately and executes only
the newly built Linux host fixture.
