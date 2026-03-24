# Host Dependency Register — v0.1.0

> Every direct host OS coupling point in the codebase, grouped by
> abstraction category and mapped to its target PAL trait (Phase 31).
>
> This register is the extraction checklist for `platform-core` /
> `platform-darwin-host`.

---

## Summary

| PAL Trait | Callsites | Files | Crates |
|-----------|-----------|-------|--------|
| `MemoryMapper` | 4 | 1 | macho-exec |
| `HostSymbolResolver` | 15 | 5 | macho-exec, objc-runtime, cf-lite |
| `UnwindRegistry` | 3 | 1 | macho-exec |
| `HostObjCBridge` | 5 trampolines | 3 asm files | objc-runtime, cf-lite |
| `Clock` | 6 | 4 | cf-lite (3), libsystem-shims (1) |
| `Rng` | 1 | 1 | cf-lite |
| `Hasher` | 1 | 1 | macho-parse |
| `FilesystemProvider` | 12 (prod) | 8 | macho-runtime, macho-resolve, cf-lite, macho-inspect, macho-fixtures, test-harness |
| `ProcessInfo` | 5 | 2 | cf-lite, libsystem-shims |
| `ThreadPrimitives` | 50+ | 4 | libsystem-shims, cf-lite, objc-runtime |
| Heap allocation | 100+ | 25+ | cf-lite (20+), objc-runtime (2), libsystem-shims (2) |

---

## 1. Memory Mapping — `MemoryMapper`

All callsites are in `crates/macho-exec/src/mapped.rs`.

| Line | Call | Purpose |
|------|------|---------|
| ~76 | `libc::sysconf(libc::_SC_PAGESIZE)` | Page size for alignment |
| ~123 | `libc::mmap(null, size, PROT_READ\|PROT_WRITE, MAP_ANON\|MAP_PRIVATE, -1, 0)` | Allocate contiguous anonymous region per image |
| ~205 | `libc::mprotect(addr, size, prot)` | Set per-segment protection after fixups |
| ~239 | `libc::munmap(addr, size)` | Cleanup on `MappedImage::drop` |

**Extraction plan**: Create `MemoryMapper` trait with `page_size()`, `map_anonymous()`, `protect()`, `unmap()`.  Move calls into `DarwinHostPlatform` impl.

---

## 2. Symbol Resolution — `HostSymbolResolver`

### `macho-exec` (resolver)

| File | Line | Call | Purpose |
|------|------|------|---------|
| `fixup/resolver.rs` | ~18 | `RTLD_DEFAULT` handle init | StubResolver handle |
| `fixup/resolver.rs` | ~29 | `libc::dlsym(handle, name)` | Resolve safe-listed symbols to host addresses (86+ symbols) |

### `objc-runtime` (host ObjC bridge)

| File | Line | Call | Purpose |
|------|------|------|---------|
| `shims/dispatch.rs` | ~25 | `libc::dlsym(RTLD_DEFAULT, "objc_msgSend")` | Host ObjC msg fallback |
| `shims/dispatch.rs` | ~40 | `libc::dlsym(RTLD_DEFAULT, "sel_registerName")` | Host selector registration |
| `shims/lifecycle.rs` | ~816 | `libc::dlsym(RTLD_DEFAULT, "objc_exception_throw")` | Host exception throw |
| `shims/lifecycle.rs` | ~865 | `libc::dlsym(rtld, "objc_getClass")` | Create host NSException |
| `shims/lifecycle.rs` | ~866 | `libc::dlsym(rtld, "sel_registerName")` | Create host NSException |
| `shims/lifecycle.rs` | ~867 | `libc::dlsym(rtld, "objc_msgSend")` | Create host NSException |
| `shims/lifecycle.rs` | ~943 | `libc::dlsym(RTLD_DEFAULT, "objc_begin_catch")` | Host C++ EH catch |
| `shims/lifecycle.rs` | ~962 | `libc::dlsym(RTLD_DEFAULT, "objc_end_catch")` | Host C++ EH end catch |
| `shims/lifecycle.rs` | ~982 | `libc::dlsym(RTLD_DEFAULT, "objc_exception_rethrow")` | Host exception rethrow |

### `cf-lite` (Foundation bridge)

| File | Line | Call | Purpose |
|------|------|------|---------|
| `nssortdescriptor.rs` | ~26 | `libc::dlsym(rtld, "sel_registerName")` | Host compare dispatch |
| `nssortdescriptor.rs` | ~27 | `libc::dlsym(rtld, "objc_msgSend")` | Host compare dispatch |
| `foundation/mod.rs` | ~209 | `libc::dlopen("Foundation.framework", RTLD_LAZY)` | Load host Foundation for EHTYPE |
| `foundation/mod.rs` | ~226 | `libc::dlsym(handle, "OBJC_EHTYPE_$_NSException")` | Resolve NSException EHTYPE for @catch |

**Extraction plan**: Create `HostSymbolResolver` trait with `resolve(name)`, `resolve_in(handle, name)`, `open_library(path)`.  The ObjC-specific bridge calls feed into a `HostObjCBridge` trait subset.

---

## 3. Unwind Registration — `UnwindRegistry`

All callsites in `crates/macho-exec/src/unwind.rs`.

| Line | Symbol | Purpose |
|------|--------|---------|
| ~60 | `extern "C" { __unw_add_find_dynamic_unwind_sections }` | FFI to libunwind registration |
| ~63 | `extern "C" { __unw_remove_find_dynamic_unwind_sections }` | FFI to libunwind deregistration |
| ~74 | `unsafe extern "C" fn find_unwind_sections(...)` | Callback invoked by libunwind |

**Extraction plan**: Create `UnwindRegistry` trait with `register(segments, slide)`, `unregister(segments, slide)`.  macOS-specific libunwind details stay in `DarwinHostPlatform`.

---

## 4. Assembly Trampolines — `HostObjCBridge`

Architecture-locked to ARM64 (AArch64).

| File | Symbol | Purpose |
|------|--------|---------|
| `objc-runtime/src/trampoline.s` | `_shim_objc_msg_send_asm` | ARM64 objc_msgSend — preserves x0-x7, d0-d7, lookup IMP, tail-call |
| `objc-runtime/src/trampoline.s` | `_shim_objc_msg_send_super_asm` | ARM64 objc_msgSendSuper — struct read + lookup + tail-call |
| `objc-runtime/src/trampoline.s` | `_shim_objc_msg_send_super2_asm` | ARM64 objc_msgSendSuper2 — current-class variant |
| `cf-lite/src/nslog_trampoline.s` | `_shim_nslog_asm` | ARM64 NSLog variadic → loads stack args, tail-calls Rust |
| `cf-lite/src/format_trampoline.s` | `_shim_nsstring_format_asm` | ARM64 stringWithFormat: variadic → shuffles args, tail-calls Rust |

**Extraction plan**: Trampolines are platform-arch specific.  On non-ARM64, alternative dispatch (C interop, libffi) would be needed.  Keep trampolines in arch-gated modules; provide arch selection via `HostObjCBridge` trait.

---

## 5. Clock / Time — `Clock`

| File | Line | Call | Purpose |
|------|------|------|---------|
| `cf-lite/src/nsdate.rs` | ~19 | `libc::gettimeofday(...)` | Wall clock for NSDate |
| `cf-lite/src/nstimezone.rs` | ~31 | `libc::gettimeofday(...)` | Wall clock for timezone offset |
| `cf-lite/src/nstimezone.rs` | ~33 | `libc::localtime_r(...)` | Timezone offset calculation |
| `cf-lite/src/nstimezone.rs` | ~41 | `libc::readlink("/etc/localtime")` | IANA timezone name |
| `cf-lite/src/nscalendar.rs` | ~90 | `libc::localtime_r(...)` | Calendar date components |
| `libsystem-shims/src/mach.rs` | ~44 | `libc::mach_absolute_time()` | Monotonic clock |

**Extraction plan**: Create `Clock` trait with `wall_time_usec()`, `monotonic_ns()`, `utc_offset_sec()`, `timezone_name()`.

---

## 6. Random Number Generation — `Rng`

| File | Line | Call | Purpose |
|------|------|------|---------|
| `cf-lite/src/nsuuid.rs` | ~19 | `libc::arc4random_buf(buf, len)` | CSPRNG for UUID v4 generation |

**Extraction plan**: Create `Rng` trait with `fill_bytes(buf)`.

---

## 7. Cryptographic Hashing — `Hasher`

| File | Line | Call | Purpose |
|------|------|------|---------|
| `macho-parse/src/codesig.rs` | ~193 | `extern "C" { CC_SHA256(data, len, out) }` | Code signature page hash verification (CommonCrypto FFI) |

**Extraction plan**: Create `Hasher` trait with `sha256(data) -> [u8; 32]`.  Alternative: replace with pure-Rust `sha2` crate to eliminate the FFI entirely.

---

## 8. Process Information — `ProcessInfo`

| File | Line | Call | Purpose |
|------|------|------|---------|
| `cf-lite/src/nsprocessinfo.rs` | ~45 | `libc::getpid()` | Process identity for NSProcessInfo |
| `cf-lite/src/nsprocessinfo.rs` | ~109 | `libc::getpid()` | Process description |
| `libsystem-shims/src/process.rs` | ~95 | `shim_getprogname()` | Process name (reads static state, no libc) |
| `libsystem-shims/src/process.rs` | ~103 | `shim_nsgetexecutablepath()` | Executable path (reads static state) |
| `libsystem-shims/src/process.rs` | ~144 | `shim_getenv()` | Environment variable lookup (static state) |

**Extraction plan**: Create `ProcessInfo` trait with `pid()`, `process_name()`, `executable_path()`.

---

## 9. Mach Kernel Services

| File | Line | Call | Purpose |
|------|------|------|---------|
| `libsystem-shims/src/mach.rs` | ~37 | `libc::mach_task_self()` | Mach task self port |
| `libsystem-shims/src/mach.rs` | ~44 | `libc::mach_absolute_time()` | Monotonic time (→ `Clock` trait) |
| `libsystem-shims/src/mach.rs` | ~49 | `mach_timebase_info(...)` | Timebase conversion info |

**Extraction plan**: `mach_task_self` → `ProcessInfo` trait.  Timebase → `Clock` trait.

---

## 10. Threading / Pthread — `ThreadPrimitives`

### `libsystem-shims/src/pthread.rs` (42 shims — centralized)

All pthread operations delegate to real `libc::pthread_*`:
- Thread lifecycle: create, join, detach, exit, self, cancel, equal
- Attributes: attr_init, attr_destroy, attr_setdetachstate, attr_getdetachstate
- Mutex: init, lock, trylock, unlock, destroy
- Mutex attributes: init, destroy, settype
- RWLock: init, rdlock, wrlock, tryrdlock, trywrlock, unlock, destroy
- Condition variables: init, destroy, wait, timedwait, signal, broadcast
- Thread-specific data: key_create, key_delete, getspecific, setspecific
- Once: pthread_once
- Scheduling: sched_yield, pthread_main_np

### `libsystem-shims/src/tls.rs`

- `pthread_key_create` / `pthread_getspecific` / `pthread_setspecific` for per-thread TLV storage

### `cf-lite/src/nslock.rs`

- `pthread_mutex_init/lock/trylock/unlock/destroy` for NSLock, NSRecursiveLock
- `pthread_cond_init/wait/signal/broadcast/destroy` for NSCondition, NSConditionLock
- `pthread_mutexattr_*` for recursive mutex setup

### `cf-lite/src/nsthread.rs`

- `libc::pthread_main_np()` for isMainThread
- `libc::nanosleep(...)` for sleepForTimeInterval:

### `objc-runtime/src/shims/lifecycle.rs`

- `pthread_mutex_init/lock/unlock/destroy` + `pthread_mutexattr_*` for @synchronized support

**Extraction plan**: Create `ThreadPrimitives` trait or keep as host-delegated (since pthreads are fundamental to in-process execution).

---

## 11. Heap Allocation — `libc::calloc` / `malloc` / `free`

**Pervasive** — 100+ callsites across 25+ files.

### Major clusters

| Crate | Pattern | Scope |
|-------|---------|-------|
| cf-lite | `libc::calloc(1, SIZE)` for alloc, `libc::free` for dealloc | All 40 Foundation classes |
| objc-runtime | `libc::calloc` for objc_alloc, method/ivar/property arrays | lifecycle.rs, blocks.rs |
| libsystem-shims | `libc::calloc` for TLV storage | tls.rs |

**Extraction plan**: Heap allocation is typically not abstracted away (it's universally available).  Consider a `HeapAllocator` trait only if targeting no-std environments.  For Phase 31, leave as-is.

---

## 12. Filesystem — `FilesystemProvider`

### Production code

| File | Line | Call | Purpose |
|------|------|------|---------|
| `macho-runtime/src/lib.rs` | ~79 | `std::fs::read(binary_path)` | Load main binary |
| `macho-runtime/src/loader.rs` | ~38 | `std::fs::read(&path)` | Load dylib dependency |
| `macho-resolve/src/bind.rs` | ~67 | `std::fs::read(path)` | Read dependency for binding |
| `macho-resolve/src/resolve/mod.rs` | ~180 | `std::fs::read(&resolved)` | Read child dependency |
| `cf-lite/src/nsfilemanager.rs` | ~43 | `Path::exists()` | fileExistsAtPath: |
| `cf-lite/src/nsfilemanager.rs` | ~75 | `Path::is_dir()` | fileExistsAtPath:isDirectory: |
| `cf-lite/src/nsfilemanager.rs` | ~82 | `std::env::current_dir()` | currentDirectoryPath |
| `cf-lite/src/nsfilemanager.rs` | ~96 | `std::fs::read_dir(path)` | contentsOfDirectoryAtPath: |
| `macho-inspect/src/exec.rs` | ~40 | `std::fs::read(&path)` | CLI dylib loading |
| `macho-fixtures/src/lib.rs` | ~28 | `std::fs::read_to_string(path)` | Fixture manifest loading |
| `test-harness/src/lib.rs` | multiple | `std::fs::read(binary_path)` | Test runner binary loading |

### Test-only code

~40+ sites across `macho-parse`, `macho-map`, `macho-objc`, `macho-resolve`, `macho-exec`, `trust-policy`, `macho-run-plan` tests — all reading fixture binaries.

**Extraction plan**: Create `FilesystemProvider` trait with `read(path)`, `exists(path)`, `read_link(path)`, `read_dir(path)`.

---

## Extraction Priority (Phase 31)

| Priority | Trait | Reason |
|----------|-------|--------|
| 1 | `MemoryMapper` | 4 calls, 1 file — cleanest extraction |
| 2 | `UnwindRegistry` | 3 calls, 1 file — clean boundary |
| 3 | `Hasher` | 1 call — could replace with pure-Rust sha2 crate |
| 4 | `Rng` | 1 call — trivial extraction |
| 5 | `Clock` | 6 calls, 4 files — moderate scatter |
| 6 | `ProcessInfo` | 5 calls, 2 files — moderate scatter |
| 7 | `FilesystemProvider` | 12 calls, 8 files — moderate scatter |
| 8 | `HostSymbolResolver` | 15 calls, 5 files — high scatter, complex |
| 9 | `ThreadPrimitives` | 50+ calls — defer unless targeting no-pthread environments |
| 10 | Heap allocation | 100+ calls — defer indefinitely (universally available) |
