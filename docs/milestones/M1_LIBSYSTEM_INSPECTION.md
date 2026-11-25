# Selected libSystem dylib inspection

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

Status: implemented with source-owned byte fixtures. **The selected libSystem
output has not been built or inspected by this change. M1 remains incomplete.**

[`inspect_puredarwin_libsystem.py`](../../scripts/inspect_puredarwin_libsystem.py)
checks the static identities and dependency paths of two already available
files: `usr/lib/libSystem.B.dylib` and `usr/lib/system/libdyld.dylib`. It does not
invoke a compiler, linker, Nix, loader or subprocess. It does not download,
realize, repair or execute an input.

The [existing fixture helper](../../scripts/build_puredarwin_fixtures.py) checks
that these files exist and records their hashes. Its executable checks do not
inspect their identities or transitive dependency paths. This separate helper
provides that narrower check without changing the frozen fixture, inventory,
planner, recipe or cross-toolchain helpers and their retained hashes. A later
qualified fixture runner must adopt it explicitly after verifying actual build
and installed-output provenance.

## Contract derived from the selected source

The contract is specific to PureDarwin revision
`0163b2db225b85e3bf96fc25646ea290105ac298`, baseline x86_64 and macOS deployment
26.5. It is not a policy for arbitrary macOS libraries.

| File role | Required `LC_ID_DYLIB` name | Required direct dependency |
| --- | --- | --- |
| `libsystem` | `/usr/lib/libSystem.B.dylib` | One reexport of `/usr/lib/system/libdyld.dylib` |
| `libdyld` | `/usr/lib/system/libdyld.dylib` | One required load of `/usr/lib/libSystem.B.dylib` |

Both identities must record current version `1292.100.5` and compatibility
version `1.0.0`. The selected
[libSystem target](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libSystem/stub/CMakeLists.txt)
sets its install name and versions, and links/reexports the libdyld target. The
[libdyld target](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libSystem/libdyld/CMakeLists.txt)
sets the other install name and the same versions, and links `-lSystem`.
Their minimum-OS declarations are 26.5. The common
[shared-library function](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/cmake/crosscompile.cmake)
applies the install-name and architecture options and disables CMake rpaths.
These source declarations establish what this inspection requires; they do not
establish which commands an unobserved build actually emitted.

The selected [installation phase](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/build.nix)
copies these two files and separately installs the `usr/lib/dyld` executable.
**This helper does not inspect that executable.** A `libdyld.dylib` pass cannot
be substituted for a dyld-executable or guest-loader result.

## Reviewed inputs and invocation

Use files and their exact SHA-256 hashes from independently verified installed
output evidence. Before running the helper, prepare a private JSON expectation
file and independently review/hash it. The helper does not generate expected
values from the files it is about to check. Its schema is:

```json
{
  "schema_version": 1,
  "scope": "m1_libsystem_dylib_expectations",
  "source_revision": "0163b2db225b85e3bf96fc25646ea290105ac298",
  "minimum_os": "26.5.0",
  "sdk_version": "<reviewed major.minor.patch>",
  "files": {
    "libsystem": {
      "sha256": "<independently verified file SHA-256>",
      "dependencies": [{
        "name": "/usr/lib/system/libdyld.dylib",
        "load_kind": "reexport",
        "current_version": "<reviewed dependency current version>",
        "compatibility_version": "<reviewed dependency compatibility version>"
      }]
    },
    "libdyld": {
      "sha256": "<independently verified file SHA-256>",
      "dependencies": [{
        "name": "/usr/lib/libSystem.B.dylib",
        "load_kind": "required",
        "current_version": "<reviewed dependency current version>",
        "compatibility_version": "<reviewed dependency compatibility version>"
      }]
    }
  }
}
```

The placeholders are intentionally invalid inputs. Replace them with reviewed
values from the target decision and exact link-input/producer evidence. Dylib
identity versions, the version requirements recorded in dependency commands,
and the SDK field are distinct quantities. The helper does not invent a
dependency version from the peer's filename or assume that an SDK version is
proven by the deployment target. Dependency compatibility requirements must be
no greater than the inspected peer's current version. A missing/unexpected edge
or metadata discrepancy remains a finding requiring source/producer review;
there is no flag that ignores it or adds an arbitrary dependency.

```sh
python3 scripts/inspect_puredarwin_libsystem.py \
  --libsystem "$VERIFIED_LIBSYSTEM_FILE" \
  --libdyld "$VERIFIED_LIBDYLD_FILE" \
  --expectations "$REVIEWED_EXPECTATIONS" \
  --expectations-sha256 "$REVIEWED_EXPECTATIONS_SHA256" \
  --output-dir "$NEW_PRIVATE_INSPECTION_DIRECTORY"
```

For a fresh comparison with a prior successful static receipt, add both
`--previous-inspection "$PREVIOUS_RESULT"` and
`--previous-inspection-sha256 "$PREVIOUS_RESULT_SHA256"`. The prior schema,
scope, success status, inspector hash, expectation hash, inspected contents and
limitation fields must match. A plan, failed receipt, altered receipt or receipt
from another helper cannot become a successful reinspection. Input filesystem
locations may differ; the comparison binds the inspected bytes and metadata.

This caller-supplied expectation file is not a selected-source build receipt.
The helper does not validate a Nix derivation, store registration, NAR, compiler
runtime archive, source checkout or successful producer command. Those remain
separate gates in the [staged library path](M1_STAGED_LIBRARIES.md). Verify the
reviewed helper before executing Python; its own recorded hash and repeat
checks detect drift during this operation, not hostile code already executing
under the same account.

## Static checks and retained evidence

Each input is read into one bounded snapshot through a nonblocking,
no-final-symlink file descriptor. The helper checks regular-file type, size,
descriptor/path identity and timestamps across the read, then the externally
selected hash. It reads the inputs again before success to detect drift.

For each dylib it requires a thin little-endian baseline x86_64 `MH_DYLIB`,
exactly one source-selected identity, exactly one macOS `LC_BUILD_VERSION` with
the selected minimum and reviewed SDK field, and the exact reviewed dependency
record. Relative paths, host paths, extra dependencies, weak replacement edges,
rpaths, encryption, foreign/legacy version commands and unsupported commands
fail. Dependency paths resolve only to the other inspected member of this pair;
the inspector never searches the host filesystem or falls back to a system
library.

The parser requires zero padding before and after each load-command path string,
and bounds load-command counts, sizes and alignment, segment file/VM
ranges, section data and relocations, symbol/string tables, dynamic-symbol
subranges and the known link-edit payload ranges. It checks a mapped executable
`__TEXT` header region and refuses overlapping segments. Supported header flags
are explicitly enumerated; unknown, prebound, cache, simulator, alternate
namespace and executable-only modes fail. Segment flags are limited to
`SG_NORELOC` and `SG_READ_ONLY`: protected/encrypted, high-VM, fixed-VM and unknown
mapping modes fail. Sections must use a known type and attribute mask; ordinary
file-backed section addresses must match the segment's VM-to-file mapping,
and zero-fill sections must occupy its zero-filled tail with file offset zero.
These flag checks do not verify security properties named by a header flag.
It bounds but does not
interpret dyld fixup/export payloads, code signatures or relocation semantics.
The format references are the pinned
[LLVM Mach-O definitions](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.8/llvm/include/llvm/BinaryFormat/MachO.h)
and the selected XNU [load-command definitions and padding requirements](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/xnu/EXTERNAL_HEADERS/mach-o/loader.h).

Limits are 128 MiB per dylib, 4 MiB of load commands, 4,096 commands, 255
sections, 262,144 symbols, 1,024 bytes per load-command path, 8,192 bytes per
symbol name and 512 KiB per JSON input. These are defensive parser bounds, not
build-capacity estimates or new host resource allocations. An otherwise valid
file beyond this supported subset requires a separate reviewed inspection.

The evidence directory must be new, have an existing parent and remain outside
the Nix store. It receives the exact expectation snapshot, the optional prior
receipt snapshot and a terminal `result.json`. The terminal receipt is written
to an exclusive partial file, flushed, synced and closed before exclusive
atomic publication. `result.json.partial` is deliberately retained and is never
terminal evidence. A write, flush, close or publication failure returns failure
and cannot publish a success receipt. Prior directories are never overwritten.

| Result | Meaning |
| --- | --- |
| `pass`, exit 0 | Both inspected byte snapshots satisfy this exact static identity/dependency contract and the terminal receipt was retained. |
| `fail`, exit 1 | Malformed/unsupported bytes, changed inputs, expectation or prior-receipt mismatch, missing required metadata, or failed evidence retention. |
| `blocked`, exit 2 | A new permitted evidence directory cannot be established. |

## Remaining qualification

The source deliberately permits named Objective-C undefined references in
libSystem with `-Wl,-U` options and expects runtime lookup. A blanket assertion
that no undefined symbols exist would contradict the selected source. This
helper checks dependency **paths**, not exported/imported symbol satisfaction,
flat-namespace bindings, initializer order, runtime availability, ABI behavior
or the presence of code implementing every required operation.

Every receipt leaves `source_identity_verified`, `sdk_provenance_verified`, `build_provenance_verified`,
`registered_output_verified`, `symbol_binding_checked`, `fixup_semantics_checked`,
`signature_verified` and `selected_libsystem_qualified` false. Dyld-executable
inspection, fixture linking, guest execution, Burdettie runtime execution and
security enforcement remain `not_run`, including after a static pass. Actual
compiler-rt inspection and producer/link provenance still precede a qualified
fixture link; a final runtime reference graph cannot prove which static archive
was linked.

The focused suite uses source-generated Mach-O records and ordinary private
temporary files. It covers wrong architecture/platform/install identity and
dependency fields, malformed/truncated tables and strings, symlink/FIFO refusal,
bounded reads, input/helper mutation, exact receipt binding, exclusive evidence
directories and failed terminal publication. Regression cases also reject
protected/high-VM segments, unknown header/section flags and inconsistent
section mappings even when the supplied file hashes match. It does not inspect a selected
build, run a Darwin program or change any security backend:

```sh
python3 -m unittest discover -s scripts/tests \
  -p 'test_inspect_puredarwin_libsystem.py' -v
```
