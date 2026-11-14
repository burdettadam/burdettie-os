# M1 Linux/PureDarwin fixture and runtime ABI gate

Status: the pinned source SDK was realized and both source-owned fixtures compiled successfully into x86_64 Mach-O object files on 2026-11-14. **Executable linking and selected-guest/runtime execution remain unqualified.** The [object compilation receipt](evidence/objects-2026-11-14.json) records this limited result. Object compilation and native-host probe tests do not establish PureDarwin ABI behavior, Burdettie execution, reboot persistence or confinement.

## Pinned build inputs

The recipe uses PureDarwin `0163b2db225b85e3bf96fc25646ea290105ac298` and its actual source definitions. No proprietary SDK is required by this recipe.

| Input | Selection at that revision |
| --- | --- |
| Source lock SHA-256 | `00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0` |
| Root Nixpkgs | `fd1462031fdee08f65fd0b4c6b64e22239a77870`, reached through the root input's `nixpkgs_3` node |
| Source submodule | `external/corecrypto` at `f577aafe4b9a89727968666237d20cc51d2f770d`; recursive state must match the pinned tree |
| Source-derived SDK output | `packages.x86_64-linux.apple-sdk`; `Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk` |
| Object-only compiler | Unwrapped Clang 21.1.8, resolved through `.cc` on the SDK's single compiler-bearing `nativeBuildInputs` entry |
| Executable-build compiler wrapper | `packages.x86_64-linux.darwin-cross-toolchain`; `x86_64-apple-darwin20.4-clang` wrapper using upstream LLVM 21 |
| Actual Clang target | `x86_64-apple-macosx26.5` |
| Host linker | `packages.x86_64-linux.native-ld`; in-tree cctools ld64, not an arbitrary host linker |
| Target library | `packages.x86_64-linux.libsystem`; source-built `libSystem.B.dylib` and `system/libdyld.dylib` |

The wrapper's historical `darwin20.4` filename does not select the deployment version. `target-info.nix` and the compiler wrapper select 26.5. That label does not prove macOS 26.5 API coverage. The root Nixpkgs input must be followed through the lock graph; the node named simply `nixpkgs` belongs to another dependency. Sources: pinned [flake](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.nix), [lock](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.lock), [target selection](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/lib/target-info.nix), [wrapper](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/toolchain.nix), and [SDK construction](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/apple-sdk-pinned.nix).

## Object-only compilation with the realized SDK

[`compile_puredarwin_objects.py`](../../scripts/compile_puredarwin_objects.py) provides an earlier compile gate using the SDK and its already-realized native Clang input. It reuses the executable builder's pinned source/submodule/lock checks and Mach-O parser. Its offline Nix evaluation selects only `apple-sdk` and that input's unwrapped `.cc` output, with lock updates and import-from-derivation disabled. Missing inputs return `blocked`; the helper never fetches or builds a dependency.

Run the default preflight first:

```sh
python3 scripts/compile_puredarwin_objects.py \
  --puredarwin-source /path/to/pinned-puredarwin
```

After `ready_to_compile`, explicitly request compilation into a **new** private directory whose parent exists:

```sh
python3 scripts/compile_puredarwin_objects.py \
  --puredarwin-source /path/to/pinned-puredarwin \
  --compile --output-dir /path/to/private-runs/object-compile-01
```

The helper snapshots both C sources, then invokes unwrapped Clang with `--target=x86_64-apple-macosx26.5`, the selected `-isysroot`, C11, strict warnings and `-c`. It records source/compiler/SDK identities and hashes, exact commands and parsed object metadata in `result.json`. It requires thin x86_64 `MH_OBJECT` output and refuses existing output directories without modifying their contents. Optional `--llvm-nm /path/to/existing/llvm-nm` records actual undefined-symbol output; it never acquires that tool, and a supplied tool's failure remains a failure.

### Observed result on 2026-11-14

The SDK-only build had already completed in **75.11 seconds**, with pinned source and header checks recorded in the [source/SDK receipt](evidence/source-preflight-2026-11-14.json). The retained development host then ran the object helper from Burdettie `89130c63edd0226933e8abd2ff556d3d0c4d5fe0`: offline preflight passed in **12 seconds**, and compilation passed in **11 seconds**, both with exit status 0. These phases used the existing unwrapped **Clang 21.1.8** SDK input and performed no downloads or dependency realization. See the [object receipt](evidence/objects-2026-11-14.json) for artifact, source, compiler, header and log identities.

| Captured artifact | Bytes | Parsed architecture/type |
| --- | ---: | --- |
| `simple_cli.o` | 768 | Thin x86_64 `MH_OBJECT` |
| `m1_abi_probe.o` | 5,136 | Thin x86_64 `MH_OBJECT` |

Both objects carry `LC_BUILD_VERSION` for macOS with minimum OS **26.5.0** and SDK field **0.0.0**. The latter is the observed object load-command value; SDK provenance comes from the separately recorded pinned derivation and header hashes. It does not satisfy or change the executable builder's stricter SDK load-command requirement below.

No LLVM nm was supplied. Symbol inspection therefore remains `not_run`, and the tool identity and undefined-symbol lists remain `null`. Empty direct-library dependency lists in relocatable objects do not establish that their imports can be linked or resolved. No executable was linked or launched, and no allocation, file, thread or exit behavior was tested on the selected target. The report keeps linking, selected-guest execution, Burdettie execution and enforcement at `not_run` even with status `objects_compiled_and_inspected`.

## Executable build without starting a dependency build

[`build_puredarwin_fixtures.py`](../../scripts/build_puredarwin_fixtures.py) requires a complete, clean checkout at the selected revision, matching submodules, Nix with already-cached locked evaluation inputs, and the four already-realized outputs above. A depth-one checkout is supported: its exact-revision Git URL explicitly enables `submodules=1&shallow=1`, so Nix accepts the shallow source without requiring history expansion. It resolves the package output/derivation identities with offline Nix evaluation and disables import-from-derivation. It never invokes `nix build`, installs a toolchain, downloads a closure, updates a lock or launches a guest. Evaluation can materialize Nix source/derivation metadata; it is not a guarantee of zero Nix-store writes.

Before acquiring those inputs on a shared host, measure the source and closure footprint against the existing free space and reserve. A small C fixture does not imply that its SDK/libSystem build closure is small. Missing outputs produce `blocked` and exit 2. Capacity planning and any bounded dependency realization remain the platform owner's task; this script deliberately cannot start that work implicitly.

Run the no-build preflight:

```sh
python3 scripts/build_puredarwin_fixtures.py \
  --puredarwin-source /path/to/pinned-puredarwin
```

When it returns `ready_to_build`, compile into a **new** private directory whose parent already exists:

```sh
python3 scripts/build_puredarwin_fixtures.py \
  --puredarwin-source /path/to/pinned-puredarwin \
  --build --output-dir /path/to/private-runs/fixture-build-01
```

The output contains `simple_cli`, `m1_abi_probe`, snapshots of their source files and `result.json`. Keep it outside ordinary source inputs. The report records package output/derivation paths, source and recipe hashes, compiler version, exact compile arguments, binary hashes, direct dependencies and unresolved import names. Compiling the snapshots binds the recorded source hashes even if the development checkout changes during the build. Existing output directories are refused without modifying their contents. Each compiler invocation has a 60-second deadline; retain host CPU/memory limits when using the shared development instance.

The link recipe follows the pinned upstream [small C program recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/xxd.nix), with explicit source-built libSystem and dyld paths. It does **not** pass `-undefined dynamic_lookup`: a missing symbol must fail the link and remain a feasibility finding. The script checks a thin, unencrypted x86_64 `MH_EXECUTE`, one `/usr/lib/dyld` loader command, one `LC_MAIN`, the exact 26.5 minimum/SDK command, no rpath and only `/usr/lib/libSystem.B.dylib` as a direct library dependency. These checks do not prove a transitive dependency closure, loadability, signature validity or runtime behavior.

`built_and_statically_checked` means only compilation and those checks passed. The report always keeps guest execution, Burdettie runtime execution and enforcement at `not_run`; observed execution requires separate records.

### Source and evaluation footprint

Use Nix **2.27 or later** for this unchanged source. Relative `path:` lock inputs require the format introduced in [Nix 2.26](https://nix.dev/manual/nix/2.27/release-notes/rl-2.26.html); the `inputs.self.submodules` declaration was added in [Nix 2.27](https://nix.dev/manual/nix/2.27/release-notes/rl-2.27). This is a syntax floor, not a tested claim that every later Nix release behaves identically.

The complete, non-truncated GitHub tree metadata for the selected [main source revision](https://api.github.com/repos/PureDarwin/PureDarwin/git/trees/0163b2db225b85e3bf96fc25646ea290105ac298?recursive=1) sums to **480,298,521 file bytes**. The selected [corecrypto submodule](https://api.github.com/repos/Vali0004/corecrypto/git/trees/f577aafe4b9a89727968666237d20cc51d2f770d?recursive=1) adds **3,617,400 file bytes** and has no nested submodules. These are source file sizes, not measured filesystem allocation or build peak usage. Reserve extra space for Git objects, store copies, Nixpkgs/other locked inputs, toolchain binaries and temporary build trees.

The narrow `libsystem` derivation disables the kernel and general userspace build. Its explicit targets include libSystem, dyld, static dispatch/XPC pieces and notifyd/logd. The source filter includes all of `src/Libraries` (271,914,674 file bytes in that revision), selected XNU headers, cmake files and relevant tools. Its SDK and native linker add their own source file sets. This is materially narrower than the full desktop image; the upstream full-image 60-GB guideline does not prove that the fixture lane exceeds the host budget. Conversely, the roughly 0.5-GB source total does not prove that the dependency closure or build will fit.

A sparse checkout containing only `nix/` is adequate for source inspection, but is not sufficient to evaluate/build those file sets. Prefer one complete shallow pinned checkout and the exact small corecrypto submodule within the measured budget. Then evaluate/dry-run only `apple-sdk`, `native-ld`, `darwin-cross-toolchain` and `libsystem`, one stage at a time. Record the planned downloads, realized closure sizes and local build requirements before starting a bounded build. Avoid `nix flake show` over the entire package set and avoid a full-image build as the first sizing experiment. No sparse subset is qualified as equivalent by this change.

## Probe behavior and evidence

[`fixtures/simple_cli/main.c`](../../fixtures/simple_cli/main.c) remains the first execution probe: exact `hello` output and exit status 0. [`fixtures/m1_abi_probe/probe.c`](../../fixtures/m1_abi_probe/probe.c) then exercises:

- `malloc`, `realloc`, memory preservation and `free`.
- `pthread_create`/`pthread_join`, an actual worker computation, returned pointer and result comparison.
- File creation with `O_EXCL`, complete writes, `fsync`, close, reopening, exact UTF-8 bytes and end-of-file validation.
- Normal process termination and an `atexit` callback, with a distinct marker that the supervisor must observe together with the actual exit status.

Use a fresh run-specific nonce containing at most 64 ASCII letters, digits, underscores or hyphens. `write` refuses to replace a retained file. `read` never writes it; wrong nonces, missing files and trailing bytes fail. For example, in the selected guest:

```sh
./m1_abi_probe write /var/tmp/burdettie-m1/fresh-probe.txt run-specific-nonce
printf 'observed-exit=%s\n' "$?"
```

The required output is a line containing that nonce, `mode=write allocation=pass file=pass thread=pass`, a separate `exit_cleanup=pass` line, and the observed zero exit status. Kernel output, the echo of this command and a success string embedded in source are insufficient. Retain the exact binary hash and guest/image identity with the transcript.

After completely stopping and starting QEMU against the same writable disk, run:

```sh
./m1_abi_probe read /var/tmp/burdettie-m1/fresh-probe.txt run-specific-nonce
printf 'observed-exit=%s\n' "$?"
```

That second execution must produce the corresponding `mode=read` and cleanup markers with zero status. This distinguishes direct guest process execution and persistence from build/inspection evidence. Run the same fixture through Burdettie only after its CLI port can start in that guest, and retain a separate route/result. Calling the fixture's `main` in-process does not by itself prove that guest exit callbacks or process cleanup are correctly isolated from the supervisor.

## Runtime ABI assessment and remaining port work

The baseline source assessment was made at Burdettie `4e804df245b122137d67c45f9d026e261ac68e42`. [PR #7](https://github.com/burdettadam/burdettie-os/pull/7), at `f6918f029c65144eea3ca7e3b437a58bdefebb4e`, now contains separately reviewed runtime prerequisites. It is not merged into this platform branch. Neither the assessment nor those host tests claim that a Rust binary has linked or run on the selected candidate.

| Area | Concrete source finding | Required evidence or change |
| --- | --- | --- |
| Narrow CLI | The baseline pulled `rfd` and AppKit through `cf-lite`. PR #7 makes native dialogs optional through the complete CLI/runtime dependency path while preserving defaults. Three locked Darwin dependency-graph checks and the new no-dialog tests pass on macOS CI. | Review/integrate the feature gate, then inspect the actual selected-target link. The AppKit-free graph is not a verified PureDarwin binary or runtime. |
| Platform services | `platform-darwin-host` calls mmap/mprotect, pthread primitives, Mach clocks, `arc4random_buf`, dlopen/dlsym and libunwind dynamic-section registration. | Confirm the selected libSystem/dyld/unwind exports and behavior. Record unresolved symbols; do not replace them with successful stubs. |
| Rust standard library | The repository pins Rust 1.93.0; an ordinary `x86_64-apple-darwin` standard library targets Apple's platform assumptions. | Bind Rust std and compiler/linker provenance to the selected target, inspect generated loads/imports and run a minimal Rust std probe before the larger runtime. No cross-target Rust build has been qualified here. |
| Guest entry | `macho-exec/src/entry.rs` contains an x86_64 entry convention; the interpreted pipeline is ARM64. | Exercise native x86_64 mapping, fixups, constructors, argument passing and return in the guest. Existing ARM64 interpreter success does not establish that route. |
| Exit cleanup | Normal `atexit`/C++ cleanup remains unimplemented. PR #7 separates immediate exit without flushing and intercepts actual interpreter exit shim addresses before host FFI. Native exit still terminates the embedding process. | Treat a missing cleanup marker as a runtime compatibility failure. Implement launch-owned callbacks with architecture/DSO/image lifetime and drain before unmapping; interpreted callbacks must execute through the interpreter. Do not drain stale process-global callbacks in the supervisor. |
| Runtime framework loading | Foundation/CoreFoundation/CoreGraphics paths are opened in several compatibility components. | Determine which calls occur on the chosen console route and which require a source-built replacement or a properly reported unsupported capability. |
| Display/session | `window-backend-darwin` uses `winit` and `softbuffer`; it does not establish a bare-PureDarwin session. | Qualify an explicit renderer/input integration separately. No GUI success is inferred from the console probe. |

Repository entry points: [CLI dependencies](../../crates/macho-inspect/Cargo.toml), [runtime dependencies](../../crates/macho-runtime/Cargo.toml), [cf-lite dependencies](../../crates/cf-lite/Cargo.toml), [lock](../../Cargo.lock), [Darwin platform](../../crates/platform-darwin-host/src/lib.rs), [unwind registration](../../crates/platform-darwin-host/src/unwind.rs), [native entry](../../crates/macho-exec/src/entry.rs), [exit lifecycle](../../crates/libsystem-shims/src/lifecycle.rs), and [display dependencies](../../crates/window-backend-darwin/Cargo.toml).

PR #7's [headless CLI contract](https://github.com/burdettadam/burdettie-os/blob/f6918f029c65144eea3ca7e3b437a58bdefebb4e/docs/HEADLESS_CLI.md) and [exit lifecycle requirements](https://github.com/burdettadam/burdettie-os/blob/f6918f029c65144eea3ca7e3b437a58bdefebb4e/docs/RUNTIME_EXIT_LIFECYCLE.md) describe the changes and remaining ownership limits. Its macOS build/tests, FFI, quickstart, Docker and Linux ARM64 interpreter jobs pass; dependency-policy scanning still fails on the existing unmaintained `ttf-parser` advisory. No check is suppressed and no guest qualification is inferred.

## Validation performed for this change

The executable fixture suite has **20 passing tests**. It covers wrong-architecture/version/dependency rejection, malformed commands, lock-input indirection, pinned shallow Git input handling, Nix's required feature version, refusal of missing pinned inputs, retained-output preservation, and native-host C write/read/negative cases. The C probe compiles with `-std=c11 -Wall -Wextra -Werror -pthread` on the Linux development host. Its exact file bytes and cleanup markers pass there.

The object helper adds **9 passing focused tests** for its offline input gate, object architecture/type checks, explicit compilation boundary, source snapshots, retained-output preservation, and compiler/optional-nm failures. The complete current script suite passes **63 tests**. Those workflow tests use synthetic Mach-O inputs; the separately captured objects above provide the actual selected-SDK compilation evidence. Their captured bytes, hashes, parsed metadata and source snapshots were checked against the retained report.

The original no-build preflight against the sparse inspection checkout returned `blocked` for its deliberately uninitialized submodule. The retained AWS host now has the complete pinned checkout, exact corecrypto submodule and Nix 2.28.3. A no-build evaluation of the four inputs passed without a lock update or import-from-derivation; see the [AWS findings](M1_AWS_DEVELOPMENT.md) and [source receipt](evidence/source-preflight-2026-11-14.json).

The per-output plans below overlap; do not sum their download or unpacked sizes. They are pre-build estimates, not measured peak storage.

| Requested output | Local derivations | Fetch paths | Download MiB | Unpacked MiB |
| --- | ---: | ---: | ---: | ---: |
| `apple-sdk` | 3 | 72 | 412.25 | 1908.90 |
| `native-ld` | 23 | 262 | 768.52 | 2993.90 |
| `darwin-cross-toolchain` | 27 | 262 | 768.52 | 2993.90 |
| `libsystem` / all four | 32 | 264 | 916.73 | 4811.47 |

Two concrete constraints remain beyond those estimates. The pinned compiler-rt recipe unpacks the LLVM monorepo while building its builtins subset, and the generic PureDarwin build uses Debug configuration. Also, raw Ninja invocations in `native-ld.nix`, `compiler-rt.nix` and `build.nix` bypass Nixpkgs' default Ninja build-phase concurrency argument: `--cores 1` alone does not limit their compiler process count. Keep larger realization blocked until those peak-space and job-concurrency bounds are addressed explicitly. The SDK's small MIG compiler uses the source's `pcc.sh` helper, which does honor `NIX_BUILD_CORES`.

The four outputs are link inputs, not a complete guest process closure. The selected libSystem source intentionally permits unresolved Objective-C runtime references and uses a separately loaded matching libobjc; its real loader requirements must be checked in the selected image. Header/export presence does not prove implementation behavior. Sources: [compiler-rt](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/compiler-rt.nix), [native linker](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/native-ld.nix), [generic build](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/build.nix), [MIG compiler](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/migcom.nix), [libSystem linkage/startup contract](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libSystem/stub/CMakeLists.txt), and [libobjc dependencies](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/apple/libobjc.nix).

Selected-SDK object compilation has passed. Executable linking, import resolution, direct execution in the selected guest, execution through Burdettie and security enforcement remain open gates. The successful older-image shell/file experiment is separate bootstrap evidence. M1 remains in progress.
