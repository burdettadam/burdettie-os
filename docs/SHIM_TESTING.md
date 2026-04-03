# Foundation Shim Testing Guide

> How to write, run, and scale unit tests for cf-lite Foundation shims.
> Designed for parallel sub-agent development.

---

## Current State

| Metric | Value |
|--------|-------|
| Total `pub extern "C" fn shim_*` functions | **959** |
| Files with unit tests | **18** |
| Files with shims but **no tests** | **50+** |
| Unit tests total | **~140** |
| Corpus end-to-end tests | 30 tools (100% pass) |
| Fixture end-to-end tests | 63 (100% pass) |

Real Apple Foundation has **~2000–5000 selectors** across **~250 classes**.
The ROADMAP estimates **500+ additional selectors** needed beyond current shims
for meaningful macOS binary compatibility.

---

## Architecture

### Test Helper Module

All shim tests share a common utility module:

```
crates/cf-lite/src/shim_test_helpers.rs    # Shared helpers (cfg(test) only)
```

Key helpers:

| Function | Purpose |
|----------|---------|
| `alloc_obj(size, isa)` | Allocate a zeroed obj with real ISA |
| `alloc_obj_fake(size)` | Allocate with fake ISA `0xDEAD` |
| `free_obj(obj)` | Free an obj |
| `make_string(s)` | Create a CFString from `&str` |
| `read_string(obj)` | Extract CFString content as `String` |
| `assert_string_eq(obj, expected)` | Assert CFString value |
| `write_field(obj, offset, value)` | Write u64 at offset |
| `read_field(obj, offset)` | Read u64 at offset |
| `read_field_i64(obj, offset)` | Read i64 at offset |
| `alloc_u64_buf(count)` | Allocate a buffer of u64 slots |
| `free_buf(ptr)` | Free a buffer |
| `make_u64_array(items)` | Copy a `&[u64]` into an allocated buffer |

### Test Location

Tests live **inline** in each shim file as `#[cfg(test)] mod tests { ... }`.
This keeps tests close to the code they validate and allows targeted runs:

```bash
# Run tests for a single class
cargo test -p cf-lite -- nsdata::tests

# Run tests for multiple classes
cargo test -p cf-lite -- nsdata::tests nsvalue::tests nserror::tests

# Run all cf-lite tests
cargo test -p cf-lite
```

---

## How to Add Tests for an Untested Class

### Step 1: Read the shim file

Open the target file (e.g., `crates/cf-lite/src/nscache.rs`). Note:

1. **Object layout** — documented at the top (offsets for ISA, fields)
2. **`pub extern "C" fn shim_*`** — these are the functions to test
3. **Helper functions** — `make_*`, `alloc_*`, `extract_*` that construct objects
4. **Class address** — find the ISA via `crate::nsobj::<class>_class_addr()`

### Step 2: Add the test module

Append to the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn basic_creation() {
        // Use existing make_* helpers if available, or alloc_obj
        let obj = alloc_obj(24, crate::nsobj::nsdata_class_addr());
        // ... call shim functions ...
        free_obj(obj);
    }

    #[test]
    fn nil_returns_zero() {
        // Every shim should handle nil (obj=0) gracefully
        assert_eq!(shim_<class>_<method>(0, 0), 0);
    }
}
```

### Step 3: Test categories

For each class, cover these categories:

1. **Construction** — class methods (`+[Class ...]`) that create objects
2. **Accessors** — getters that read fields
3. **Mutators** — methods that modify state (for mutable classes)
4. **Nil safety** — every shim should handle `obj == 0`
5. **Edge cases** — empty collections, zero-length data, boundary values
6. **Equality/comparison** — `isEqual:`, `isEqualTo*:`, `compare:` if present
7. **Description** — `description` returns a valid NSString

### Step 4: Run

```bash
cargo test -p cf-lite -- <module>::tests
```

---

## Sub-Agent Parallelism

Each class is self-contained and can be worked on independently. A coordinating
agent can dispatch multiple sub-agents in parallel:

```
Sub-agent 1: Add tests for nscache.rs
Sub-agent 2: Add tests for nsdate.rs
Sub-agent 3: Add tests for nscharacterset.rs
Sub-agent 4: Add tests for nssortdescriptor.rs
```

### Rules for sub-agents

1. **Only modify the target file** — add `#[cfg(test)] mod tests` at the bottom
2. **Use `crate::shim_test_helpers::*`** — do NOT duplicate helper code
3. **Do NOT modify lib.rs** or other modules
4. **Test every `pub extern "C" fn shim_*`** in the file
5. **Always test nil input** — `assert_eq!(shim_foo(0, 0), 0)` or equivalent
6. **Free all allocations** — call `free_obj` on everything created
7. **No platform dependency** — tests run without `set_platform()`. If a shim
   calls `crate::platform()`, it will panic. Those shims can only be tested
   through corpus (note this in a comment).

### Prompt template for sub-agents

```
Add unit tests to crates/cf-lite/src/<FILE>.rs

Read the file to understand its object layout and shim functions.
Add a #[cfg(test)] mod tests block at the end of the file.
Use `use super::*;` and `use crate::shim_test_helpers::*;`.

Test every pub extern "C" fn shim_* function:
- Basic construction and field roundtrip
- Nil/zero input safety
- Edge cases (empty, boundary values)
- Description format if present

Use alloc_obj(size, crate::nsobj::<class>_class_addr()) to create objects.
Use make_string("...") for CFString args.
Use free_obj() for cleanup.

Do NOT modify any other files.
```

---

## Classes Needing Tests (Priority Order)

### Tier 1 — High-traffic Foundation classes (do first)

| File | Class | Shim count |
|------|-------|------------|
| `nsdate.rs` | NSDate | ~10 |
| `nscache.rs` | NSCache | ~8 |
| `nssortdescriptor.rs` | NSSortDescriptor | ~5 |
| `nscharacterset.rs` | NSCharacterSet | ~15 |
| `nsenumerator.rs` | NSEnumerator | ~5 |
| `nsexception.rs` | NSException | ~8 |
| `nsfilemanager.rs` | NSFileManager | ~15 |
| `nslocale.rs` | NSLocale | ~10 |
| `nstimezone.rs` | NSTimeZone | ~8 |
| `nsurl.rs` | NSURL | ~12 |

### Tier 2 — Serialization & networking

| File | Class |
|------|-------|
| `nsjsonserialization.rs` | NSJSONSerialization |
| `nskeyedarchiver.rs` | NSKeyedArchiver |
| `nsxmlparser.rs` | NSXMLParser |
| `nsurlcomponents.rs` | NSURLComponents |
| `nsurlrequest.rs` | NSURLRequest |
| `nsstream.rs` | NSStream |

### Tier 3 — Text processing

| File | Class |
|------|-------|
| `nsregularexpression.rs` | NSRegularExpression |
| `nsscanner.rs` | NSScanner |
| `nsattributedstring.rs` | NSAttributedString |
| `nspredicate/` | NSPredicate |
| `nsdateformatter/` | NSDateFormatter |
| `nsnumberformatter.rs` | NSNumberFormatter |

### Tier 4 — Collections & containers

| File | Class |
|------|-------|
| `nsorderedset.rs` | NSOrderedSet |
| `nscountedset.rs` | NSCountedSet |
| `nshashtable.rs` | NSHashTable |
| `nsmaptable.rs` | NSMapTable |
| `nsmutabledata.rs` | NSMutableData |
| `nsmutableset.rs` | NSMutableSet |
| `nsmutablecharacterset.rs` | NSMutableCharacterSet |

### Tier 5 — Concurrency & system (need platform; partial coverage)

| File | Class | Note |
|------|-------|------|
| `nslock/` | NSLock, NSRecursiveLock, etc. | Uses `pal_threading()` — needs platform or skip |
| `nsthread.rs` | NSThread | Single-threaded stub |
| `nsoperation.rs` | NSOperation | Serialized execution |
| `nsrunloop.rs` | NSRunLoop | Single-pass stub |
| `nstask.rs` | NSTask | Needs filesystem; corpus covers this |
| `nsurlsession.rs` | NSURLSession | Needs network; corpus covers this |

### Tier 6 — AppKit stubs (low priority)

| File | Class | Note |
|------|-------|------|
| `nsappearance.rs` | NSAppearance | Stub — returns sentinel |
| `nsapplication.rs` | NSApplication | Stub |
| `nscolor.rs` | NSColor | Stub |
| `nsevent.rs` | NSEvent | Stub |
| `nsfont.rs` | NSFont | Stub |
| `nsimage.rs` | NSImage | Stub |
| `nsview.rs` | NSView | Stub |
| `nswindow.rs` | NSWindow | Stub |

---

## Known Compromises

These are documented to help future contributors understand the testing
boundaries. See also: `crates/cf-lite/src/shim_test_helpers.rs` header comments.

| # | Compromise | Impact | Remedy |
|---|-----------|--------|--------|
| C1 | **No ObjC dispatch in tests** — shim functions are called directly as Rust functions, bypassing `objc_msgSend` and selector lookup | Tests don't validate method registration or selector routing | Corpus/fixture tests cover dispatch integration |
| C2 | **No retain/release semantics** — tests manually `free_obj()`; no autorelease pool, no refcount tracking | Leaks in tests are invisible; double-free would crash not be caught | Acceptable for shim logic validation |
| C3 | **Fake ISA pointers** — many tests use `0xDEAD` instead of real class addresses | Some shims that inspect ISA to branch on class type won't work correctly with fake ISA | Use real ISA via `nsobj::<class>_class_addr()` where ISA matters |
| C4 | **No platform provider** — tests using `crate::platform()` will panic | NSDate, NSCalendar, NSTimeZone, NSUUID, NSFileManager cannot be fully unit-tested | Use corpus tests for platform-dependent shims; or call `set_platform()` in test setup |
| C5 | **Single-threaded only** — `cargo test` uses `--test-threads=1` for ObjC global state safety | Threading shims (NSLock, NSThread) are not stress-tested | Acceptable given L6 limitation |
| C6 | **String equality is byte-level** — `extract_cfstring_cstr` does UTF-8 comparison, not locale-aware `NSString isEqual:` semantics | Non-ASCII string comparison tests may diverge from Apple behavior | Acceptable for current scope |
| C7 | **Object layout is internal convention** — offset constants are defined per-file and not validated against Apple's real layouts | If Apple's layout differs from ours, tests pass but runtime will fail on real binaries | This is an inherent property of the shim approach; layout is verified through corpus tests |
| C8 | **~50+ shim files still have zero tests** — only ~18/73 source files have test coverage | Large regression risk surface | This guide enables systematic closure |

---

## Running the Full Suite

```bash
# All cf-lite unit tests
cargo test -p cf-lite

# All cf-lite unit tests + full workspace tests
cargo test

# Corpus compatibility (end-to-end)
cargo run -p compat-runner -- scan corpus/

# Specific class filter
cargo test -p cf-lite -- nsdata::tests --test-threads=1
```
