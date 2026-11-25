# In-process notification stability batch

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

This change improves the existing Foundation reference implementation. It does
not qualify selected PureDarwin libraries, guest boot, cross-process distributed
notifications, owner-selectable confinement, or completion of M1.

## Problems addressed

The retained Build & Test log for run `43985693657` includes a failed
`nsdistributednotificationcenter::tests::remove_observer_by_name` test followed
by a poisoned-registry panic in `post_with_no_observers_does_not_panic` at
`crates/cf-lite/src/nsdistributednotificationcenter.rs:70`. The distributed tests
reset the same global registry without a shared test guard. An assertion while
holding its mutex can poison the mutex and a later extern-C entry can abort.
The earlier recovery comment's `windows::pasteboard` diagnosis is not the
supported source location and must not be used as a reproduction recipe.

Source inspection also identified production issues in both notification
centers: callbacks ran under the registry mutex; local removal guessed that an
object was a block token by reading its first ivar; sender filters were ignored;
and the distributed CF bridge treated a C callback address as an ObjC block.

## Behavior after this change

Both centers snapshot matching registrations, release the mutex, and only then
invoke callbacks. Each registration has its own shared active marker. Removal
or reset invalidates pending snapshots; newly registered observers wait for a
later post. A reset cannot make an old snapshot deliver a new registration.
Callbacks can remove themselves or post recursively without reacquiring a mutex
that the dispatcher still holds. This is not a guarantee that removal waits for
callbacks already executing concurrently on another thread.

Local block observers are removed by their exact registered sentinel identity,
not guessed object contents or a reused numeric token. A failed sentinel
allocation does not install an unremovable observer. Nil callbacks and invalid
nil selector registrations do not enter the registry. Named removal handles
block observers and applies the supplied sender criterion. A specific name does
not remove an unrelated wildcard registration.

Local sender matching uses object identity. Distributed selector sender matching
uses string contents. The narrow existing CF registration helper does not have
a sender-filter parameter; that limitation is unchanged. Its callback now uses
the five-argument C ABI: center, observer context, name, object, and userInfo.
It is no longer dereferenced as if it contained a block invoke pointer.

The implementation remains synchronous and in-process. Queue dispatch, block
copying/retention, cross-process transport, suspension and session options are
not implemented by this batch. Guest observers, blocks and callback contexts
must remain live during delivery. The change does not promise automatic
observer lifetime management or treat arbitrary guest pointers as safe inputs.

## Regression coverage and execution boundaries

The two Rust test modules retain all 17 previous test names and add 18 tests,
for 35 tests in these modules. Registry-mutating tests use a module-specific
RAII guard and inspect copied state outside the production mutex. Actual owned
block/C callbacks test arguments, recursive posting, self-removal, pending
cancellation, reset/re-registration, sender filtering, exact handles, stale
handles and wildcard removal. Callbacks record failures rather than panicking
across the C ABI. Their try-lock check makes a lock-held regression fail without
hanging the test process.

The development container has no Rust toolchain. Rust compilation, formatting,
Clippy, native tests and the full workspace remain required CI gates; source
inspection or the Python checks below do not substitute for them.

The repository-hygiene workflow still scans all reachable history with the same
pinned scanner, checksum, configuration, redaction and failure policy. It now
writes its report to a unique temporary directory, prints only validated rule,
commit, file and line metadata, and deletes the temporary report on exit. No
raw report is uploaded. Scanner failure codes are preserved; a missing/invalid
report also fails the step. No scanner rule, exception, baseline, ignored
finding, history rewrite, timeout or runner configuration is changed.

Local validation: **20 Python tests passed**, including seven executions of the
actual workflow shell with controlled scanner substitutes. They check clean,
findings, error, malformed-report and missing-report paths, exact exit status,
cleanup, bounds, duplicate-key rejection and omission of secret-bearing fields.
These are diagnostics tests, not a real Gitleaks scan or a claim that the two
previous history findings are resolved.

## References

- [CF callback ABI](https://developer.apple.com/documentation/corefoundation/cfnotificationcallback?language=objc)
- [Local named observer removal](https://developer.apple.com/documentation/foundation/notificationcenter/removeobserver(_:name:object:))
- [Distributed named observer removal](https://developer.apple.com/documentation/foundation/distributednotificationcenter/removeobserver(_:name:object:))
- [Gitleaks report options](https://github.com/gitleaks/gitleaks)

Exact-head CI results and any follow-up corrections are recorded in PR #3.
