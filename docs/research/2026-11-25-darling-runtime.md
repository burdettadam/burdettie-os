# Darling over Linux: current launch modes and first runtime experiment

Research date: 2026-11-25. This record combines pinned source inspection,
executed upstream host tests and native fixture controls. **It does not report
a complete Darling build, Mach-O execution or application compatibility pass.**
The subsequent [AWS host probe](2026-11-25-aws-host.md) separates restricted
workspace failures from prerequisites available on a real Linux host.

## Decisions shaped by the research

1. Establish a complete upstream rootless runtime before writing its Rust
   supervisor adapter. Current upstream already implements unprivileged launch
   modes; a new Burdettie privileged launcher is not the starting requirement.
2. Start with the `core`/`system` component closure and a fixed x86_64 CLI corpus.
   Expand to Foundation and GUI workloads after the loader/library/server
   baseline works. A smaller component selection is not a measured build-size
   guarantee.
3. Keep one coherent Darwin ABI closure in a worker. Embedding only
   darlingserver into Burdettie's synthetic Mach/ObjC world remains a separate,
   later experiment justified by demonstrated benefit.
4. Treat rootless launch and application confinement as different properties.
   Compare explicit enforcement arrangements only after the upstream control
   works. See [ADR 0004](../adr/0004-linux-integration-boundaries.md).

## Inspected source identities

| Repository/component | Revision |
| --- | --- |
| Darling main | `90af44da21d314b9acd24de1d1272e4aba8b9cc6` |
| darlingserver submodule | `3604875f2d3fa46985e914650d23ce124f99ac27` |
| xnu submodule | `bca1777f5070e5c477780861cda2346511f03043` |
| objc4 submodule | `a68efe2ba2159e84e1b9a52b5b34f993567c17b2` |

The parent tree and its full pinned submodule closure, not this abbreviated
table alone, define an eventual runtime build. Relevant primary sources are
the [launcher](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/darling.c),
[mode selection](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/launcher-mode.c),
[server/namespace handling](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/src/startup/launcher-server.c),
[launcher test guide](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/tests/launcher/README.md)
and [component selection](https://github.com/darlinghq/darling/blob/90af44da21d314b9acd24de1d1272e4aba8b9cc6/cmake/darling_parse_components.cmake).

## The current launcher has three explicit modes

| Mode | Current source behavior | Burdettie implication |
| --- | --- | --- |
| Root/privileged | Existing privileged launch path; privileged invocation does not honor the unprivileged mode selectors | Not required for the first proposed control |
| Rootless | Selected with `DARLING_ROOTLESS=1`; uses user/mount/UTS namespaces and mapped UID 0; server handles its PID-namespace work | First complete-runtime control candidate |
| NonRoot | Selected with `DARLING_NONROOT=1`; uses a copied prefix without the rootless mode's private namespace setup | Candidate inside an independently qualified outer launcher |

NonRoot takes precedence when both unprivileged selectors are supplied. The
prototype should select exactly one mode and record it. Current source can
derive the installation prefix from the executable path for an unprivileged
installation. The normal installer still requests setuid on the launcher;
an explicitly non-setuid evaluation package is needed to exercise the intended
unprivileged route. No launcher was installed or host privilege changed here.

These source paths are newer than general documentation describing only a
setuid launcher and fixed compiled prefix. Preserve the exact source revision
when reproducing them; do not infer that an older packaged release has the
same behavior.

Rootless startup deliberately retains host SysV IPC for X11 interoperability,
and the prefix can expose host files through `/Volumes/SystemRoot`. Named
profiles and separate prefixes help organize state, but they do not implement
different Burdettie application grants. The whole server/prefix/process tree
belongs to the application/profile trust domain. A read-only lower image with
an unrestricted writable upper layer is not immutable code admission.
See the [enforcement investigation](2026-11-25-linux-enforcement.md).

## Executed upstream launcher tests

The reproducible runner is
[`scripts/research/darling_host_probe.py`](../../scripts/research/darling_host_probe.py).
It verifies the exact parent revision and unchanged selected upstream files,
builds eight unmodified upstream test programs in a temporary directory,
applies timeouts, and distinguishes skips from passes. The source checkout
must include `src/startup`, `tests/launcher` and
`src/shellspawn/shellspawn.h`.

```sh
python3 scripts/research/darling_host_probe.py \
  --source /path/to/pinned-darling \
  --output /path/to/private-results.json
```

Environment: Debian 13, x86_64, GCC `14.2.0-19`; reported kernel `6.18.57`.
All eight programs compiled. The final runner exited 1 because two programs
failed; no failure was converted to a pass or skip.

| Upstream program | Executed result |
| --- | --- |
| `ids` | Pass |
| `env` | Pass |
| `prefix` | Fail: five socket/bind/listen/path/connect assertions |
| `mode` | Pass |
| `rootless-ns` | Pass, without a reported skip |
| `server` | Fail: expected same-mount-namespace result was not obtained |
| `prefix-state` | Pass |
| `profile-prefix` | Pass |

Independent mechanism probes found `EPERM` when creating named Unix stream
and datagram sockets, and `EPERM` for `NS_GET_USERNS` on the current mount
namespace. Those restrictions are consistent with the two failures. They are
environment blockers to investigate, not established upstream defects. In the
same environment, `process_vm_readv` returned `EPERM` and
`PR_SET_PTRACER_ANY` returned `EINVAL`. A working socketpair or namespace
creation command does not compensate for these missing runtime operations.

The [real-host follow-up](2026-11-25-aws-host.md) passed the selected socket,
memory, namespace and Landlock prerequisites. It did not rerun these eight C
programs or run Darling. Thus the upstream test results above remain six
passes and two failures, with a narrower explanation for the research-host
limitation.

## A fixed first fixture corpus

The first corpus uses existing source-owned fixtures at the strategy baseline
`d264e02f379b8fd4c4205bc416ea060064ad59fd`. Thin x86_64 Mach-O binaries,
compiler/SDK/linker inputs and runtime closure must be recorded separately.
The existing fixture build script's macOS/ARM defaults must not silently choose
the architecture for this Linux x86_64 experiment.

| Fixture | Required behavior | Linux source-control result |
| --- | --- | --- |
| `return42` | Exit 42 | Native ELF control passed |
| `simple_cli` | Print `hello` and exit 0 | Native ELF control passed |
| `constructor_test` | Both constructors run before `main` and initialize state | Native ELF control passed |
| `pthread_basic` | Thread/join, mutex, TLS and related fixture assertions succeed | Native ELF control passed |
| `tlv_basic` | Print `counter=43 flag=1` and exit 0 | Native ELF control passed |

All five were compiled with the local GCC C11 workflow (`-O0`, with `-pthread`
for the pthread fixture) and executed successfully as native Linux ELF source
controls. These runs establish fixture behavior on that target only. They
do not establish Darwin ABI compatibility, Mach-O loading, a PureDarwin SDK
result or a Burdettie runtime result.

Source SHA-256 identities:

| Fixture source | SHA-256 |
| --- | --- |
| `return42/return42.c` | `cc57bd948c0c900a3da05b15cca7761258377936c883624bd8dee159c604813d` |
| `simple_cli/main.c` | `f85badd2007451bbda4791e7fe820b41be0a424172a567573511688bff975235` |
| `constructor_test/main.c` | `8e65d68cb2a306fa6fadd9e9d9e5dabe215103b768af70cfad3ac0cb1847dd44` |
| `pthread_basic/main.c` | `721df044e8c56ee0c503c5550bc61f264576b77156252d10086c5d97b29ab3df` |
| `tlv_basic/main.c` | `d5436e21a7a2c02cd5d433bf9b01527f00d13358002402c93bafc6d0660878b1` |

`nsdata_basics` is a next Foundation candidate; it was inspected but not built
or run in this investigation. Repair or exclude implementation-specific
expectations before using a fixture to reject upstream reuse. For example,
`nsstring_ops` currently requires a non-null direct C-string pointer;
[Apple's public CFString header](https://github.com/apple-oss-distributions/CF/blob/main/CFString.h)
allows `CFStringGetCStringPtr` to return null. The fallback copy API and public
string semantics are appropriate checks; a private storage representation is
not a portable acceptance criterion. No fixture was modified in this research.

## Component and artifact selection

The inspected component graph puts minimal loader/server/libSystem/launchd
work in `core`, with basic shell/system utilities in `system`. Broader CLI
development and GUI component sets add substantially more providers. Determine
the actual dependency closure for each next fixture instead of assuming that
the minimal closure contains Foundation or GUI support.

An official binary release was also inspected without installation/execution:

| Field | Inspected value |
| --- | --- |
| Release tag | `v0.1.20260608` |
| Release publication date | 2026-06-09 |
| Asset | `debs_20260608.zip`, 118,018,438 bytes |
| Asset SHA-256, checked against release API digest | `27469ef3932da2e91dd7fb34b70e3628a3e54b7af9fb5480051f44af35eca1fd` |
| Core package | `0.1.20260609~noble`, declared installed size 93,021 KiB |
| System package | Same version, declared installed size 31,525 KiB |

Source: [official release](https://github.com/darlinghq/darling/releases/tag/v0.1.20260608).
Package metadata is not measured peak disk usage. These older binaries cannot
qualify the current launcher/security source changes. Do not combine their
libraries with a newly built launcher/server and call that a pinned upstream
control without separately establishing compatibility of the combination.

The [general build guide](https://docs.darlinghq.org/build-instructions.html)
documents material source/build/storage requirements. A minimal build may use
less, but its actual peak remains unmeasured. The retained AWS host was already
below its protected disk reserve, so no current runtime download/build was
started there. The architectural question remains open for a suitable bounded
build environment; a capacity stop does not justify replacing upstream code.

## The next development gate

Record a complete current-source rootless control on the fixed five-fixture
corpus, then add the Rust supervisor with the same inputs. Compare behavior,
startup/steady resources, failure reporting and cleanup. Keep direct Darwin
traps and cross-ISA translation outside that result: Darling's documented
[library-based syscall emulation](https://docs.darlinghq.org/internals/basics/system-call-emulation.html)
does not cover software issuing raw inline Darwin system calls.

Follow with two domains carrying different grants, whole-tree cleanup,
server-mediated negative operations and immutable dependency admission. Do
not spend the first integration effort on a direct server embedding, a new
Mach port registry or a replacement Objective-C runtime unless this measured
comparison establishes that upstream integration cannot meet a named goal.
