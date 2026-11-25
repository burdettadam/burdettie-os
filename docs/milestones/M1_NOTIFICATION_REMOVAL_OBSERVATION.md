# Native block-token removal characterization and runtime correction

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

## Retained native evidence

Diagnostic head `f0ac4612e31b67cfed38135f855f880584be3f74` ran in the existing
Build & Test job `133898326850`, workflow run `44041795322`. The runner was
macOS 26.4 (25E223), ARM64, with Xcode 26.3 (17C5123f) and Rust 1.93.0.
All thirteen independent Foundation cases reported `before: 1`, `after: 0`,
`after_cleanup: 0`, and `control_failures: 0`.

Each case used fresh owner/foreign centers, sender identities and a retained
block token. Before removal, foreign-center posts and mismatched name/sender
posts were checked not to deliver; the owning-center positive post had to
deliver once. Cases independently observed plain removal through owner,
foreign and default centers; exact, wrong-name, wrong-sender and nil-filter
named removal through owner/foreign centers; and specific-filter removal of
wildcard registrations. No named case reused a token removed by another case.

These native observations show cancellation by token identity even through a
different center or with mismatched supplied filters on that Foundation host.
They are not a claim about every Darwin release. Apple's public
`removeObserver(_:name:object:)` documentation describes name/sender matching;
this empirically observed block-token behavior must not be confused with that
general documented filtering rule or applied to selector observers.

The diagnostic retained the original 31-check gate. That gate still failed with
four assertions and zero block callbacks instead of two, so the diagnostic did
not hide the failure. The failure record remains available at the cited job.

## Production correction

The shim now recognizes only registered block-token identities for this
cancellation behavior. It never reads arbitrary observer memory to infer token
type. Selector removal still requires its receiving center and name/sender
filters. Delivery remains center-scoped for both observer kinds. Cancellation
invalidates the existing active marker, including pending shim delivery
snapshots; callback execution still occurs outside the registry mutex.

Four former unit-test expectations encoded the disproven block-token behavior.
They are corrected with positive delivery checks before removal. Additional
regressions verify that removing one token preserves other registrations of
the same block and that selector removal still rejects mismatched filters.
The callback-pending regression tests the shim's existing snapshot policy; the
native matrix does not establish Foundation's callback ordering semantics.

The main fixture keeps 31 checks and two required positive block deliveries.
Its block section now tests delivery filtering and owning-center removal;
foreign/mismatched token-removal coverage lives in the independent matrix.
A new real `--suppress-blocks` negative control must exit 1 with zero callbacks
and failed assertions. The existing collapsed-center negative remains required.
Accepting a silent callback loop is not a fix.

The independent observation source is byte-for-byte unchanged from f0ac461.
The integration gate now requires all thirteen exact named reports, six fields,
one positive callback, zero post-removal callbacks, zero cleanup callbacks and
zero control failures. Missing, duplicate, unknown and malformed evidence is
rejected. The same compiled observer runs under native Foundation and through
the Burdettie runtime, with exact report equality required. Two parser tests
cover a complete report set and eight corrupt/missing-evidence variants.

## Validation boundaries

Current-head native CI must establish execution of the correction and the
native/runtime comparisons. The earlier diagnostic proves only the reference
observations. The nine process-cleanup regressions, 15-second compiler and
8-second process deadlines, 1-MiB stream bounds and required workflow gates
remain intact. No CI definition, scanner exception, history rewrite or cloud
resource is changed. Historical full-history secret findings remain separate.

No local Rust compiler or macOS SDK is available in the recovery container.
Local source identity and whitespace checks are not substitutes for native CI.
This work does not qualify a selected PureDarwin library, guest image, boot,
persistence, asynchronous queues, block-copy ownership or security enforcement.
