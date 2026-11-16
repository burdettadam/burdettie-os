# M1 thread-specific state and lifecycle probe

This source-owned fixture adds executable coverage for POSIX thread-specific
state, synchronization and cleanup. A Linux host reference is not a Darwin
result. A macOS-host Burdettie runtime pass is not a selected PureDarwin guest
result. No result here qualifies OS boot, persistence or security enforcement.

## Operation and exact acceptance

[`probe.c`](../../fixtures/m1_thread_state_probe/probe.c) starts three workers
in each of two batches. Each batch creates a new key, mutex and condition
variable. The main thread and workers store different context pointers under
the same key. All workers must initially observe a NULL value, retain their own
value across 32 synchronized rounds, and have their destructor invoked once
when they exit. The destructor checks that its key has already been cleared.
The parent key is cleared explicitly before deletion; it must not run a
thread-exit destructor during this operation.

A single `pthread_once` control spans both batches. Every worker invokes it;
exactly one initialization is accepted. Each successful join must return that
worker's exact context pointer. Worker-owned results are read only after its
successful join. Partial thread-creation failures wake and join every worker
that actually started, then report failure rather than treating less work as
success. An unrecoverable synchronization/join error aborts instead of returning
with live callbacks that could still access the image's state.

During each round, the parent compares the errno storage addresses of all live
workers with each other and its own. The workers publish these addresses under
the mutex before announcing arrival. Foreign errno storage is never read or
written. Local volatile write/read checks have no intervening library calls;
this does not assume successful library calls preserve errno. This is POSIX
key-specific data and errno coverage, **not** language-level Mach-O TLV,
`__thread`, general C++ thread-local constructors, cancellation or signal safety.

| Report field | Required value |
| --- | ---: |
| Workers requested, started, joined | 6 each |
| Completed worker rounds | 192 |
| Key-state checks (`tls_checks`) | 280 |
| Local errno write/read/address-stability checks | 256 |
| Pairwise live errno-address checks | 384 |
| Once initializer calls | 1 |
| Worker destructors and join-return checks | 6 each |
| Pthread errors and failures | 0 each |

The fixture checks these totals itself, prints one JSON object and returns zero
only on success. Output errors are nonzero. Default-clock timed waits share a
two-second absolute deadline per batch; predicate loops tolerate spurious
wakeups. A scheduler/resource failure is a failed observation, never a pass.
There is no skip, shortened-count or fault-injection mode in the shipped source.

## Host reference and retained-report validation

Use an already trusted host compiler and a new output location:

```sh
cc -std=c11 -O2 -Wall -Wextra -Werror -pthread \
  fixtures/m1_thread_state_probe/probe.c -o "$NEW_PROBE"
"$NEW_PROBE" > "$NEW_REPORT" 2> "$NEW_STDERR"
observed_exit=$?
python3 scripts/validate_m1_thread_state.py \
  --report "$NEW_REPORT" --exit-code "$observed_exit" \
  --platform linux --architecture x86_64
```

Capture the actual exit even with shell error-exit enabled. The example assumes
new caller-controlled paths and does not create a shared-host containment
boundary. Keep an outer timeout around execution, as the CI reference does.
The validator rejects duplicate JSON keys, unexpected fields, wrong target tags,
wrong/boolean counts, incomplete work, nonzero exits and contradictory status.
A supplied exit code and matching JSON are a consistency check, not authenticated
execution. Platform/architecture tags describe compilation, not proof of the
host/guest that ran the file.

The existing Ubuntu M1 job compiles and executes this reference and retains its
report, stderr, actual exit and source/binary hashes in the existing artifact.
No runner, job, permission or job-timeout increase is required.

## Native runtime integration

[`m1_thread_state_tests.rs`](../../crates/macho-runtime/tests/m1_thread_state_tests.rs)
is a required **macOS-host** integration test in the existing workspace test
job. It compiles this exact source with the already installed Xcode toolchain,
then executes the resulting Mach-O through `Runtime::load_and_run`, not by
launching it directly as a host application. Compilation, missing source,
load failures, a nonzero fixture exit, a signal and an invalid report all fail.
The parent checks the actual child process status and all report counters.

The new fixture deliberately uses `probe.c` and is compiled within that dedicated
test rather than the generic `main.c` builder/scorecards. No existing fixture is
removed or excluded. A dedicated child has an eight-second alarm because faulty
thread callbacks, joins and loader behavior can be fatal or hang. This is a
trusted test watchdog, not hostile-process containment or descendant supervision.
The child keeps its new temporary build directory for failure diagnosis.

```sh
cargo test --locked -p macho-runtime --test m1_thread_state_tests -- --nocapture
```

Only the host-native slice is exercised. This test does not run on non-macOS
hosts and does not establish Linux interpreted-pthread support, selected guest
ABI support, an independently qualified SDK or selected libSystem provenance.

## Negative controls and remaining gate

The Python suite compiles separate source-owned negative executables. It tests
no-op key assignment, no-op once initialization, missing destructors, total and
partial creation failure, wrong join returns, key-allocation failure, timed-wait
errors and deliberately shared errno storage. The shared-errno control retains
mutex protection around every write; it does not induce a C data race. Each
control must actually return exit 1 with failed counters, and every started
worker must have been joined. A compiler error, signal or external timeout is
not accepted as one of these expected negative results.

```sh
python3 -m unittest discover -s scripts/tests -p 'test_m1_thread_state_probe.py' -v
```

The next selected-target gate is to build this source against verified selected
compiler-rt/libSystem outputs and run it in the selected guest, recording the
actual process result, image identity, provenance and target architecture. The
host/native-reference lanes do not close that gate. The fixture does not alter
any policy or confer trust on pthread providers.

Interface references: Apple's archived
[pthread-key documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_key_create.3.html),
[condition-variable documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_cond_wait.3.html),
and the POSIX
[join contract](https://www.man7.org/linux/man-pages/man3/pthread_join.3p.html).
These describe interface semantics, not observed implementation qualification.
