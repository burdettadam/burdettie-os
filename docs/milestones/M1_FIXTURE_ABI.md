# M1 Linux/PureDarwin fixture and runtime ABI gate

Status: recipe and source probe implemented; **selected-target cross-build and guest execution are not yet qualified**. The native-host probe tests validate their C logic and evidence protocol. They do not establish PureDarwin ABI compatibility, Burdettie execution, reboot persistence or confinement.

## Pinned build inputs

The recipe uses PureDarwin `0163b2db225b85e3bf96fc25646ea290105ac298` and its actual source definitions. No proprietary SDK is required by this recipe.

| Input | Selection at that revision |
| --- | --- |
| Source lock SHA-256 | `00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0` |
| Root Nixpkgs | `fd1462031fdee08f65fd0b4c6b64e22239a77870`, reached through the root input's `nixpkgs_3` node |
| Source submodule | `external/corecrypto` at `f577aafe4b9a89727968666237d20cc51d2f770d`; recursive state must match the pinned tree |
| Source-derived SDK output | `packages.x86_64-linux.apple-sdk`; `Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk` |
| Host cross compiler | `packages.x86_64-linux.darwin-cross-toolchain`; `x86_64-apple-darwin20.4-clang` wrapper using upstream LLVM 21 |
| Actual Clang target | `x86_64-apple-macosx26.5` |
| Host linker | `packages.x86_64-linux.native-ld`; in-tree cctools ld64, not an arbitrary host linker |
| Target library | `packages.x86_64-linux.libsystem`; source-built `libSystem.B.dylib` and `system/libdyld.dylib` |

The wrapper's historical `darwin20.4` filename does not select the deployment version. `target-info.nix` and the compiler wrapper select 26.5. That label does not prove macOS 26.5 API coverage. The root Nixpkgs input must be followed through the lock graph; the node named simply `nixpkgs` belongs to another dependency. Sources: pinned [flake](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.nix), [lock](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.lock), [target selection](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/lib/target-info.nix), [wrapper](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/toolchain.nix), and [SDK construction](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/apple-sdk-pinned.nix).

## Build without starting a dependency build

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

This is a source assessment at Burdettie `4e804df245b122137d67c45f9d026e261ac68e42`, not a claim that a Rust binary has linked or run on the candidate.

| Area | Concrete source finding | Required evidence or change |
| --- | --- | --- |
| Narrow CLI | `macho-inspect` defaults to no `gui` feature, but it unconditionally depends on `macho-runtime` and `cf-lite`. `cf-lite` has a non-optional macOS `rfd` dependency; the lock includes `objc2-app-kit`, Foundation and CoreFoundation for `rfd`. | Inspect the actual macOS target link and factor desktop-only host services behind explicit features if necessary. `--no-default-features` alone is not a verified AppKit-free runtime build. |
| Platform services | `platform-darwin-host` calls mmap/mprotect, pthread primitives, Mach clocks, `arc4random_buf`, dlopen/dlsym and libunwind dynamic-section registration. | Confirm the selected libSystem/dyld/unwind exports and behavior. Record unresolved symbols; do not replace them with successful stubs. |
| Rust standard library | The repository pins Rust 1.93.0; an ordinary `x86_64-apple-darwin` standard library targets Apple's platform assumptions. | Bind Rust std and compiler/linker provenance to the selected target, inspect generated loads/imports and run a minimal Rust std probe before the larger runtime. No cross-target Rust build has been qualified here. |
| Guest entry | `macho-exec/src/entry.rs` contains an x86_64 entry convention; the interpreted pipeline is ARM64. | Exercise native x86_64 mapping, fixups, constructors, argument passing and return in the guest. Existing ARM64 interpreter success does not establish that route. |
| Exit cleanup | `libsystem-shims/src/lifecycle.rs` registers `atexit` callbacks, but the inspected runtime has no production call to `run_atexit_handlers`; its `exit` shim explicitly bypasses cleanup and calls host `_exit`. | Treat a missing cleanup marker as a runtime compatibility failure. Implement worker-owned cleanup while guest code remains valid; do not run stale callbacks in the supervisor. |
| Runtime framework loading | Foundation/CoreFoundation/CoreGraphics paths are opened in several compatibility components. | Determine which calls occur on the chosen console route and which require a source-built replacement or a properly reported unsupported capability. |
| Display/session | `window-backend-darwin` uses `winit` and `softbuffer`; it does not establish a bare-PureDarwin session. | Qualify an explicit renderer/input integration separately. No GUI success is inferred from the console probe. |

Repository entry points: [CLI dependencies](../../crates/macho-inspect/Cargo.toml), [runtime dependencies](../../crates/macho-runtime/Cargo.toml), [cf-lite dependencies](../../crates/cf-lite/Cargo.toml), [lock](../../Cargo.lock), [Darwin platform](../../crates/platform-darwin-host/src/lib.rs), [unwind registration](../../crates/platform-darwin-host/src/unwind.rs), [native entry](../../crates/macho-exec/src/entry.rs), [exit lifecycle](../../crates/libsystem-shims/src/lifecycle.rs), and [display dependencies](../../crates/window-backend-darwin/Cargo.toml).

## Validation performed for this change

The focused suite has 20 tests. It covers wrong-architecture/version/dependency rejection, malformed commands, lock-input indirection, pinned shallow Git input handling, Nix's required feature version, refusal of missing pinned inputs, retained-output preservation, and native-host C write/read/negative cases. The C probe compiles with `-std=c11 -Wall -Wextra -Werror -pthread` on the Linux development host. Its exact file bytes and cleanup markers pass there.

An actual no-build preflight against the sparse pinned source inspection checkout returns `blocked` for the deliberately uninitialized submodule. Nix and the realized candidate outputs are also absent from that development environment. Offline Nix resolution, selected-target compilation, direct Darwin execution, Burdettie execution and reboot persistence remain unrun. Neither these 20 tests nor this document closes M1.
