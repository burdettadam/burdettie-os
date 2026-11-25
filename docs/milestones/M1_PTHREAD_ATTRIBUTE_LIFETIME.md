# M1 pthread attribute lifetime

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

## Defect and correction

At public baseline `6257197c7aa9121910856c8bb3b1b9e9e68d7027`, the non-macOS
`shim_pthread_create` removed the supplied attribute handle from its registry,
then destroyed its host attribute object after a single creation attempt. This
also happened on a native creation failure. Later creates with the same handle
silently used default attributes; getters and setters no longer found the object.

Creation now borrows the registered opaque host object under the existing
registry mutex. The guard remains held only through `libc::pthread_create`,
then is explicitly released. Only the existing attribute destroy/reinitialize
operations retire that object. No opaque `pthread_attr_t` bytes are copied.
The start-record allocation occurs after handle validation, and the original
failure cleanup remains in place.

Null attributes still request host defaults. An unknown non-null handle now
returns `EINVAL`, consistently with the attribute getters/setters, instead of
silently discarding settings. This is a defensive shim behavior, not a claim
that POSIX defines use of uninitialized or destroyed attribute objects. The
native macOS forwarding path is unchanged.

## Regression scope

`crates/libsystem-shims/src/pthread/attribute_lifetime_tests.rs` compiles as a
child of the actual production shim on Linux. Its seven GNU/Linux tests cover:

1. Repeated creation and changes to the same attribute object, with exact saved
   settings and an observation of the live child stack.
2. Four concurrent creators sharing one initialized handle.
3. Null-attribute default creation and exact joined return value.
4. Rejection of an unregistered non-null handle before the callback runs.
5. Destroyed-handle rejection followed by reinitialization and successful reuse.
6. Start-routine rejection without consuming the attribute object.
7. A real native creation failure and successful retry using the same object.

Each started callback reenters the attribute getter, so a registry lock retained
while waiting for the child would prevent completion. All created joinable
threads are joined before their argument allocation is reclaimed. On an
unexpected join failure the test retains the allocation rather than freeing
memory that a child may still access. No callback assertion unwinds across C.

The final case uses a GNU/Linux zero-CPU affinity mask to induce native `EINVAL`.
It modifies only the test's host attribute object, not process scheduling,
resource limits, the parent's affinity, or a shared service. It restores a mask
queried from the calling thread before retrying. This is test setup for the
native failure path, not a newly exposed Darwin affinity API.

The live stack check permits a bounded larger cached allocation; it does not
require the requested minimum and physical host allocation to be identical.
The exact requested size must still be available through the registered getter.
These tests exercise host pthread behavior through the Rust shim, not a Mach-O
application, the ARM64 interpreter, or the selected PureDarwin guest.

## Running the checks

The existing Docker development job now runs the focused production-shim group
with the existing Rust 1.93.0 image and lockfile:

```sh
cargo test --locked -p libsystem-shims --lib \
  pthread::shims::attribute_lifetime_tests -- --nocapture
```

The Docker build bounds this compile/test operation at 180 seconds and requires
both exit zero and the seven-passed/zero-failed/zero-ignored test summary. No new
workflow, CI runner, permission, job timeout, or dependency pin is introduced.
An empty group, a timeout, or a failure cannot qualify the image.

## Evidence boundaries

The development container has no installed Rust compiler and cannot resolve
external package/Git hosts. Its C host control validates only the real native
failure setup and retry, not execution of the Rust shim. Current-head CI must
compile and execute the Rust tests before this increment is called validated.

The earlier thread-state CI on head `6257197` has now completed all seven jobs,
including the previously queued native Build & Test job. That is historical
host/runtime evidence, not validation of this new increment.

Selected compiler-rt/libSystem/dyld builds, native selected-image fixtures,
boot, persistence and all selected-guest security controls remain separate gates.
No AWS workload, retained disk, cache, service, or capacity setting is changed.

## API basis

The Linux man-pages project documents attribute reuse across one or more
pthread_create calls, explicit destruction, and the opacity of pthread_attr_t:

- https://man7.org/linux/man-pages/man3/pthread_attr_init.3.html
- https://man7.org/linux/man-pages/man3/pthread_create.3.html
- https://man7.org/linux/man-pages/man3/pthread_attr_setaffinity_np.3.html
