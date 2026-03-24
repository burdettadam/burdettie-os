# FFI Guide — Using darwin-runtime from C, Swift, and Python

The `darwin-runtime-ffi` crate exposes the runtime's core functionality through
a stable C ABI. Any language with C FFI support can load the library and call
these functions.

## Building the library

```bash
cargo build --release -p darwin-runtime-ffi
```

This produces `target/release/libdarwin_runtime_ffi.dylib` and a generated C
header at `crates/darwin-runtime-ffi/include/darwin_runtime.h`.

## Installing

```bash
# Install to /usr/local (default)
./packaging/install.sh

# Or a custom prefix
./packaging/install.sh --prefix ~/darwin-runtime

# Or via Homebrew
brew install --build-from-source burdetty-os/tap/darwin-runtime
```

After installation, `pkg-config darwin-runtime --exists` returns 0.

---

## API Reference

### Types

| Type | Description |
|---|---|
| `DarwinVersion` | `{ major, minor, patch }` — library version |
| `DarwinRuntimeResult` | `{ exit_code, images_loaded, shims_installed, initializers_run, error }` — execution result |
| `DarwinClassInfo` | `{ name, method_count, class_method_count, instance_size, is_root }` — ObjC class metadata |
| `DarwinMethodInfo` | `{ selector, type_encoding }` — ObjC method metadata |
| `DarwinClassList` | `{ names, count }` — array of class name strings |

### Functions

#### Version

| Function | Signature | Description |
|---|---|---|
| `darwin_runtime_version` | `() → DarwinVersion` | Returns the library version (currently 0.1.0) |

#### Execution

| Function | Signature | Description |
|---|---|---|
| `darwin_runtime_run` | `(path: *const char) → DarwinRuntimeResult` | Run a Mach-O binary with permissive security |
| `darwin_runtime_run_secure` | `(path: *const char) → DarwinRuntimeResult` | Run with Apple-like security policy |

#### ObjC Introspection

| Function | Signature | Description |
|---|---|---|
| `darwin_objc_class_count` | `() → uintptr_t` | Number of registered ObjC classes |
| `darwin_objc_selector_count` | `() → uintptr_t` | Number of registered selectors |
| `darwin_objc_get_class` | `(name: *const char) → DarwinClassInfo` | Look up a class by name |
| `darwin_objc_list_classes` | `() → DarwinClassList` | List all registered class names |
| `darwin_objc_list_methods` | `(name: *const char, out_count: *mut uintptr_t) → *mut DarwinMethodInfo` | List instance methods of a class |
| `darwin_foundation_class_count` | `() → uintptr_t` | Count of Foundation classes (NSString, etc.) |

#### Error Handling

| Function | Signature | Description |
|---|---|---|
| `darwin_runtime_last_error` | `() → *const char` | Last error message (thread-local), or NULL |

#### Memory Management

| Function | Signature | Description |
|---|---|---|
| `darwin_string_free` | `(s: *mut char) → void` | Free a string returned by a `darwin_*` function |
| `darwin_class_list_free` | `(list: DarwinClassList) → void` | Free a class list from `darwin_objc_list_classes` |
| `darwin_method_list_free` | `(methods: *mut DarwinMethodInfo, count: uintptr_t) → void` | Free a method array from `darwin_objc_list_methods` |

### Memory ownership rules

- Strings inside `DarwinClassInfo.name` are **caller-owned** — free with `darwin_string_free`.
- `DarwinClassList` is **caller-owned** — free with `darwin_class_list_free`.
- Method arrays from `darwin_objc_list_methods` are **caller-owned** — free with `darwin_method_list_free`.
- The pointer from `darwin_runtime_last_error` is **library-owned** — do NOT free it. It is valid until the next FFI call on the same thread.

---

## Usage from C

### Build with pkg-config

```bash
cc -o myapp myapp.c $(pkg-config --cflags --libs darwin-runtime)
```

### Example

```c
#include <darwin_runtime.h>
#include <stdio.h>

int main(void) {
    // Check version
    DarwinVersion v = darwin_runtime_version();
    printf("darwin-runtime %u.%u.%u\n", v.major, v.minor, v.patch);

    // Run a binary
    DarwinRuntimeResult r = darwin_runtime_run("/path/to/fixture");
    if (r.error != 0) {
        const char *err = darwin_runtime_last_error();
        fprintf(stderr, "Error: %s\n", err ? err : "unknown");
        return 1;
    }
    printf("exit code: %d, images: %zu\n", r.exit_code, r.images_loaded);

    // Introspect ObjC runtime (after running a fixture)
    printf("Classes: %zu\n", darwin_objc_class_count());

    DarwinClassList list = darwin_objc_list_classes();
    for (size_t i = 0; i < list.count; i++) {
        printf("  %s\n", list.names[i]);
    }
    darwin_class_list_free(list);

    // Look up a specific class
    DarwinClassInfo info = darwin_objc_get_class("NSString");
    if (info.name) {
        printf("NSString: %zu methods, %zu class methods\n",
               info.method_count, info.class_method_count);
        darwin_string_free(info.name);
    }

    return 0;
}
```

See [examples/c-client/](../examples/c-client/) for a complete working example.

---

## Usage from Swift

The `swift/` directory contains a Swift Package Manager package with an
idiomatic Swift wrapper.

### Add as a dependency

```swift
// Package.swift
dependencies: [
    .package(path: "../path/to/burdetty-os/swift"),
]
```

### Example

```swift
import DarwinRuntime

// Version
let v = DarwinRuntime.version
print("darwin-runtime \(v)")  // "darwin-runtime 0.1.0"

// Run a binary
do {
    let result = try DarwinRuntime.run(path: "/path/to/fixture")
    print("exit: \(result.exitCode), images: \(result.imagesLoaded)")
} catch {
    print("Error: \(error)")
}

// ObjC introspection (after running a fixture)
print("Classes: \(ObjCRuntime.classCount)")
for name in ObjCRuntime.listClasses() {
    print("  \(name)")
}

if let info = ObjCRuntime.getClass(name: "NSString") {
    print("NSString: \(info.methodCount) methods")
}

for method in ObjCRuntime.listMethods(className: "NSString") {
    print("  -[\(method.selector)] \(method.typeEncoding)")
}
```

See [examples/swift-client/](../examples/swift-client/) for a complete working
example.

---

## Usage from Python

Use `ctypes` to load the dylib directly — no compilation needed.

### Example

```python
import ctypes
import os

# Load the library
lib = ctypes.CDLL("libdarwin_runtime_ffi.dylib")

# Define return types
class DarwinVersion(ctypes.Structure):
    _fields_ = [
        ("major", ctypes.c_uint32),
        ("minor", ctypes.c_uint32),
        ("patch", ctypes.c_uint32),
    ]

class DarwinRuntimeResult(ctypes.Structure):
    _fields_ = [
        ("exit_code", ctypes.c_int32),
        ("images_loaded", ctypes.c_size_t),
        ("shims_installed", ctypes.c_size_t),
        ("initializers_run", ctypes.c_size_t),
        ("error", ctypes.c_int32),
    ]

# Configure function signatures
lib.darwin_runtime_version.restype = DarwinVersion
lib.darwin_runtime_version.argtypes = []

lib.darwin_runtime_run.restype = DarwinRuntimeResult
lib.darwin_runtime_run.argtypes = [ctypes.c_char_p]

lib.darwin_objc_class_count.restype = ctypes.c_size_t
lib.darwin_objc_class_count.argtypes = []

lib.darwin_runtime_last_error.restype = ctypes.c_char_p
lib.darwin_runtime_last_error.argtypes = []

# Use it
v = lib.darwin_runtime_version()
print(f"darwin-runtime {v.major}.{v.minor}.{v.patch}")

result = lib.darwin_runtime_run(b"/path/to/fixture")
if result.error == 0:
    print(f"exit: {result.exit_code}, images: {result.images_loaded}")
else:
    err = lib.darwin_runtime_last_error()
    print(f"Error: {err.decode() if err else 'unknown'}")

print(f"ObjC classes: {lib.darwin_objc_class_count()}")
```

> **Note**: Set `DYLD_LIBRARY_PATH` to include the directory containing
> `libdarwin_runtime_ffi.dylib`, or use the full path in `ctypes.CDLL()`.

---

## Thread safety

- Each thread has its own error state (`darwin_runtime_last_error`).
- The ObjC runtime state is protected by a global mutex. Introspection functions
  are thread-safe but will block if another thread is executing a binary.
- `darwin_runtime_run` and `darwin_runtime_run_secure` should not be called
  concurrently — the runtime uses global state for the loaded binary.

## Error handling pattern

All execution functions set a thread-local error on failure:

```c
DarwinRuntimeResult r = darwin_runtime_run(path);
if (r.error != 0) {
    const char *msg = darwin_runtime_last_error();
    // msg is valid until the next darwin_* call on this thread
    fprintf(stderr, "Failed: %s\n", msg);
}
```
