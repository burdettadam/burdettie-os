# Installed cross-toolchain inspection

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

[`inspect_puredarwin_cross_toolchain.py`](../../scripts/inspect_puredarwin_cross_toolchain.py)
adds a callable inspection step for an **already realized**, exactly selected
cross-toolchain. It does not plan or build packages, fetch dependencies, execute
wrappers, compile fixtures, or qualify a guest. Its local tests use small owned
filesystem trees and synthetic Nix command results. They are not an actual
cross-toolchain realization or output inspection.

The [bounded cross-toolchain producer](M1_CROSS_TOOLCHAIN_BUILD.md) now calls
this inspector after revalidating a retained plan and observing its exact
offline realization. Its source tests do not establish an actual build;
recovering the private metadata archive and checking the current host remain
execution prerequisites. [The staged downstream planner](M1_STAGED_LIBRARIES.md) remains
unchanged, including all thirteen helper/recipe inputs. M1 remains incomplete;
all seventeen security backend families remain unavailable and unqualified.

## Why a separate inspector is needed

The selected PureDarwin
[`toolchain.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/toolchain.nix)
constructs fourteen `writeShellScriptBin` packages and joins them with
`symlinkJoin`. Its installed tool entries legitimately point outside the joined
output into those individual wrapper packages. The native-linker inspector's
requirement that every link remain inside one package therefore does not apply
to this output.

The pinned Nixpkgs
[`writeTextFile`, `writeShellScriptBin` and `symlinkJoin` implementations](https://github.com/NixOS/nixpkgs/blob/fd1462031fdee08f65fd0b4c6b64e22239a77870/pkgs/build-support/trivial-builders/default.nix)
provide a concrete inspection contract. The joined derivation declares its
ordered `paths`; each wrapper derivation declares an executable destination and
the complete `text` written there. The inspector reads those exact derivations
from the local store. A familiar filename, a shebang, or a plausible shell
fragment is not sufficient evidence.

The expected ordered names, used as a completeness check, are:

| Group | Installed names |
| --- | --- |
| Compilers | `x86_64-apple-darwin20.4-clang`, `x86_64-apple-darwin20.4-clang++` |
| Binary tools | The same prefix followed by `ar`, `ranlib`, `strip`, `nm`, `objdump`, `otool`, `dsymutil`, `install_name_tool`, `lipo`, `ld` |
| Unprefixed tools | `xcrun`, `dsymutil` |

The selected cross recipe's preservation assertions and exact candidate tuple
remain the authority for choosing the derivation. This inspector does not infer
that source provenance from a directory's name.

## Caller contract

Before calling this function, the producer must validate the selected
source/lock, a successful cross-stage metadata plan, the complete successful
native-linker/TAPI/SDK receipt chain, and the exact reviewed helper hashes. It
must then observe successful realization of that plan's candidate inside the
owner's existing resource limits. An existing directory or this inspector's
return value alone does not replace the successful build receipt.

The current planner exposes the freshly checked native inspection as
`current_native_inspection`. After the preceding gates and actual realization:

```python
from inspect_puredarwin_cross_toolchain import inspect_cross_toolchain_output

native_inspection = cross_plan["current_native_inspection"]
cross_inspection = inspect_cross_toolchain_output(
    candidate=cross_plan["candidate"],
    dependencies={
        "native_linker": native_inspection["registered_output"],
        "sdk": native_inspection["dependencies"]["sdk"],
    },
    directory=new_inspection_directory,
    report=build_report,
)
```

The evidence directory must not already exist; its real parent must exist
outside `/nix/store`. The inspector creates it exclusively. All Nix commands
are constructed by the helper, use the direct local store and `--offline`, and
name literal validated store paths. There is no caller-supplied command,
realization, substituter query, repair, signature waiver, or whole-store scan.
The caller must select the reviewed Nix executable and preserve its own bounded
CPU, memory, disk, inode and overall-time guard. Individual metadata commands
have a 30-second timeout; each store-verification command has a 90-second timeout.

Any exception is a failed live inspection, including missing output, failed or
timed-out verification, unsupported metadata, changed files and evidence-write
errors. `report["cross_toolchain_output_inspection_status"]` remains `"fail"`
until every check and the final exclusive evidence write have completed. The
caller must retain and persist that report on failure. Partial files are never
deleted or reused. A later attempt needs a new evidence directory.

## Checks and retained proof

1. **Candidate registration.** A single-output `path-info` result must bind the
   exact candidate output to the planned deriver, a canonical SHA256 NAR hash
   and a positive integer NAR size. The raw bytes are retained as
   `output-path-info.json` and bound by checksum.
2. **Declared producers.** Offline `derivation show` records must identify the
   exact candidate and direct inputs. Each of the fourteen ordered join paths
   must have one declared producer and one `out`, the expected `/bin/` destination,
   executable status and bounded UTF-8 `text`. Unknown structured environments,
   dynamic outputs, missing fields or ambiguous outputs fail inspection.
3. **Native-linker binding.** The clang, clang++ and ld wrapper derivations must
   select `out` from the successful native-linker derivation, and their registered
   references must contain that exact native output. Other wrappers must not
   acquire this dependency. These are declared dependency and byte checks;
   they do not assert that a particular invocation actually executes the linker.
4. **Registered closure.** Recursive local `path-info` must contain exactly the
   closure reachable from the cross output and the independently selected
   native-linker and SDK outputs. References must be unique and present. Every
   member's NAR identity is retained. Copied source paths may lack a deriver;
   this is recorded as null, not fabricated producer provenance. A wrapper's
   direct references must be selected outputs of its declared derivation inputs,
   or explicitly declared source paths. An available store path alone is not
   sufficient.
5. **Installed bytes and layout.** The join must contain only a real `bin`
   directory and the fourteen expected symlinks. Each link must name the exact
   declared wrapper file. Each wrapper output must contain only its real `bin`
   directory and that one regular executable. The installed bytes must equal
   the full producing derivation's `text`. Extra files, alternate links,
   dangling/cyclic indirection, nonregular files, special mode bits and changed
   bytes fail. File sizes, hashes, modes, directory modes and link text are
   retained.
6. **Default integrity and trust checks.** Offline `store verify --derivation`
   verifies the exact derivation records read by the inspector; a separate
   recursive verification checks the registered output closure. No content or
   trust checks are disabled. File inventories and closure registration are
   checked again after verification. Nix documents these default checks and
   the distinction between derivations and outputs in its
   [2.28-series verification reference](https://nix.dev/manual/nix/2.28/command-ref/new-cli/nix3-store-verify).

Metadata JSON is limited to 16 MiB per parsed response, with duplicate fields
rejected; each script is at most 64 KiB. The helper accepts at most 128
derivations per query and 2,048 registered closure entries. These are rejection
bounds, not measurements of an actual selected output. The Nix command recorder
retains stdout/stderr without imposing a compiler-style file limit; its caller's
resource guard remains necessary. No compiler is invoked here.

The successful result uses schema version 1, scope
`puredarwin_cross_toolchain_installed_wrappers` and status `pass`. It contains
the observed candidate, dependency identities, normalized derivation hashes,
wrapper producers, selected reference bindings, registered closure and complete
layout/file/link inventory. It also retains both verification command records.
Every external command record includes the actual integer exit code, explicit
timeout status and stdout/stderr hashes. The result is written to
`inspection.json` only after verification; it must still be paired with the
producer's successful terminal receipt.

A later consumer supplies `expected=successful_cross_inspection`. Fresh
inspection must match the prior scope/version/status and all substantive
observations. Raw query serialization and registration timestamps are not used
as substitutes for NAR, derivation, reference and file identity.

## Remaining execution and ABI work

The pinned wrappers default to an external SDK path ending in `MacOSX11.3.sdk`;
their `xcrun --show-sdk-version` response is the literal `11.3`. Neither is proof
of the selected source-derived 26.5 SDK. A later bounded compiler invocation
must explicitly set `DARWIN_SDK_ROOT` to the validated SDK output and retain
its actual arguments. The wrapper's prefixed name also does not prove the
effective target: the source sets `clangTarget` to
`x86_64-apple-macosx26.5`, strips deployment-version options and permits later
arguments to affect target selection. These are source observations, not
execution results.

Separate work remains for actual cross realization and output inspection,
compiler invocation/target proof, compiler-rt archive inspection, libSystem and
dyld Mach-O identity/dependency inspection, fixture linking, selected-guest
execution and backend qualification. In particular, compiler-rt is a static
input to libSystem; a final runtime store-reference list need not contain its
archive. Its successful build and actual link provenance require their own
checks. This helper does not change permissive, audit or enforced policy, and
does not promote any backend capability.

The additive [libSystem dylib inspector](M1_LIBSYSTEM_INSPECTION.md) supplies a
source-tested static identity/dependency check for `libSystem.B.dylib` and
`libdyld.dylib`. Its tests do not establish an actual library build or inspection,
and it leaves the separate `dyld` executable and all execution gates unqualified.

## Focused tests

```sh
python3 -m unittest discover -s scripts/tests \
    -p 'test_inspect_puredarwin_cross_toolchain.py' -v
```

The tests cover legitimate external join links, exact script/producer identity,
native and SDK drift, wrong producer output selection, unknown metadata,
missing/extra tools, undeclared references, source paths without derivers,
unsafe links, bounded JSON/script reads, verification failure and timeout,
mutation during verification, reinspection, exclusive evidence paths and final
evidence-write failure. The shared Nix command recorder is exercised with
synthetic subprocess results; no Nix evaluation, build, download or wrapper
execution is needed or claimed. The existing M1 test-discovery step runs these
tests. Only additional path triggers are added to its workflow.
