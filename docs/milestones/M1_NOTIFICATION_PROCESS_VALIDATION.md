# Notification qualification: process ownership and failure cleanup

This increment repairs the strict-Clippy build blocker in the native
notification integration test without suppressing the lint or weakening the
native fixture's assertions. It does not qualify a selected Darwin guest.

## Owned child lifecycle

The spawned `Child` is now a named binding transferred immediately into
`RunningProcess`, whose destructor terminates and reaps an unfinished child.
The constructor creates the process group itself, so every guard has the same
ownership invariant. There is no fallible operation between spawning the child
and installing the guard.

The existing `try_wait` loop is retained in `wait_bounded`. A completed child is
marked reaped; timeout, log-metadata failure, and assertion unwinding destroy
the guard. Cleanup signals the owned process group and also kills the owned
leader before waiting, covering a leader that changed its process group.
Existing log files remain exclusive and are never overwritten. Completed logs
are size-checked again before reading, including when a short-lived writer
exits between polling iterations.

The native compiler deadline remains 15 seconds; the Foundation reference,
negative control, and Burdettie runtime deadlines remain 8 seconds. The
1-MiB-per-stream bound, all 31 top-level fixture checks, callback counts,
reference/runtime equality, and real negative-control exit requirement are
unchanged. No CI runner, job, permissions, timeout, or secret-scanning rule is
changed.

## Nine added regressions

The macOS integration suite now checks actual source-owned processes for:

1. Destruction of a guard while its child is running.
2. Panic unwinding after guard installation.
3. A zero-deadline rejection of a live child.
4. Missing-log metadata while monitoring a live child.
5. A real exit 7 with retained stdout and stderr.
6. A real SIGTERM exit that cannot count as success.
7. Finite 2-MiB writers exceeding stdout and stderr bounds.
8. Existing evidence refusing both replacement and process launch.
9. Spawn failure preserving the newly created empty log files.

Reaping assertions use nonblocking `waitpid` and require `ECHILD`. A still-live
child is cleaned up before a failed assertion is reported. This tests OS-level
reaping instead of merely trusting a Boolean flag. The oversized-output test
uses a finite writer, not an unbounded disk-filling loop.

The native notification fixture source and all prior runtime behavior tests
remain unchanged. These added tests run within the existing required macOS
workspace test command. Compiling an empty macOS-only integration test on
Linux is not a native execution pass.

## Validation boundaries

Exact-head CI must establish Rust formatting, strict Clippy, execution of these
regressions, and the existing Foundation/runtime comparison. The local recovery
overlay's Python tests are a separate validation lane, not evidence that these
Rust tests executed. Retained local bundles remain additive recovery history;
they must not replace the public repository history.

This guard is a bounded trusted-tool test harness, not a hostile-process
sandbox. It does not prevent a hostile descendant from escaping its process
group or imposing a brief disk spike between polling intervals. It makes no
claim about selected toolchain provenance, selected libraries, guest boot,
persistence, or security enforcement.
