# Downstream plans using a verified staged-TAPI native linker

This is an explicit **plan-only** path. It requires an actual successful,
inspected native-linker build and verifies its currently realized output before
planning a downstream stage. Synthetic tests, an existing output directory, a
running build, a TAPI-only success, or a successful metadata plan do not satisfy
that gate. The [recovered native
evidence](evidence/staged-native-linker-recovered-2026-11-14.json) now establishes
the historical successful build and installed-output inspection, including the
complete result, sidecars and logs. Independent review accepted that receipt
chain. The unchanged consumer must still perform fresh current-output and
source verification before planning; this evidence does not claim an actual
downstream plan or realization.

The existing linker, libraries, object and fixture helpers retain their original
bytes and defaults. The new files are
[`puredarwin_staged_libraries.nix`](../../scripts/puredarwin_staged_libraries.nix),
[`plan_puredarwin_staged_libraries.py`](../../scripts/plan_puredarwin_staged_libraries.py)
and the shared
[`inspect_puredarwin_native.py`](../../scripts/inspect_puredarwin_native.py).

## Coherent package binding

The selected PureDarwin source is still revision
`0163b2db225b85e3bf96fc25646ea290105ac298`, with the existing immutable lock and
source-derived SDK. The new composition reconstructs staged TAPI using the
unchanged reviewed stager, then constructs an **unbounded** native linker,
cross-toolchain, compiler-rt and libSystem baseline using that TAPI.

It passes this coherent baseline to the unchanged
[`puredarwin_libraries.nix`](../../scripts/puredarwin_libraries.nix). That recipe
applies the existing one-job bounds and compiler-rt staging. The resulting
bounded native linker must equal both the unchanged staged-linker recipe's
candidate and the inspected, realized native output.

Only replacing `native-ld` in an otherwise original package set would leave the
library phase comparisons referring to a different baseline. Passing an already
bounded linker would repeat the guarded Ninja replacement. The new composition
avoids both cases without weakening those assertions or duplicating their
implementation.

| Stage | Dependency selection and unchanged behavior |
| --- | --- |
| `cross-toolchain` | Rebind the existing clang, clang++ and ld wrappers to the verified native linker. Other wrappers and the join command stay unchanged. LLVM-source and libSystem evaluation remain lazy. |
| `compiler-rt` | Use that cross-toolchain, the unchanged complete LLVM source and visibility patch, existing reduced writable staging, and one-job build/install commands. Preserve Release and all target flags. |
| `libsystem` | Use the same native linker, cross-toolchain and bounded compiler-rt. Preserve the selected source, Debug configuration, target list, install phase and other inputs. |

Every selected stage records original upstream, previous bounded,
TAPI-rebound unbounded, and new candidate derivation/output identities. Baseline
checks compare phases after only the intended dependency-path replacements.
The existing recipe's assertions then check the bounded result.

These bindings follow the pinned
[`flake.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.nix),
[`toolchain.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/toolchain.nix),
[`compiler-rt.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/compiler-rt.nix)
and [generic build recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/build.nix).

## Required native success receipt

A separately reviewed native build driver must produce schema version 1,
scope `staged_libtapi_native_linker_build`, status
`staged_libtapi_native_linker_built`, `realization_command_succeeded: true`, and
`output_inspection_status: "pass"`. The actual native build has produced those
values, and its complete recovered receipt and inspection sidecar passed
independent review and the receipt validator. The public sanitized receipt is
not a substitute for the original private inputs or the fresh checks below.

The required receipt contains:

- `plan_result_sha256`, `plan_burdettie_revision`, and the build's separate
  `burdettie_revision`; `started_at` and `finished_at` must form a valid UTC interval.
- `helper_sha256` identifying the unchanged staged-linker planner,
  `output_inspector_sha256` identifying the shared inspector used after realization,
  and `driver_sha256` matching the externally supplied, independently reviewed
  driver checksum.
- The exact plan fields `recipe_sha256`, `libtapi_recipe_sha256`,
  `native_linker_recipe_sha256`, `stager_sha256`, `libtapi_plan_helper_sha256`,
  `source`, `sdk`, and `successful_libtapi`.
- Every field named by the planner's `NATIVE_FIELDS`: original, previous
  bounded, rebound and candidate tuples; staged-TAPI recipe record; unchanged
  field proof; build inputs; configure/build phase hashes; and job-patch record.
- `realized_output` equal to the candidate output and successful `phases`,
  including exactly one phase named `staged-native-linker-build`. Every successful
  exit code must be the integer zero; JSON booleans, floats and strings are rejected,
  and each new native phase must explicitly record `timed_out: false`.
- `realization` containing the absolute `nix_executable`, absolute `out_link`,
  `out_link_was_absent: true`, and the exact `argv` used by that build phase.
- `output_inspection` returned by the shared inspector, plus its retained raw
  `output-path-info.json` sidecar. The registered output binds path, exact
  deriver, canonical SHA256 NAR hash, positive NAR size and sidecar checksum.

The successful native plan must retain its checksummed `libtapi-plan.json`,
`libtapi-build-result.json` and `libtapi-output-path-info.json` sidecars. The new
consumer checks this earlier successful TAPI chain as well as the native chain.
It reads receipts with a bounded size and rejects duplicate JSON keys. Recorded
realization commands are compared with commands constructed by the reviewed
helper; receipt text cannot supply an arbitrary command to execute.

The original plan's realization argv must match the unchanged linker's command
construction, including its historical absence of `--store local`. The actual
build argv must add the direct-local store prefix and name the same candidate
`drv^out`. Only its absolute Nix executable and new output link are parameterized.
The comparison preserves one job, one core, two substitution jobs, empty remote
builders, `min-free 0`, `--keep-failed`, `--print-build-logs`, `--json` and the
single output link. The recorded build phase must contain that exact argv.

Earlier plan/build revisions need not equal the new consumer's revision. The
receipt records the separate revisions and requires the recipe/helper/stager
bytes involved in those transitions to match. This permits the new helper to
consume unchanged previously built inputs without pretending it existed at the
earlier revision.

### Shared output inspection

After validating the source/plan and successfully realizing the candidate, a
native driver can call:

```python
inspection = inspect_native_output(
    candidate=plan["candidate"],
    dependencies=plan["current_registered_outputs"],
    original_tapi_output=plan["staged_libtapi"]["upstream"]["out"],
    directory=new_evidence_directory,
    report=build_report,
)
```

The dependencies are the exact previously observed registered TAPI and SDK
identities. The function is not a build driver or a replacement for validating
the plan. A consumer also supplies `expected=successful_output_inspection`.

The shared function performs direct-local offline `path-info` checks. It records
every regular output file's size, hash and mode, every symlink's text/resolution,
and ELF64 little-endian x86_64 headers. The pinned `ld` wrapper, `ld.real` and
`strip` must be regular executables; output links must resolve inside the package.

Store references must include the exact staged TAPI output and exclude the
original TAPI output. Other references, including legitimate source/SDK paths,
are retained for review. The following checks distinguish static metadata from
observed host-loader behavior:

1. A bounded ELF parser reads `ld.real` program headers, its absolute `PT_INTERP`,
   `DT_NEEDED`, and effective `DT_RUNPATH` or `DT_RPATH`. Exactly one direct TAPI
   dependency must resolve inside the verified staged package. An absolute
   `DT_NEEDED` is supported; relative paths, loader-token expansion, host-cache
   lookup and audit/filter indirection are outside this narrow contract and fail
   inspection. The source's versioned `.so.12git` library name is supported.
2. The existing interpreter must be executable inside one of the native output's
   registered store references. Fresh metadata binds that loader's deriver and
   NAR identity. Default offline `nix store verify` then checks the native, TAPI,
   SDK and loader outputs, without repair or verification waivers.
3. The checked interpreter runs `--list` against the exact `ld.real` path. Its
   observed TAPI resolution must normalize to the same inspected staged library.
   The receipt keeps this observation separately from static ELF metadata; raw
   streams remain available, while changing ASLR addresses are not identities.
4. The checked native wrapper runs `ld -v`. Its output must identify `ld`, `ld64`,
   x86_64 support and the pinned TAPI version `1300.6.5`, matching the installed
   `Version.inc`. The linker version token is retained as emitted, including an
   unresolved macro token if that is what the selected source produces; no
   numerical linker version is invented.

Both host probes have a ten-second timeout, a 64-KiB limit for each retained
stdout/stderr file, and an environment containing only `PATH=/usr/bin:/bin` and
`LC_ALL=C`. No loader override is inherited. The child-only file-size limit
applies to these probes, never to Nix metadata/database operations. Exceeding
either bound fails inspection. The consumer checks the prior native and loader
NAR identities, file inventory, dynamic binding, normalized loader resolution,
and version smoke result; it rechecks the output inventory and TAPI binding after
the probes. It does not require an invented successful `strip --help` invocation.
These checks establish a narrow host-tool inspection, not Darwin fixture linking
or general ELF/runtime compatibility.

The version checks follow the pinned
[`Options.cpp`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/tools/cctools/ld64/src/ld/Options.cpp)
and [`helper.c`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/tools/cctools/ld64/src/3rd/helper.c),
and TAPI's selected
[`build.sh`](https://github.com/tpoechtrager/apple-libtapi/blob/640b4623929c923c0468143ff2a363a48665fa54/build.sh)
and [`Version.cpp`](https://github.com/tpoechtrager/apple-libtapi/blob/640b4623929c923c0468143ff2a363a48665fa54/src/tapi/tools/libtapi/Version.cpp).
The dependency-list output format is defined by glibc's
[`rtld.c`](https://github.com/bminor/glibc/blob/glibc-2.42/elf/rtld.c);
the inspector always uses the already-realized loader selected by the actual
ELF and binds its current registered identity.

All read and smoke streams are retained. A failed live verification, changed
file, missing output, smoke timeout, failed metadata query, changed helper bytes,
or failed final evidence write remains a nonzero **failure**. Invalid or missing
prerequisite receipts are **blocked** before downstream evaluation or planning.

## Plan one stage

Use private paths and checksums from actual inspected receipts and a new private
output directory with an existing parent:

```sh
python3 scripts/plan_puredarwin_staged_libraries.py \
  --stage cross-toolchain \
  --puredarwin-source "$PUREDARWIN_SOURCE" \
  --native-linker-plan "$NATIVE_LINKER_PLAN" \
  --native-linker-plan-sha256 "$NATIVE_LINKER_PLAN_SHA256" \
  --native-linker-build-result "$NATIVE_LINKER_BUILD_RESULT" \
  --native-linker-build-result-sha256 "$NATIVE_LINKER_BUILD_SHA256" \
  --native-linker-driver-sha256 "$REVIEWED_NATIVE_DRIVER_SHA256" \
  --expected-sdk-derivation "$SDK_DRV" \
  --expected-sdk-output "$SDK_OUT" \
  --output-dir "$NEW_PLAN_DIRECTORY" \
  --query-substituters
```

Evaluation always stays offline, uses the direct local store, forbids lock
changes and disables import from derivation. The default dry-run is an offline
graph only. `--query-substituters` allows cache metadata queries during the
dry-run; it does not download or realize dependencies. The helper records the
source/code snapshots, all receipts, fresh checks and dry-run result.
The snapshots also bind the imported existing library planner, native planner,
base helpers and shared output inspector.

The one-core/one-job realization argv is recorded only, with two substitution
jobs, no remote builders, no automatic garbage collection, retained failed
builds and a new output link. It is never executed by this helper. A separate
reviewed runner must apply the owner's existing CPU, memory, reserve, growth,
inode and time limits after inspecting the actual plan and taking a fresh host
census. The [cross-toolchain producer](M1_CROSS_TOOLCHAIN_BUILD.md) implements
that next step for an offline cross-only plan inside an existing bounded
service; it does not create or enlarge the host environment. Actual realization
still requires the retained private evidence and a fresh host assessment.
Every successful plan leaves `peak_space_qualified: false`.

## Later fixture path and open gates

No new fixture-compilation entry point is introduced here. After actual
downstream outputs and inspections exist, a separate opt-in entry point should
verify their receipt/NAR chain and pass the selected package map into the
existing fixture helper's `tool_paths`, `compile_command` and `inspect_fixture`
functions. Its default remains no-build preflight, with explicit compilation
into a new directory and the same strict linker/load-command checks. Calling
the old helper unchanged would still resolve the original upstream packages.

The [bounded-library gates](M1_BOUNDED_BUILD_RECIPES.md) remain open: full LLVM
source/store and build peaks, actual compiler target precedence, checked archive
member/symbol inspection despite the upstream optional `otool` diagnostic, and
actual libSystem/dyld output qualification. Target/build flags are unchanged.
Fixture execution, ABI behavior, selected-source guest boot and all security
backends require their own observed results.

The focused tests use the real pinned Nix evaluator with small synthetic
package records, source-owned synthetic ELF program headers/dynamic tables,
and mocked Nix/loader/version responses. They test coherent propagation, lazy
cross-stage evaluation, unchanged modes/targets, missing/failed receipt rejection,
exact integer success codes, build-command bounds, current-output drift, effective
TAPI search order, loader-resolution mismatches, nonzero failure preservation and
metadata-only behavior. A small source-owned Python child checks the actual
output-size bound. They neither build nor execute a native linker. The existing
M1 CI job requires Nix so the graph tests cannot silently skip; its path filters
include these new files.
