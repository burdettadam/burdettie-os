# Bounded cross-toolchain realization

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

[`build_puredarwin_cross_toolchain.py`](../../scripts/build_puredarwin_cross_toolchain.py)
connects a retained cross-toolchain metadata plan to the existing
[installed-wrapper inspector](M1_CROSS_TOOLCHAIN_INSPECTION.md). It is a source
implementation with synthetic receipt tests and small owned subprocess tests.
It has **not** been run against the selected AWS toolchain. It does not recover
or independently verify the pending cross-toolchain archive, and does not close
that prerequisite or any M1 execution gate.

The default operation revalidates inputs and stops with
`cross_toolchain_ready_not_built`. The explicit `--realize` option permits one
bounded realization command and fresh installed-output inspection. A successful
`cross_toolchain_built` receipt means that exact command succeeded and its exact
output passed inspection. Nix can reuse an already valid output; this status
does not establish that compilation or a rebuild occurred. The receipt records
`candidate_preexisting`, `rebuild_requested: false`, and
`rebuild_execution: "not_established"`.

## Prerequisites remain external

Before executing the producer, recover and independently verify the complete
cross-toolchain metadata archive, including its original plan, dry-run log,
native build/plan receipts, TAPI sidecars, and helper identities. Use the exact
observed checksums; the producer has no successful-receipt defaults.

The private launcher must check the existing instance, remaining allowance,
shared workloads, concurrent experiments, current capacity, and automatic-stop
safeguards. The producer neither creates nor changes infrastructure, installs
software, modifies a service, extends resources, cleans the store, or checks an
EC2 stop timer. It records
`automatic_stop_guard: "not_checked_by_this_helper"` explicitly. A stale or
missing host safeguard must be resolved by the authorized launcher before use.

An existing dedicated systemd service must contain the producer as its sole
main process. The producer reads the service and cgroup and requires:

| Requirement | Observed condition |
| --- | --- |
| Identity and lifecycle | Exact requested service, `Type=exec` or `simple`, current nonroot user, current process as `MainPID`, no control process, active/running, exact cgroup |
| Descendant cleanup | `KillMode=control-group`, `ExitType=main`, `RemainAfterExit=no`, `SendSIGKILL=yes`, TERM then KILL, `Delegate=no` |
| Network isolation | `PrivateNetwork=yes`, empty `NetworkNamespacePath` and `JoinsNamespaceOf`, `NoNewPrivileges=yes` |
| Service time limits | Finite `RuntimeMaxSec` no greater than 600 seconds and `TimeoutStopSec` no greater than 25 seconds |
| CPU | Finite quota no greater than 25% of one CPU; CPU weight at most 10 |
| Scheduling and I/O priority | Nice value from 15 through 19; explicit I/O weight from 1 through 10 |
| Memory and tasks | `MemoryHigh` at most 1,280 MiB, `MemoryMax` at most 1,536 MiB, zero swap, at most 128 tasks |
| Exclusive process boundary | Only the producer before and after each phase; census includes nested cgroups; the build owner cannot write the containing cgroup directories |

These are acceptance conditions for an already prepared service. The helper
never uses `systemctl start`, `stop`, `set-property`, or a service-creation
command. The service lifecycle matters: a process group cannot contain a child
that creates a new session. Remaining descendants prevent success; the verified
containing service must clean them up when its main process exits. Direct
cleanup signals only each process group created by this operation, never a UID
or process name. SIGTERM enters failure cleanup and prompt receipt handling.

The service semantics are defined in systemd's
[`KillMode`](https://www.freedesktop.org/software/systemd/man/latest/systemd.kill.html#KillMode=),
[`ExitType`](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#ExitType=),
and [`PrivateNetwork`](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html#PrivateNetwork=)
references. Observing the required configuration and testing the local cleanup
code do not substitute for qualifying the actual containing service on the
selected host.

## Input and command binding

Required CLI inputs identify the retained cross plan and its SHA256, retained
native plan and build result, independently reviewed native-driver SHA256,
selected PureDarwin checkout, reviewed Nix executable and SHA256, reviewed code
manifest and SHA256, a new private output directory, and the existing service
name. `python3 scripts/build_puredarwin_cross_toolchain.py --help` lists them.

`--cross-plan` selects the **inner** staged-libraries planner `result.json`, whose
scope is `staged_native_linker_library_plan_only`, with its adjacent retained
`dry-run.log`. It does not accept the outer metadata-driver receipt. Independent
verification of that outer receipt and its complete archive remains a launcher
prerequisite.

The code manifest is an externally reviewed JSON object with exactly
`schema_version: 1`, `scope: "reviewed_cross_toolchain_source"`, and a `sha256`
map naming every file in the producer's `CODE` mapping. That set contains the
unchanged planner's recipe/helper inputs, the planner itself, the producer,
cross inspector, and the imported inventory parser. Extra or omitted names,
nonregular files, or changed bytes fail. Keep the manifest private with the
review record; calculating hashes alone is not an independent review.

The launcher/operator must verify that exact reviewed checkout and manifest
**before starting Python**. Local imports happen before the in-process hash
checks, which detect drift in already trusted source; they cannot establish
trust in hostile code that has begun executing under the same UID. System
Python, Git, systemctl, Nix and its runtime libraries remain trusted host
prerequisites. This is bounded build orchestration, not a newly qualified
adversarial security boundary.

The producer performs these steps:

1. Observe the existing cgroup, service, process census and capacity before
   executing source/tool checks. Require a new evidence directory and retain
   the supplied manifest and cross plan without overwriting prior attempts.
2. Validate the frozen plan's scope, exact candidate/baseline identities,
   helper hashes, native/SDK binding, preserved recipe fields, successful
   dry-run checksum, and emitted command bounds. Receipt command strings are
   compared with commands constructed by the reviewed helper; they are never
   executed as arbitrary supplied commands.
3. Run the unchanged staged-libraries planner in an isolated Python child.
   This repeats the full native/TAPI/SDK receipt validation, immutable source
   and lock checks, live NAR/file/loader inspection, and offline evaluation.
   Compare its fresh candidate, recipe proof, source and content identities to
   the frozen plan. Raw registration serialization and loader ASLR addresses
   are not substituted for content identity.
4. With `--realize`, build only the exact candidate `drv^out`, using the direct
   local store, one job, one core, empty remote builders, preserved failed
   outputs, `--offline`, and explicit `--option substitute false`. The new
   output link and Nix's JSON result must identify that candidate exactly.
5. Run the unchanged cross inspector in a separately monitored Python child.
   Its successful terminal inspection must name the exact realized candidate.
   Keep its raw sidecars, wrapper/producer inventory, and normal store/trust
   verification results. Do not disable integrity or signature checks.
6. Recheck source, helper bytes, reviewed Nix executable, receipt checksums,
   service/process boundary and resources before publishing success.

Nix's pinned
[`--offline` implementation](https://github.com/NixOS/nix/blob/2.28.3/src/nix/main.cc)
does not override an explicitly configured substitution setting, and its
[local builder implementation](https://github.com/NixOS/nix/blob/2.28.3/src/libstore/unix/build/local-derivation-goal.cc)
permits network access for some fixed-output builders. Therefore the producer
requires the isolated service network and explicitly disables substitution in
addition to `--offline`. Missing local inputs must fail within the existing
bounds; there is no online retry or automatic budget increase.

The executable must resolve to a regular executable named `nix`; child `PATH`
must resolve that name to the exact checksummed binary. The selected version is
Nix 2.28.3. Python children use `-I -B` and one controlled source directory.
Ambient Python/loader overrides and Git/Nix user configuration are excluded.
The reviewed host executable, system Nix configuration and OS remain part of
the host trust boundary; these checks do not establish hostile-host isolation.

## Capacity, failure and evidence

Initial free disk must be at least 8 GiB plus a 128 MiB growth allowance on both
the store and evidence filesystems. Each keeps an 8 GiB reserve during the
operation; net free-byte reduction must stay within 128 MiB. Initial/live free
inode minima are 100,000/50,000, and host available-memory minima are 2 GiB/1 GiB.
The monitor samples these quantities and checks the cgroup's limits and OOM
counters. Net filesystem change is a conservative operational guard, not a
measurement of this process's exclusive allocation or a strict peak-space
proof.

Phase limits are 300 seconds for fresh planning, 180 for realization, and 180
for output inspection, within the producer's 600-second checked overall
budget and the service's separately observed hard runtime limit. Each phase
retains at most 20 MiB of combined stdout/stderr. Failed, timed-out, interrupted,
over-budget, changed-input and failed-inspection results remain failures. There
is no retry, store repair, garbage collection or evidence-directory reuse.

Each phase records the constructed argv, integer exit status or missing exit,
timeout/output-limit state, timestamps and stream checksums. Build success and
inspection success remain separate fields. The terminal receipt is first
written, flushed, synced and closed under `result.json.partial`; an atomic
exclusive hard link then installs `result.json`. A prepublication write or
close failure cannot leave a terminal success file. The partial name remains
for recovery. Missing terminal evidence, including abrupt service termination,
is incomplete evidence and must never be promoted to success.

Even a future successful run proves neither wrapper invocation nor effective
compiler target/SDK use. Compiler-rt, libSystem, executable fixture linking,
guest boot/runtime and security enforcement remain separate unqualified gates.
No policy mode or backend capability is changed by this producer.

## Focused validation

```sh
python3 -m unittest discover -s scripts/tests \
    -p 'test_build_puredarwin_cross_toolchain.py' -v
```

Tests use synthetic receipt/plan/inspection results and actual small owned
Python child processes. They cover provenance/argv drift, missing hashes,
metadata/build distinctions, resource and containing-service rejection,
source/helper changes, nonzero exits, timeout, output bounds, a detached child,
SIGTERM cleanup, failed inspection, evidence-directory reuse, incorrect Nix
resolution, environment isolation and receipt-publication failure. They do
not execute Nix, create a service, access AWS or qualify a real cross-toolchain.
