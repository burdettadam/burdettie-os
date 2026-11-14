# Runtime process-exit scope and remaining cleanup work

The runtime currently embeds guest execution in its host process. This is not
the managed-worker isolation described by the M1 security contract. The fixes
here separate immediate exit from normal-exit output handling and prevent an
interpreted guest's explicit exit from terminating that embedding host.

## Current behavior

| Route and operation | Behavior and limit |
| --- | --- |
| Native C `exit` (`_exit` Mach-O symbol) | Flushes C/Rust stdio and terminates the embedding process; guest cleanup callbacks are still unsupported |
| Native C `_exit` and `_Exit` (`__exit`/`__Exit` Mach-O symbols) | Terminate immediately without flushing stdio or invoking cleanup; the existing `_Exit` table alias has the same immediate behavior |
| Interpreted explicit exit | Both actual lifecycle shim addresses and host libc exit addresses return the guest status through `FfiResult::Exit`; the embedding process remains alive |
| Return from guest `main` | Returns a runtime result, but does not yet drain a launch-owned callback registry |
| C `atexit` | Records a function in a process-global registry; the runtime does not automatically invoke it |
| C++ `__cxa_atexit` | Callback arguments and DSO ownership are not retained; successful registration is not cleanup qualification |

The source paths are [lifecycle shims](../crates/libsystem-shims/src/lifecycle.rs),
[interpreter bridge](../crates/macho-runtime/src/interp_bridge.rs), and the
[native entry pipeline](../crates/macho-runtime/src/pipeline_native.rs). The
interpreter previously registered only host libc addresses. A symbol resolved to
the lifecycle shim could instead reach `auto_call_host` and call the shim's real
process-exit operation. Registering the actual shim addresses closes that route.
Host lookup uses the platform's `RTLD_DEFAULT` handle instead of assuming that
the global-search handle is null on every Unix target.

## Why a global callback drain is unsafe

Adding `run_atexit_handlers()` to every exit is insufficient. Callbacks can
remain registered after a previous guest's mappings are dropped. The cached
runtime context does not create a new callback registry for every execution.
An interpreted ARM64 callback also cannot be invoked as a native x86_64 function
pointer. Concurrent or nested executions would share the current global state.

The next cleanup implementation must establish the following ownership and
ordering before enabling callbacks:

1. Create a launch-owned registry before any guest constructor, retaining the
   callback's execution route, argument, DSO and mapped-image lifetime. Guest
   threads must register with that same launch without acquiring another
   launch's authority or callbacks.
2. Drain normal-exit callbacks once, in the required reverse registration order,
   while the owning images are still mapped. Main return and explicit normal
   exit must enter the same protocol. Immediate exit must bypass it.
3. Remove a callback from the pending queue before invoking it and release the
   registry lock around guest execution. Define nested registration, recursive
   finalization and concurrent shutdown so callbacks cannot deadlock or run
   twice. Support the actual C++ callback argument and DSO finalization rules.
4. Invoke interpreted callbacks through the interpreter and native callbacks
   only through their valid native route. Flush the launch's streams after
   normal cleanup, then release mapped images and owned resources.
5. Use a managed worker for process termination and preserve supervisor/session
   survival. Do not use unchecked unwinding through C/Rust frames to turn the
   existing native non-returning exit API into an in-process return.

## Regression evidence

The lifecycle test launches child processes with an explicitly buffered C stream
and checks all exit aliases: normal exit flushes the marker, immediate exit
preserves the requested status without flushing it. The interpreter test runs
a source-owned AArch64 `BLR X16` through the real decoder and FFI path for each
shim/host exit address, checks the returned status, and verifies that its host
survived. Each case runs in a child test process so a regression produces an
ordinary test failure instead of terminating the parent test suite.

These tests do not execute a PureDarwin image, qualify the x86_64 Mach-O runtime
inside that guest, or prove normal-exit cleanup, guest confinement or M1
completion.
