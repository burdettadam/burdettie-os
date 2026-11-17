# M1 libtapi source staging

This opt-in recipe reduces the writable source copy for the pinned native
linker's `apple-libtapi` dependency. It preserves the complete immutable source
input and changes only the derivation's unpack phase. The existing
[`plan_puredarwin_linker.py`](../../scripts/plan_puredarwin_linker.py) and
[`plan_puredarwin_libraries.py`](../../scripts/plan_puredarwin_libraries.py)
retain their original behavior and identities.

**This is a new derivation, not a continuation of an interrupted build.** Nix's
`--keep-failed` retains diagnostic files; it does not automatically reuse an
unfinished derivation's object files. The pinned libtapi
[`build.sh`](https://github.com/tpoechtrager/apple-libtapi/blob/640b4623929c923c0468143ff2a363a48665fa54/build.sh)
also removes its local `build` directory before configuring. Do not run it in a
retained failed tree. This helper never reads a prior build as an incremental
cache and never modifies or deletes one.

## Source and derivation contract

The recipe selects exactly one `apple-libtapi` from the pinned PureDarwin native
linker's build inputs. It requires version `1300.6.5`, upstream revision
`640b4623929c923c0468143ff2a363a48665fa54`, and the unchanged fixed-output hash in
[`libtapi.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/libtapi.nix).
It keeps the existing Python 3 dependency and adds no build dependency.

The override asserts that the source, patches, patch hook, build phase, native
and target build inputs, configure and install phases, install/CMake controls,
and unpack hooks stay equal. The original build still selects Release,
`tapi;clang`, its existing target list and install targets, and obtains its job
count from `NIX_BUILD_CORES`. No target, optimization, architecture or compiler
version is changed. The SDK derivation and output must still match the already
realized baseline.

The source stager creates a new directory with the complete input's directory
structure. Every directory is real and writable by its owner. Unchanged regular
files become absolute symlinks to the retained immutable input. Original
symlinks keep their original link text; in this revision,
`build_tapi_tools.sh -> build.sh` must resolve to the new staged wrapper.
Hidden files and empty directories remain represented.

The following files are private copies, with separate inodes:

| Input | Reason it must be a regular writable file |
| --- | --- |
| `build.sh`, `install.sh`, and regular files under `tools/` | The original `postPatch` hook discovers executable regular files and rewrites their interpreter lines. |
| `src/llvm/include/llvm/TextAPI/MachO/Architecture.def` | The original [`libtapi-riscv.patch`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/libtapi-riscv.patch) modifies this file. |
| `src/llvm/cmake/config.guess` | Nixpkgs' [`updateAutotoolsGnuConfigScriptsPhase`](https://github.com/NixOS/nixpkgs/blob/fd1462031fdee08f65fd0b4c6b64e22239a77870/pkgs/build-support/setup-hooks/update-autotools-gnu-config-scripts.sh) uses `find -type f` before replacement. A file symlink would silently skip the original update. |
| Regular files under `src/tapi/include/` | The pinned [`install(FILES)` declarations](https://github.com/tpoechtrager/apple-libtapi/blob/640b4623929c923c0468143ff2a363a48665fa54/src/tapi/include/CMakeLists.txt) must install ordinary public headers rather than links back to the source input. |

The inventory rejects unexpected `config.guess` or `config.sub` paths instead of
silently changing which files the automatic hook sees. Existing output paths,
outputs inside the immutable source, and unsupported source entry types are
also rejected. There is no recursive chmod through source links and no hardlink
to a file that a later hook can modify.

Using real directories matters because LLVM's
[`TableGen.cmake`](https://github.com/tpoechtrager/apple-libtapi/blob/640b4623929c923c0468143ff2a363a48665fa54/src/llvm/cmake/modules/TableGen.cmake)
uses recursive file discovery with the Makefiles generator. Replacing whole
source directories with symlinks could change that discovery. The stager only
links files and preserves the directory structure.

## Plan without building

The complete libtapi source and selected SDK must already be present. Use a new
private plan directory with an existing parent:

```sh
python3 scripts/plan_puredarwin_libtapi.py \
  --puredarwin-source "$PUREDARWIN_SOURCE" \
  --expected-sdk-output "$SDK_OUTPUT" \
  --expected-sdk-derivation "$SDK_DERIVATION" \
  --output-dir "$NEW_PLAN_DIRECTORY" \
  --query-substituters
```

The helper verifies the pinned clean PureDarwin checkout and lock file,
evaluates offline without lock changes or import-from-derivation, snapshots the
exact recipe and stager, and hashes the complete realized libtapi source. It
then runs only a Nix dry-run. `--query-substituters` allows binary-cache metadata
queries for that dry-run; it does not download dependencies or realize outputs.
Without that option the dry-run is an offline graph, which is not a binary-cache
capacity estimate. Remote builders and automatic free-space garbage collection
are disabled by per-command options.

The private result includes original and candidate libtapi derivation/output
identities, the unchanged SDK tuple, full source identity, recipe and stager
hashes, upstream patch-content and build/patch-phase hashes, a complete source
manifest, and the dry-run's return code and log hash.
It reports `planned_not_built` only after a successful dry-run. It also records
separate realization arguments with one job, one core, two substitution jobs,
`--keep-failed`, build logs, and a new `result-libtapi` output root. The helper
does not execute those arguments.

The plan's source manifest records every directory, regular file and original
symlink, source modes and file hashes, and whether each regular file will be
copied or linked. Its source-inventory digest is independent of the new staging
directory. The actual unpack phase rehashes the immutable source after staging
and fails if that inventory changed. It writes a complete manifest in the new
build directory and emits a compact `BURDETTIE_LIBTAPI_STAGING` receipt to the
build log, including the manifest digest and measured staging allocation. The
compact receipt remains in retained build logs after normal successful Nix
cleanup removes temporary build files. This is a receipt for prepatch staging;
the original patch and setup hooks still run afterward.

## Observed metadata plan

The actual retained-host plan at Burdettie
`f8c59175784edb89e0087734d5020d142256297d` passed on 2026-11-14 in
**24.08 seconds**, with status **`planned_not_built`**. Offline evaluation and
the permitted cache-metadata dry-run listed only the new libtapi derivation,
with no dependency fetch paths, downloads or realization. The retained SDK and
all 12 unchanged-recipe checks matched. The [sanitized plan
receipt](evidence/libtapi-plan-2026-11-14.json) binds the source, patch, recipe,
stager, original/candidate identities and complete captured result/log hashes.

The complete source inventory records 10,666 regular files, 1,515 directories
including the root, and one original symlink. It proposes 74 private file copies
and 10,592 immutable-file links. Measured original source allocation is
**217,415,680 bytes**; estimated staging allocation is **50,094,080 bytes**.
Staging itself has not run in this result. The estimate excludes configuration,
generated files, objects, linking, outputs and additional filesystem metadata;
it does not qualify build peak usage. The full 3,111,190-byte source manifest
remains on the host with its recorded hash and was not locally captured.

At the metadata phase's finish, 9,525,194,752 disk bytes remained free, leaving
935,260,160 bytes above the unchanged 8-GiB reserve. This is a planning
measurement. Candidate realization, output inspection, native-linker rebinding
and downstream execution remain separate gates.

This implementation head passed [100 script tests, 37 contract tests and four
examples](https://github.com/burdettadam/burdettie-os/actions/runs/43627478562),
with no skips. Its [six functional CI
jobs](https://github.com/burdettadam/burdettie-os/actions/runs/43627478519) passed;
the required `cargo-deny` advisory check and [repository history
scan](https://github.com/burdettadam/burdettie-os/actions/runs/43627478517) remain
failed. Those results are bound to `f8c5917`; a later evidence update must pass
its own checks. M1 remains incomplete.

## Observed realization

A separate bounded build at the same `f8c5917` source completed on 2026-11-14
at **20:08:20 UTC**. Its realization phase passed in **4,082.6 seconds**, and
its installed-output inspection passed. The [new build
receipt](evidence/libtapi-build-2026-11-14.json) binds the complete recovered
result, build log, NAR sidecar and journal. It leaves the earlier metadata and
failed-build receipts unchanged.

Actual prepatch staging allocated **50,089,984 bytes**, copying 74 regular files
and linking 10,592 to the complete source. The original patch, shebang rewrites
and config.guess hook then ran. The output contains eight ordinary headers and
the inspected 4,910,128-byte x86_64 ELF library, with no source-output symlinks
or complete-source/temporary-build references. Registered NAR size is
4,940,088 bytes. The prepatch source check is distinct from the later native
metadata plan's fresh complete-source rehash.

The build stayed above the 8-GiB reserve: its observed minimum free space was
8,995,893,248 bytes, its final free space was 9,317,019,648 bytes, and maximum
observed whole-host growth was 526,663,680 bytes. Cgroup memory peaked at
1,096,855,552 bytes without high/max/OOM events. This is a measured success for
this bounded library build; it does not qualify subsequent build peaks.

## Capacity and qualification gates

The allocation estimate covers the directory structure, file symlinks and
private copies. It rounds file symlinks to at least one filesystem block,
includes source directory allocations and rounded copied-file bytes, and labels
the calculation as an estimate. It excludes CMake configuration, generated
files, objects, linking, outputs, other dependency activity and additional
filesystem metadata. `peak_space_qualified` remains false.

An actual build requires separate review of the current host census, complete
cache plan, source allocation and a finite monitored budget. Keep the existing
CPU, memory, swap, task, job and disk-reserve limits, and preserve all earlier
sources, build trees, logs, stores and VM artifacts. A longer finite wall-time
budget may address a time-only cutoff, but a fresh derivation starts its build
work again. An exploratory growth cap is a stop condition, not proof of fit.

After successful realization, inspect the actual output, its derivation and NAR
identity, installed header file types and bytes, library format and dependencies,
and store references. A source path can remain embedded in a binary even when
there are no output symlinks. Confirm the effective compiler include paths and
the applied architecture patch in the recorded build commands. Do not manually
register copied files under a predicted store path.

The existing default helpers still select their original dependencies. The
separate [staged-linker recipe](M1_STAGED_LIBTAPI_LINKER.md) now provides an
explicit rebind and has an actual **32.21-second metadata-plan pass**, with nine
local derivations and no fetched paths. The [downstream composition and shared
inspector](M1_STAGED_LIBRARIES.md) require a separately successful native build
before their planning path can run. LibTAPI success alone does not qualify
native linking, executable fixtures, the selected Darwin guest, Burdettie
runtime ABI or any security backend. Those gates remain open.

## Focused validation

```sh
NIX_BIN=/path/to/nix CMAKE_BIN=/path/to/cmake \
  python3 -m unittest discover -s scripts/tests -p '*libtapi*.py' -v
```

The tests evaluate the recipe with a real Nix binary and synthetic package
records, then execute the resulting unpack phase against a small source
fixture. They verify complete inventory, distinct writable copies, unchanged
input bytes, the original relative link, both unpack hooks and closed failure
paths. A real patch and C preprocessor check exercise the architecture include;
the hook regression uses the original `find`/`cp` behavior. A real CMake
configuration checks recursive discovery and regular header installation, with
a negative control that demonstrates the symlink-install problem. These are
fixture tests, not compilation of the actual bundled LLVM/Clang/TAPI tree.

The existing M1 workflow requires Nix, CMake, the C preprocessor and `patch` for
these checks. Missing tools fail the job instead of silently skipping them.
