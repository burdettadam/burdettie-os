# M1 host-runtime notification-center isolation

This batch corrects receiver isolation in the local notification shim. It is
host-runtime work, not selected PureDarwin guest or security qualification.

## Defect and correction

The previous registry stored observer/name/sender filters, but discarded the
`NSNotificationCenter` receiver on registration, posting and removal. Therefore
independent custom centers and the default center shared subscriptions; a post
could reach a different center's observers and a removal could cancel them.

Each local registration now retains the receiver's pointer identity. Snapshot
selection and both removal forms require that identity before matching names,
senders or observers. Callback delivery still occurs outside the registry lock;
active markers still cancel removed pending snapshots. Global runtime reset
still invalidates and removes registrations from all centers.

Existing internal direct-shim calls use receiver zero as shorthand for the
default center. That bridge convention remains; it is not a claim that sending
an Objective-C message to `nil` creates or uses a center. Nonzero custom receiver
identities are never collapsed into the default center.

## Regression coverage

All 19 previous local and 16 distributed notification test cases remain. Twelve
new local cases cover independent/default centers, `postNotification:`, selector
and block removal, wildcard subscriptions, wrong-center callback removals,
reposting and registration across centers, global reset, and three concurrent
posting workers with 64 deliveries each. Their counts are exact: an empty or
partial delivery loop cannot pass. Distributed code changes in this batch only
apply the formatting required by the pinned Rust toolchain.

The source-owned Objective-C fixture in
`fixtures/m1_notification_center_probe/probe.m` exercises public Foundation
methods with two custom centers and the default center. It performs 31 top-level
checks and requires 5 shared-selector, 1 default, 3 filtered, 1 reentrant,
2 foreign-center and 2 block deliveries, plus 9 notification-field checks.
The `--collapse-centers` negative control aliases the two custom centers and
must produce an actual exit 1 with nonzero failures, not an invented failure
receipt.

The macOS-only `m1_notification_center_tests` integration test compiles that
same source with the installed SDK, runs the native Foundation reference and
negative control, then executes the unmodified binary through
`Runtime::load_and_run` in an isolated child. It requires exact positive report
fields/counters and equality between the Foundation and runtime reports.
A compiler error, missing report, process crash, timeout, duplicate report,
incorrect exit status or mismatched counter fails the test.

Commands run in their own process groups. The compiler has a 15-second deadline;
reference/runtime processes have 8-second deadlines. Logs use exclusively
created files, a sampled 1-MiB-per-stream bound and retained failure evidence.
On a timeout or harness panic, the unreaped process group is killed and reaped.
These are trusted source-owned smoke-test limits, not an adversarial sandbox or
a substitute for the selected shared-host build containment requirements.
No existing CI job, runner, permissions, timeout or scanner policy is changed.

## Validation boundaries

Run with the repository's pinned Rust toolchain:

```sh
cargo fmt --all -- --check
cargo test -p cf-lite --lib nsnotificationcenter::tests
cargo test -p cf-lite --lib nsdistributednotificationcenter::tests
cargo test -p macho-runtime --test m1_notification_center_tests -- --nocapture
```

The native integration file is macOS-only. An empty Linux test binary is not a
native execution pass. Local Clang checks using minimal declaration stubs are
syntax checks only, not a real SDK compilation or Foundation/runtime execution.
Consult exact-head CI logs for observed build/test results; the existence of a
test or this guide is not a passing receipt.

## Remaining limitations

Center pointers are borrowed identities, not retained or validated capabilities.
Callers must remove registrations before releasing custom centers. This does
not implement block copying, automatic observer/center lifetime management,
queues, asynchronous dispatch, distributed IPC, or waiting for an already
started callback during concurrent removal. The source fixture keeps all
objects/blocks valid through its synchronous operations and makes no ownership
or leak-freedom claim. Selected guest boot, persistence, ABI qualification and
security enforcement remain independent open gates.
