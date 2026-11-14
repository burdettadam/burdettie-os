# Bounded Darwin ARM64 printf on x86_64 hosts

The ARM64 interpreter on Unix x86_64 hosts handles the exact resolved host
`printf` address with an explicit Darwin LP64 adapter. It supports literal
bytes, `%%`, `%d`, `%lu` and `%s`. It does not infer arbitrary foreign-function
signatures or construct a host `va_list` from guest bytes.

## Argument and output contract

Darwin ARM64 passes the format pointer in `x0` and printf's variadic arguments
in eight-byte stack slots. The adapter reads the actual interpreter SP, requires
16-byte alignment, and copies complete slots from registered memory only.
`%d` interprets the low 32 bits as a signed C int; padding in the upper half is
ignored. `%lu` reads a 64-bit unsigned long. `%s` reads a 64-bit pointer and then
copies bounded, NUL-terminated bytes. Neither format nor string bytes need to
be UTF-8.

The implementation bounds the format to 4,096 bytes, string content to 4,096
bytes, arguments to 64, and complete output to 65,536 bytes. A terminator must
be present within the permitted memory and length. All conversions complete
before output starts. Unsupported conversions, including `%n`, flags, widths,
precisions, positional parameters, other integer modifiers and floating-point
formats, stop the interpreter with an explicit `FfiRejected` reason. Rejected
calls never fall through to generic host invocation or print a partial result.

Successful output uses a typed host printf call with a constant `"%s"` format
and one owned C string. The guest format is never supplied to host libc. This
preserves existing C stdout buffering and its byte-count/error return; it does
not add a flush or change immediate-exit behavior.

ABI references:

- [Apple: Writing ARM64 code for Apple platforms](https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms)
- [LLVM 21.1.8 Darwin variadic calling convention](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.8/llvm/lib/Target/AArch64/AArch64CallingConvention.td)
- [LLVM 21.1.8 va_start/va_arg lowering](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.8/llvm/lib/Target/AArch64/AArch64ISelLowering.cpp)

The LP64 Darwin `va_list` is one char-pointer cursor, not the generic AAPCS64
register-save structure. More-aligned scalars, aggregates and other variadic
types need separate classification; this adapter makes no claim for them.

## Owned immutable string exports

An address being mapped in the host is not ownership evidence. The adapter
does not use the interpreter's existing host-passthrough probes for format,
stack or string reads.

Each interpreter Memory owns a bounded collector of immutable string
snapshots. During a synchronous, resolved host Objective-C IMP call, a scoped
observer receives notifications from the central CFString constructor while
that constructor still owns its CString. It copies those owned bytes and
records the object/backing-address pair. No receiver fields or arbitrary raw
pointers are read to reconstruct ownership.

A creation observation alone does not authorize string access. After the
normal guest-code check, the exact built-in UTF8String IMP is invoked with its
declared signature. Only its matching observed object and returned backing
address can export the snapshot. Normal, Super and Super2 dispatch share this
path. Selector names and guest type encodings never establish the signature or
ownership. The guest receives the original returned pointer.

Exports are a separate read-only byte-copy table, not general Memory regions.
They do not make an address executable or authorize variadic stack reads.
The collector retains at most 1,024 observations and 1 MiB of string bytes,
including their NUL terminators. Existing reinitialization and mutable backing
replacement paths invalidate pending and exported snapshots before replacing
the data. They do not create immutable observations for the replacement.

The observer stack is thread-local only to route active calls. Owned records
belong to that interpreter; no global address registry or raw mutable Memory
pointer is installed. Creation reaches only the innermost observer.
Invalidation reaches every active enclosing observer, so a nested interpreter
cannot leave its caller's snapshot stale when it replaces the same backing.
Nested calls restore the previous observer even on unwind, and workers do not
inherit it. Export bytes remain alive while that same interpreter runs its exit
callbacks, then disappear with Memory. The original shim allocation and its
existing lifetime are unchanged.

## Limits and validation

Mutable, unobserved, worker-created and earlier-interpreter strings are not
qualified exports. In particular, separate +load and main interpreters do not
share collectors. Direct C factory calls outside the observed Objective-C path
are not covered by this constructor observation slice. Imported constant
strings can be read only when their bytes are already in registered memory.
Unsupported host strings cause explicit printf rejection.

Invalidation covers the active synchronous scopes only. Mutation from another
worker, an inactive interpreter, or an unobserved native call is outside this
export contract; the collector is not a general shared-object lifetime system.

The existing host-IMP and constructor valid-argument contracts still apply.
This is no claim of memory confinement, thread ownership, native-x86 Darwin
guest support, or an enforced security backend. Other variadic entry points,
generic floating-point/struct/indirect-result translation, and general
Objective-C string lifetime management remain separate work. Native execution
and the existing ARM64-host printf route are unchanged.

Source-owned tests exercise actual BLR/FFI dispatch, signed padding, unsigned
values, byte strings, exact target/IMP selection, limits, rejected memory and
formats, nested observer restoration and invalidation, worker exclusion,
reinitialization, mutable replacement and an interpreted exit callback using
an export.
The existing NSCharacterSet fixture also compares its actual stdout with
`fixtures/nscharacterset_basics/expected_stdout.txt` on Unix x86_64 hosts.
That golden is 415 bytes and requires the existing semantic exit check as
well. Missing fixture inputs retain the suite's explicit missing-input report;
a successful qualification run must supply the fixture and execute the check.

Focused commands, using the repository's pinned Rust toolchain:

```sh
cargo test --locked -p arm64-translate --lib mem::cstring_exports::tests
cargo test --locked -p cf-lite --lib cstring_observer::tests
cargo test --locked -p macho-runtime --lib interp_bridge::printf::tests -- --test-threads=1
cargo test --locked -p macho-runtime --test interpreted_objc_tests -- interp_nscharacterset_basics --exact --nocapture --test-threads=1
```

These checks do not replace required CI or selected-guest execution evidence.
