# M1 native sandbox operation probe

This is an executable qualification **input**, not a security backend or a
qualified control. The [C fixture](../../fixtures/m1_sandbox_probe/probe.c)
tests a narrow native PureDarwin file policy against real operations. The
[runner and capture validator](../../scripts/run_m1_sandbox_probe.py) retain
results and reject missing, inconsistent or incomplete evidence.

The selected PureDarwin guest has not executed this probe. All 17 control
families in the [backend allocation](../contracts/BACKEND_ALLOCATION.md) remain
unsupported and unavailable. This change does not enable an enforced launch or
implement permissive/audit/enforced mode selection. Applying a restrictive
profile just to obtain audit observations would violate that mode distinction.

## What the executable measures

The parent creates a new, private fixture directory and nonce-bearing files.
It first starts an unsandboxed baseline worker. That worker performs eight
operations, forks a descendant, collects the descendant's same eight
operations, and reports both actual exit statuses. A separate restricted worker
opens its retained descriptors, installs one native profile, performs its eight
operations, then forks a descendant to test inherited file restrictions.
Neither worker replaces an existing profile or calls Burdettie's compatibility
`sandbox` shims. The profile is:

```scheme
(version 1)
(allow default)
(deny file-read-data file-write-data file-write-create
      (subpath (param "DENY_ROOT")))
```

`DENY_ROOT` is supplied as a fixed key/value parameter pair, followed by a null
terminator. Its nonempty absolute value is derived from `realpath` before any
policy is applied. The deny directory and allowed sibling directory contain
separate files for the worker and descendant. The literal profile is passed to
`sandbox_init_with_parameters` with flags zero; no Apple compiled-profile blob
or `sandbox_check` query is treated as enforcement evidence.

| Case, run in both worker and descendant | Actual operation | Limited success requirement after installation |
| --- | --- | --- |
| Allowed read | Fresh `open` followed by `read` | Exact nonce-bearing contents |
| Allowed write | Fresh `open` followed by `write` and `fsync` | One byte written, with no synchronization error |
| Allowed create | Fresh `open(O_CREAT | O_EXCL)` and write | New sibling file created and written |
| Forbidden read-open | Fresh `open(O_RDONLY)` | `-1` and `EPERM`; no read is attempted |
| Forbidden write-open | Fresh `open(O_WRONLY)` | `-1` and `EPERM`; no write is attempted |
| Forbidden create-open | Fresh `open(O_CREAT | O_EXCL)` | `-1` and `EPERM`; the file remains absent |
| Retained read descriptor | Actual `pread` after installation | `-1` and `EPERM` from that syscall |
| Retained write descriptor | Actual `pwrite` after installation | `-1` and `EPERM` from that syscall |

The parent also reads the restricted write sentinels after the worker exits and
checks that their complete contents remain unchanged, including the retained
write sentinel. It checks that both forbidden create files remain absent.
Writes to the preopened result pipe are IPC only and do not count as an allowed
file-write positive. Retained descriptor cases are distinct from newly opened
files; an open denial cannot stand in for an unattempted read/write denial.
Each case records the close of its used descriptor, including retained ones.

The post-install fork tests inheritance of this file profile. The profile does
not deny fork, and the probe does not qualify fork prevention, exec inheritance,
daemonization, process-tree resource limits, network policy, file mappings,
rename/alias handling, object grants or arbitrary applications.

## Pinned source basis and expected gaps

The source basis is PureDarwin
[`0163b2db225b85e3bf96fc25646ea290105ac298`](https://github.com/PureDarwin/PureDarwin/tree/0163b2db225b85e3bf96fc25646ea290105ac298).
These are source findings, not observations from the selected guest:

- [`sandbox.h`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libsandbox/include/sandbox.h)
  declares the parameterized API and error deallocator. [`sandbox.c`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libsandbox/sandbox.c)
  compiles native profiles and invokes `__mac_syscall("Sandbox", 0, ...)`.
  A missing policy, compilation failure or rejected installation remains a
  nonzero blocked result; returning zero by itself does not prove restriction.
- [`sbpl.c`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Libraries/libsandbox/sbpl.c)
  resolves parameter pairs and treats an unset parameter as matching nothing.
  The harness supplies the required canonical value explicitly. That compiler
  also accepts `process-fork` as an unenforced operation; no emitted fork rule
  should be assumed. Its native format uses `SB_PROFILE_MAGIC = 0x42535044`
  and version one.
- [`Sandbox.kext`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/Extensions/Sandbox/sandbox_kext.c)
  uses the last matching native rule, compares canonical vnode paths, and maps
  fresh read/write opens to separate operation bits. Create has its own hook.
  Denials return `EPERM`. Its hook table has no vnode read/write hook and no
  process-fork hook. Non-native Apple profiles can instead become permissive
  markers, and the Apple operation-check compatibility path answers permissively.
- Pinned XNU [`vfs_vnops.c`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/xnu/bsd/vfs/vfs_vnops.c)
  calls separate MAC read/write checks for descriptor I/O. Together with the
  missing sandbox hooks, this suggests a retained-descriptor bypass. The probe
  measures that candidate gap directly and returns nonzero if observed; it
  does not omit these cases to obtain a passing result.
- [`mac_process.c`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/xnu/security/mac_process.c)
  and [`kern_fork.c`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/xnu/bsd/kern/kern_fork.c)
  describe/retain shared parent credentials on fork. The sandbox credential
  association hook retains the profile. This is a reason to test descendant
  file opens, not a substitute for their actual results.

Even if every case eventually passes, this path-based policy does not implement
Burdettie's owner-resolved object/tree grants, deny precedence, authenticated
launch ownership or the other required controls. The parser always returns
`selected_guest_qualified: false` and an empty `qualified_capabilities` list.

## Results and failure classification

The C program emits one bounded JSON object with a caller-supplied nonce,
process IDs, private process-group ID, actual worker/descendant exit statuses,
API result/errno, per-operation results and exact-sentinel checks. Its parent
accepts IPC only from its own forked worker and waits for its exit. The validator
binds the report to a separately observed process exit and, when locally run,
the actual launched PID. It checks exact role/case counts and recomputes the
result from individual operations. Duplicate JSON keys, wrong nonces, wrong
exit codes, incomplete descendants and fabricated capability promotion fail.

| Status | Exit | Meaning |
| --- | --- | --- |
| `passed_limited_probe` | 0 | All baseline, allowed-sibling, fresh-open, retained-I/O and file-preservation checks passed in this execution only |
| `coverage_gap` | 1 | The native profile installed and both roles completed, but a required operation or preservation check failed |
| `blocked_policy` | 2 | Baseline succeeded; the restricted worker could not install the native policy, and did not run restricted cases |
| Setup, baseline, incomplete or interrupted result | 2 | Prerequisite, transport, child, timeout or execution failure; no passing qualification evidence |

There is no missing-fixture success or skip. A Linux build deliberately reports
the unavailable native API, not a simulated policy. A profile that only denies
fresh opens while allowing retained reads/writes yields `coverage_gap`, even if
all allowed siblings and descendant fresh-open denials behave as expected.

## Build, execution and preservation

The fixture is standalone C11 with libSystem calls. A selected-guest build must
use the inspected source-owned compiler, selected SDK, libSystem/dyld and exact
input receipts. The existing [fixture compiler](../../scripts/build_puredarwin_fixtures.py)
provides the reviewed target flags and `compile_command`/`inspect_fixture`
functions. Its frozen default fixture list is unchanged; this new source is not
silently added to a running build or dependency plan. The selected SDK/native
link step for this probe remains unexecuted.

On a prepared guest, invoke the exact inspected binary with a **new absolute
directory** and a unique ASCII nonce of 1–64 letters, digits, hyphens or
underscores. Preserve stdout, stderr and the guest process's actual exit code:

```sh
/absolute/inspected/m1_sandbox_probe /absolute/new/probe-fixtures run_nonce
```

The C parent uses its own process group. Its workers and descendant remain in
that group; a ten-second alarm per worker phase signals only that group, with a
two-second hard termination fallback. Setup and final output still require an
outer bounded guest/VM supervisor. Do not run the guest command without that
outer bound, and never kill processes by global name or numeric UID.

When Python is available in the execution environment, the optional runner
creates an exclusive output directory, copies and verifies the supplied binary
under an exact SHA256, and executes that retained snapshot:

```sh
python3 scripts/run_m1_sandbox_probe.py run \
  --executable /absolute/inspected/m1_sandbox_probe \
  --sha256 "$PROBE_SHA256" \
  --output /absolute/new/probe-capture --nonce run_nonce
```

It bounds the executable to 4 MiB and each stdout/stderr file to 64 KiB, with an
execution deadline of at most 30 seconds followed by up to two seconds each for
TERM and KILL leader-exit waits. It disables core dumps, starts a new session,
and uses a minimal environment. Timeout cleanup targets only the process group
it created while the leader remains unreaped. The wrapper records only
`leader_exit_observed`; `descendant_termination` stays `unverified`, and it never
signals an old group ID after reaping the leader. Normal C reports separately
bind the exits of their known worker and descendant. Abort paths cannot reuse
that normal protocol as proof of whole-group cleanup. These are harness bounds,
not a general application confinement or
process-tree resource backend. Existing output paths are rejected. All new
fixtures, executable snapshots, raw logs and result files remain in place on
success, block or failure; there is no cleanup option.

Python need not be present in the selected guest. The host can validate its
captured JSON and independently retained guest exit status:

```sh
python3 scripts/run_m1_sandbox_probe.py inspect \
  --stdout /absolute/captured/probe.stdout \
  --exit-code "$OBSERVED_GUEST_EXIT" --nonce run_nonce
```

This accepts a transcript, not a self-authenticating machine identity. Keep the
image digest, observed kernel, source revisions, compiler/input receipt, binary
hash, exact command, guest stdout/stderr and exit capture together. Never infer
selected-guest provenance from a string compiled into the program or from an
arbitrary host run. Execution records contain private paths/PIDs and belong in
private evidence, not a public source commit.

## Local validation and remaining gate

The focused tests require Linux and an available C compiler; absence is an
error, not a skip:

```sh
python3 -m unittest discover -s scripts/tests -p 'test_m1_sandbox_probe.py' -v
```

They compile the real C program with `-Wall -Wextra -Werror`, exercise the actual
Linux baseline and API-unavailable result, verify retained fixtures, reject
output reuse/wrong hashes, and exercise timeout/output bounds. Synthetic JSON
unit inputs check denial/retained-handle/child/exit/result semantics. A small
synthetic declaration header also syntax-checks the Darwin API branch without
linking or calling it. That test is **not selected SDK compilation**. The M1
workflow retains its existing checks and adds path triggers for this harness.

The remaining gate is a source-built, provenance-bound run inside the selected
booted guest, followed by diagnosis of every observed gap. This probe does not
make M1 complete and does not change any security capability's availability.
