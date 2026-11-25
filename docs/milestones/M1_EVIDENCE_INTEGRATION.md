# M1 recovered evidence tooling and real link integration

This change incorporates the previously local additive tooling into the M1
branch. It does not replace the existing archive, native-output, cross-toolchain,
libSystem or fixture helpers, or alter their retained source hashes.

## Tools and their boundaries

| Tool | What it establishes | What it does not establish |
| --- | --- | --- |
| `scripts/m1_evidence.py` | Bounded regular-file snapshots, duplicate-safe JSON, exclusive atomic publication after write/flush/sync/close | Authentication of a producer or hostile same-user isolation |
| `scripts/inspect_puredarwin_compiler_rt.py` | An externally hash-bound archive inventory with member offsets/hashes, GNU/BSD names and external symbol classification | The required minimum OS, source/Nix provenance, archive-index semantics or selected compiler-rt qualification |
| `scripts/verify_compiler_rt_link.py` | Consistency of an expanded direct-ld64 capture, exact archive path, freshly reparsed inventory, map member selection and output digest | Authentication of execution claims, output Mach-O semantics, final symbol resolution or libSystem qualification |
| `scripts/inspect_puredarwin_dyld.py` | Static checks of the separate `MH_DYLINKER` artifact | Selected-build provenance, loader execution, guest boot or enforcement |
| `scripts/probe_m1_local_toolchain.py` | Observed commands and logs for three tiny owned compilations, archive construction and a real dylib link/map check | A selected PureDarwin build or execution of Darwin code |

The compiler-rt adapter deliberately remains additional to
[`inspect_puredarwin_archive.py`](../../scripts/inspect_puredarwin_archive.py).
That existing inspector checks externally selected minimum-OS requirements and
required symbol definitions. Neither inspector replaces the other. The adapter
refuses duplicate ordinary member names because a name-only linker map cannot
resolve their occurrence unambiguously; the original inspector still examines
all occurrences by offset. A static pass must not substitute for either the
original target checks or successful producer/NAR evidence.

The source-derived x86_64 libSystem contract links compiler-rt normally and
allows other libraries to be force-loaded. The new map verifier therefore
rejects force-loading the selected compiler-rt archive, global all-load modes,
ambiguous same-basename inputs, unexpanded response/file lists and blanket
undefined-symbol suppression. It does not reject unrelated force-loaded archives.
The selected source remains
`0163b2db225b85e3bf96fc25646ea290105ac298`; its
[libSystem CMake target](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libSystem/stub/CMakeLists.txt)
is the authority for this target-specific distinction.

## Run the source tests and owned link probe

```sh
python3 scripts/run_m1_overlay_tests.py
python3 scripts/probe_m1_local_toolchain.py \
  --clang /path/to/trusted/clang \
  --ar /path/to/trusted/llvm-ar \
  --linker /path/to/trusted/ld64.lld \
  --output-dir /existing/private/parent/new-owned-run
```

The strict runner requires all seven test groups and rejects skipped tests.
Its current suite contains 97 tests, including real Clang/llvm-ar archive-format
cases and actual small subprocess success/failure/timeout tests. The explicit
probe retains tool and helper hashes, full arguments, phase exit codes, streams,
archive inspection, and (only after a successful link) captured map/output
verification. Existing evidence is never overwritten. A failing real command
remains a failure: no alternate linker, target downgrade, metadata removal,
network download or selected-source rebuild occurs.

The probe uses generated source-owned C inputs, not the real compiler-rt
implementation. Its expectation document is generated for this smoke test and
is explicitly not an independently reviewed selected-build expectation. A
successful probe may establish only `owned_archive_and_link_checked`.

The existing Ubuntu 24.04 M1 job explicitly selects its installed
`/usr/lib/llvm-18/bin` tools, runs the complete existing script suite and contract
checks, adds the strict no-skip suite and real probe, and retains the owned-run
directory as an artifact even after failures. It does not add a runner, change
permissions, expand the five-minute job timeout, install a new toolchain, alter
scanner policy or convert a failed experiment into success. Its runtime command
results, not the presence of a binary filename, determine whether linking works.
The runner image's [installation script](https://github.com/actions/runner-images/blob/main/images/ubuntu/scripts/build/install-clang.sh)
installs the versioned Clang and LLD packages; each execution records the exact
installed tool identity rather than claiming a pinned selected-toolchain build.

## Capture contract for actual selected links

A separately trusted producer must capture a JSON object with exactly:

- `schema_version: 2`, `scope: "m1_compiler_rt_link_capture"`, the selected
  `source_revision`, `input_origin` (`owned_fixture` or `selected_source_build`),
  and `argv_kind: "ld64_direct"`.
- `argv`: the expanded direct-linker argument array, not compiler-driver syntax;
  absolute normalized `cwd`; actual integer `exit_code`; boolean `timed_out`.
- `archive`, `link_map`, `output`: each an object containing the original absolute
  normalized `path` and the actual lowercase SHA-256 `sha256`.

Supply the original artifacts or identical recovered copies and independently
selected hashes via `verify_compiler_rt_link.py --help`. The verifier binds map
paths to the original capture, not the recovery directory. It reparses the actual
archive bytes and compares the entire inventory to the v2 inspection receipt.
A claimed exit code inside an otherwise consistent JSON document is not itself
proof that a command ran. Raw producer logs and trusted orchestration remain
necessary. The retained output digest binds bytes but does not inspect their
Mach-O identity or loadability; use the existing libSystem inspector separately.

The archive adapter's expectation schema is version 1, scope
`m1_compiler_rt_expectations`, with `source_revision`,
`target_triple: "x86_64-apple-darwin20.4"`, `archive_sha256` and the exact
`member_names` list. The triple is a reviewed label, not proof of the effective
compiler target. Symbol inventories distinguish concrete definitions, common
symbols, indirect aliases, undefined names and private externals. Unknown load
commands and absent symbol tables are recorded rather than silently treated as
complete target qualification.

## Remaining M1 gates

Recover and independently verify the retained cross-toolchain input evidence;
qualify the bounded producer and actual selected outputs; inspect compiler-rt
using the required target/symbol contract; capture real libSystem links; inspect
libSystem/libdyld and the separate dyld executable; then link and execute native
fixtures in the selected image. Boot, persistence, runtime ABI, application
workflows and security backend qualification still require observed results.

No private deployment records, cloud identifiers, proprietary binaries or raw
shared-host logs belong in this source change. The existing
[staged-library](M1_STAGED_LIBRARIES.md),
[cross-toolchain producer](M1_CROSS_TOOLCHAIN_BUILD.md), and
[dyld inspection](M1_DYLD_EXECUTABLE_INSPECTION.md) contracts retain their scopes.
