# Host-runtime pasteboard snapshot consistency

This increment repairs two production snapshot defects and isolates tests that
share the process-global in-memory clipboard. It is not selected Darwin guest
or host clipboard-server qualification.

## Corrected behavior

The fallback writer previously derived the next change count from the calling
instance's cached generation. Two instances could publish duplicate counts;
an old instance could move the shared count backward. The shared-state mutex
now protects both snapshot publication and allocation of the next saturating
generation. Explicit writes advance the shared counter even for identical data.
Last-writer-wins snapshot contents remain the existing contract; this does not
merge arbitrary formats from stale snapshots.

The optional non-macOS arboard refresh previously read host text into the local
map and then immediately replaced that map with an older shared snapshot.
Successful text reads now update and return the shared snapshot. Empty text is
a valid value. Changed text invalidates formats belonging to the old snapshot;
identical text leaves the count and known formats unchanged. Read errors still
use the existing shared fallback rather than treating an unavailable backend as
an empty clipboard.

The native macOS clipboard implementation and its authoritative host change
counts are unchanged. This increment does not change host write-error reporting,
add clipboard-server capabilities, or fix the pre-existing optional arboard
clear-without-text behavior. A text-only observation cannot detect a host owner
change that reuses identical text or inspect unobserved host binary formats.
These limitations are not silently promoted to compatibility passes.

## Regression coverage

All six existing tests and assertions remain. Their reset/read sequences now
hold a dedicated test-only mutex, preventing unrelated parallel tests in this
module from resetting one another's state. Workspace concurrency is unchanged.

Thirteen additional regressions cover stale writes and clears, stable snapshots
until refresh, repeated equal writes, saturation at `u64::MAX`, observed host
text, empty text, identical-text refresh, stale binary-format invalidation,
invalid UTF-8, and local writes after observed host changes. Eight real threads
also perform 32 writes each; the sorted generations must be exactly 1 through
256. The test verifies the final shared generation and final contents, not only
that threads finished. The dedicated test mutex does not serialize these worker
threads.

Unit tests select the in-memory backend and never access a user's clipboard.
The host-text reducer is exercised directly with supplied text. This validates
the reducer and publication logic, not a live X11/Wayland clipboard round trip.

Run in the existing workspace/toolchain:

```sh
cargo test -p app-kit-lite --lib pasteboard
cargo fmt --all -- --check
```

Exact-head native CI must establish actual Rust execution. Local Python suites,
source review and a source-only patch check are separate evidence, not Rust
execution substitutes.

## Separate native notification reference failure

The preceding `f50b2fc` native job passed formatting, strict Clippy and all nine
new child-process cleanup regressions. Its notification reference executable
then exited 1 before Burdettie runtime comparison: all selector/field counts
matched, but four block-delivery assertions failed (`block_calls` was 0 rather
than 2). The earlier assumption that removing a returned block token through a
different Foundation center is a no-op is contradicted by that observed run.

The relevant evidence is required job `133876043867`, run `44034899714`.
This batch does not modify the notification fixture, expected counts, runtime
assertions or any failed-check handling. That reference-contract investigation
remains open; this clipboard correction must not be described as a passing full
CI or a solution to that separate failure.

No scanner exception, CI runner/job/permission/timeout change, cloud operation,
or selected-guest qualification is introduced.
