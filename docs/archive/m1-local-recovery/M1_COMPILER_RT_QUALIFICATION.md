# M1 compiler-rt qualification — selected x86_64 path

Status: **source-derived qualification plan only**. No selected compiler-rt
archive has been recovered, built, or inspected by this overlay. This document
must not be treated as build or runtime evidence.

## Selected source facts

The selected PureDarwin revision is
`0163b2db225b85e3bf96fc25646ea290105ac298`.

`build.nix` selects `activeCompilerRt` by target architecture and passes its
prefix to CMake as `PUREDARWIN_COMPILER_RT_PREFIX`. The selected
`src/Libraries/libSystem/stub/CMakeLists.txt` then requires
`${PUREDARWIN_COMPILER_RT_PREFIX}/lib/libcompiler_rt.a` to exist and links it
as an ordinary static archive for non-ARM32 targets. ARM32 alone uses
`-force_load`; that special case must not be copied to x86_64.

The same CMake file explains why broad force-loading is unsafe: it can pull
`os_version_check.c.o`, whose `__isPlatformVersionAtLeast` implementation calls
`_availability_version_check`, which the selected libSystem does not
implement. The export list generates explicit `-u` roots; the archive is
intended to contribute only members needed by those roots. ARM64/RISC-V64 add
`___clear_cache` and related exports, while the source explicitly says x86's
coherent instruction cache means compiler-rt does not build `__clear_cache`
for x86. Do not impose ARM-only symbols on x86_64.

The selected dyld CMake file separately notes that ARM32 directly links a
curated set of integer helpers from compiler-rt. That block is conditional on
`PUREDARWIN_ARM32`, so it is not evidence that x86_64 dyld directly links the
archive.

## Qualification sequence

1. Recover the exact compiler-rt archive and its producer receipt from the
   retained private evidence. Independently verify archive bytes, producer
   source revision, target triple, SDK binding, compiler identity, derivation
   identity, store registration and NAR/content identity. A path or filename is
   not provenance.
2. Confirm the archive is a regular non-symlink file and parse its archive
   structure without extracting untrusted paths. Retain member offsets, names,
   sizes, modes and hashes. Reject duplicate/ambiguous members, malformed
   headers, truncation, thin archives, ELF members, bitcode-only members and
   non-x86_64 Mach-O object members for this lane.
3. Inventory definitions and undefined references by exact symbol and member.
   Compare against the selected x86_64 export roots and actual unresolved
   references from the pinned link plan. Do not invent a blanket required
   symbol list from ARM64/ARM32 behavior. Keep Objective-C/runtime late-bound
   symbols distinct from compiler-rt builtins.
4. Explicitly identify any member implementing
   `__isPlatformVersionAtLeast` / `_availability_version_check`. Confirm that
   the selected x86_64 link does not accidentally pull the availability-check
   object into libSystem unless its dependency is deliberately implemented and
   reviewed. Do not suppress an unresolved symbol or force-load the archive to
   make the link pass.
5. Verify the exact linker command uses the ordinary archive input for x86_64,
   with no compiler-rt `-force_load` or global whole-archive behavior. Force-loading unrelated archives remains permitted by the selected recipe. Record which members the
   linker actually selected from a map/trace artifact. If the linker cannot
   provide reliable member-selection evidence, mark that subcheck unsupported.
6. Only after archive provenance and member selection pass, inspect the
   resulting libSystem export table and undefined-symbol set. Bind those
   observations to the exact input archive and linker invocation. This remains
   link evidence, not guest execution or ABI qualification.
7. Preserve raw logs and private paths outside the public repository. Publish
   only reviewed, redacted summaries and content digests approved for release.

## Acceptance gates

- Exact archive bytes and provenance independently verified.
- Every member is parsed within strict count/size bounds and the target
  architecture is confirmed per member.
- x86_64 symbol inventory is evidence-based; ARM-only exports are not required.
- The ordinary-archive link behavior is observed; no broad force-load workaround.
- The availability-check object is not accidentally selected, or its required
  provider is separately designed and qualified.
- The resulting libSystem link receipt binds the exact archive, linker,
  command, selected members and output bytes.

A pass here does not establish libSystem correctness, dyld execution, selected
image boot, runtime ABI behavior, application compatibility or security
confinement. Those remain separate M1/M2/M5 gates.

## Source references

- [Selected `build.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/build.nix)
- [Selected libSystem link configuration](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libSystem/stub/CMakeLists.txt)
- [Selected dyld link configuration](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/dyld/CMakeLists.txt)
