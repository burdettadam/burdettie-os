# Runtime process exit and launch-owned cleanup

The direct `Runtime::load_and_run` and `Runtime::load_and_run_interpreted`
pipelines now create a cleanup owner for their fixed set of mapped images before
guest initialization. Registrations through their lifecycle shims belong to that
launch. Normal return and normal exit drain eligible callbacks before those
images are unmapped. This is a limited runtime ABI implementation, not the
managed-worker isolation required by M1.

## Implemented paths

| Route and operation | Behavior and limit |
| --- | --- |
| Direct native pipeline, return from `main` | Drains launch-owned callbacks in reverse registration order, flushes shared host stdio, then releases mappings and returns the result |
| Native C `exit` (`_exit` Mach-O symbol) | Drains the active native launch, flushes shared host stdio, and terminates the embedding process; supervisor survival is not implemented |
| Native C `_exit` and `_Exit` (`__exit`/`__Exit` Mach-O symbols) | Terminate immediately without callbacks or a stdio flush; the existing `_Exit` table alias has the same immediate behavior |
| Direct interpreted pipeline, normal return or C `exit` | Runs callbacks through the same ARM64 interpreter while its stack and image mappings remain alive, then flushes shared host stdio and returns the guest status |
| Interpreted `_exit`, `_Exit`, or `abort` | Discards remaining callbacks and returns the guest status without flushing; the host survives (`abort` retains the existing shim status of 1) |
| Interpreted entry/callback fault or callback nesting limit | Discards remaining registrations and propagates failure; it does not report successful cleanup |
| Raw interpreted SVC instruction | Reports an unimplemented-instruction failure, without cleanup or flushing; no kernel syscall backend exists |
| C `atexit` and C++ `__cxa_atexit` through lifecycle shims | Require an active owner; retain callback ABI, address, argument, DSO and owning image identities; reject unsupported ownership or code ranges |
| C++ `__cxa_finalize(dso)` | Finalizes only matching entries, once, in reverse order; a null DSO selects all pending C/C++ entries |

The source paths are the [launch registry](../crates/libsystem-shims/src/exit_registry.rs),
[lifecycle shims](../crates/libsystem-shims/src/lifecycle.rs),
[runtime lifecycle integration](../crates/macho-runtime/src/exit_lifecycle.rs),
[native pipeline](../crates/macho-runtime/src/pipeline_native.rs), and
[interpreted pipeline](../crates/macho-runtime/src/pipeline_interp.rs).

Both actual shim addresses and available host libc fallback addresses are
intercepted by the interpreter. This includes callback registration and DSO
finalization as well as termination. A bare interpreter bridge with no launch
owner rejects registration; it cannot put a guest callback in host libc's
process-global atexit list. Host lookup uses `RTLD_DEFAULT`.

## Ownership, reentrance and mapping lifetime

The launch guard borrows the complete `ImageRegistry` and is thread-bound. The
runtime checks each image's architecture against the native host or the ARM64
interpreter route. Callback registration requires an entry address in an
executable segment of this launch. A nonzero DSO must belong to one of its mapped
segments. The callback function and the DSO may belong to different images.
Arguments are retained exactly; the registering program must keep the object
being destroyed valid until its callback completes.

There is no process-global callback queue. Nested launches use different
registries, and only the innermost scope on its owner thread can register or
select callbacks. A bridge may retain registry metadata, but dropping the guard
closes that registry and clears its queue before the image owner is released.
The guard's destructor never executes guest code. Rust error returns and
unwinding therefore discard callbacks instead of calling them after unmapping.

Each callback is removed from the pending queue before invocation, and the
registry mutex is released before any guest instruction runs. A recursive
finalizer cannot select an entry already in progress. New registrations during
cleanup join the same queue and are selected in reverse order. Repeated DSO
finalization does not repeat completed entries. This follows the ordering,
argument and DSO semantics in the
[Itanium C++ ABI DSO destruction API](https://itanium-cxx-abi.github.io/cxx-abi/abi.html#dso-dtor-runtime-api);
it does not establish full Darwin C++ ABI compatibility.

Interpreted finalizers use a deferred FFI operation that receives the suspended
interpreter after the bridge borrow has ended. The callback uses that
interpreter's existing stack and memory; its CPU registers are saved and
restored around the subcall, but an exclusive-load reservation is cleared rather
than resurrected. An ARM64 callback is never cast to a native function
pointer. Subcalls bypass the launch completion hook, so an ordinary callback
return cannot finish the launch recursively. Reentrant callback nesting is
limited to 64; exceeding the limit is an execution failure, not successful
cleanup or a qualified security resource limit.

The main interpreter loop, normal-exit callback drain and nested FFI finalizers
share one instruction counter for each top-level `run()`. A callback cannot
reset that counter by returning and registering itself again. The configured
budget applies inside basic blocks as well; a compiled JIT block runs only when
the whole block fits. Each separate top-level interpreter run starts its own
budget. This does not preempt native host FFI calls, enforce wall-clock limits,
or limit native callbacks, which remain non-preemptible in the embedding host.
A worker/supervisor resource boundary is still required for enforced operation.

A callback that requests normal exit abandons its continuation, replaces the
exit status, and permits the remaining callbacks to drain. A callback that
requests immediate exit stops the drain and bypasses flushing. Interpreted
`+load` methods share the launch registry: ordinary method returns retain it for
`main`; explicit exit stops the launch, and a fault now propagates instead of
being ignored. The final main completion hook runs before
`call_entry_interpreted` releases the interpreter stack. Native main completion
runs before the borrowed image registry drops.

## Remaining limitations

- Native `exit`, `_exit`, `_Exit`, aborts and fatal signals can still terminate
  the embedding host. A supervised worker with an owned image/stream/resource
  lifetime is required to preserve a session or supervisor.
- Guest pthreads and other workers do not inherit this launch owner. Their
  lifecycle-shim registration fails without a scope; this is not cross-thread
  finalization, a thread shutdown protocol or descendant containment.
- The typestate loader and standalone `macho_exec::call_entry*` callers do not
  create these guards automatically. Registration through lifecycle shims on
  those unowned routes fails. They are not cleanup-qualified by this change.
- Explicitly disabling lifecycle shims in the native configuration can expose
  host libc registration instead. That configuration is outside this cleanup
  contract. Guest-loaded libraries that implement their own lifecycle are also
  outside the shim-owned queue; no claim is made about their termination path.
- Streams are still shared with the host. Normal cleanup uses the existing C
  and Rust stdio flush operations, including `fflush(NULL)`; it does not isolate
  per-launch streams, descriptors or concurrent host output.
- `atexit_b`, thread-local destructors, exception unwinding, dynamic DSO unload,
  and arbitrary host-library callbacks are not covered. The scope deliberately
  keeps a fixed image set alive; DSO filtering does not implement `dlclose`.
- The interpreter still has a permissive host-call/memory fallback, and native
  execution remains in-process. Image ownership checks here are cleanup
  bookkeeping, not security confinement or an enforced-mode backend.

The previous raw-SVC behavior treated every syscall as `exit(x0)`. That would
incorrectly authorize normal cleanup after an unsupported kernel operation.
SVC now propagates an explicit failure, including when issued by a finalizer.
This does not implement the Darwin or Linux raw syscall ABI.

## Regression evidence and practical scope

[Runtime lifecycle fixtures](../crates/macho-runtime/tests/runtime_exit_lifecycle_tests.rs)
construct their Mach-O bytes from source-owned AArch64/x86_64 instructions and
run the public loading pipelines in child processes. They need no Apple SDK,
proprietary binary, prebuilt fixture or physical Mac. The callback instructions
reside in the actual mapped Mach-O image; the test observes their effects only
after the runtime returns and releases that image. On x86_64 the native-return
case executes native x86_64 callbacks, and the interpreted cases execute ARM64
callbacks through the real decoder/FFI path. An AArch64 native-return variant is
provided for the platform CI; local x86_64 results do not qualify it.

The cases cover normal return/exit, mixed C/C++ registration order, exact C++
arguments in the live interpreter stack, DSO-selective repeated finalization,
recursive finalization and exit, registration during cleanup, immediate exit
from main or a callback, abort, callback/entry faults, callback nesting limits,
raw unsupported syscalls in main or a callback, and a fresh launch after failure.
Buffered C streams are inspected before the
host test process exits, so a later host flush cannot hide a failed immediate-
exit assertion. Child isolation makes accidental host termination an ordinary
test failure.

The lifecycle and registry unit tests additionally cover native explicit and
recursive exit, rejection of unowned/foreign/thread-mismatched registrations,
image/DSO identity retention, route mismatch, nested launches, abandoned
registries and native reentrant finalization without holding the queue lock.
Two small-budget child cases prove that sequential self-registration and nested
FFI finalization terminate at the shared instruction limit. A child watchdog
keeps a budget-reset regression from hanging the test suite. Decoded LDXR,
callback CLREX and STXR instructions verify that cleanup cannot restore an
invalidated exclusive reservation; a hot-loop test checks the budget boundary.
The earlier actual-shim/host-exit address regression remains in place.

These tests do not boot a PureDarwin image, link the selected M1 fixture SDK,
execute this runtime in the selected Darwin guest, qualify macOS application
compatibility, or prove an enforcement backend. M1 remains incomplete until its
selected-target build, boot, runtime and security evidence gates are met.
