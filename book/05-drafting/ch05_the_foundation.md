# Chapter 5: The Foundation

## Scene 5.1: What Foundation Provides

The runtime works. `objc_msgSend` dispatches. The selector resolves. The method is found. The IMP is called.

```
$ macho-inspect --exec /opt/xcode-tools/xcode-tc/xcodebuild -version
[NSString stringWithFormat:] → 0x0000000000000000
```

No crash. That's progress. Chapter 4 ended with `SIGSEGV` — `x16 = 0x0000000000000000` because there was no dispatch at all. Now there *is* dispatch. The trampoline runs. The ISA chain walks. The selector matches. But `stringWithFormat:` returns nil. Not because the mechanism failed — because there's nothing behind it.

We have an ObjC runtime. We have class tables populated from `__objc_classlist`. We have selectors interned from `__objc_selrefs`. We have a method lookup that walks the superclass chain at assembly speed. And every method implementation points to a stub that returns zero.

Pull the thread. *Why* is `stringWithFormat:` returning nil?

The binary's `__objc_classlist` section defines classes the binary itself implements. But `NSString` isn't defined by the binary — it comes from Foundation.framework. On macOS, when `dyld` loads a binary, it also loads every framework the binary links against. Foundation.framework gets loaded, its classes get registered in the ObjC runtime, and `NSString` enters the class table with a full method list — 200+ methods, each backed by Apple's compiled implementation. By the time `xcodebuild` calls `[NSString stringWithFormat:]`, the method exists and works.

We don't load Foundation.framework. We can't — it's a Mach-O dylib compiled for macOS, full of Apple-private internal implementations, dependent on dozens of other system libraries. Even if we could parse and map it, we'd need to resolve its dependencies recursively: libdispatch, libsystem_c, libicucore, Security.framework, CoreServices, the entire macOS SDK. Foundation doesn't stand alone. It sits atop the full platform.

So we have an ObjC runtime that can dispatch messages, but no Foundation classes to dispatch *to*. The binary says `[NSString stringWithFormat:@"Xcode %@", version]`, and the runtime goes looking for class `NSString`, finds nothing, and returns nil. It's a language without vocabulary. A telephone system with no one to call.

Let's survey the damage. What does Foundation actually provide? Not the full catalog — Apple's Foundation.framework defines hundreds of classes. But which classes do real macOS binaries actually *use*?

We know because we can parse them. Every class reference in a binary appears in `__objc_classrefs`. Every selector in `__objc_selrefs`. We scan the 39 corpus tool binaries and the Xcode toolchain, counting class references:

| Class | Reference Count | Purpose |
|-------|----------------|---------|
| NSString | 39/39 binaries | Text — the universal currency |
| NSArray | 38/39 | Ordered collections |
| NSDictionary | 37/39 | Key-value mappings |
| NSData | 35/39 | Raw byte buffers |
| NSNumber | 34/39 | Boxed numerics |
| NSError | 33/39 | Error reporting |
| NSFileManager | 31/39 | Filesystem access |
| NSProcessInfo | 29/39 | Process environment, argv |
| NSBundle | 27/39 | App resources, Info.plist |
| NSDate | 25/39 | Timestamps |
| NSURL | 24/39 | File paths and network addresses |
| NSSet | 22/39 | Unordered unique collections |
| NSRegularExpression | 18/39 | Pattern matching |
| NSTask | 11/39 | Subprocess execution |
| NSJSONSerialization | 10/39 | JSON parsing/generation |
| NSURLSession | 8/39 | HTTP networking |
| NSPropertyListSerialization | 7/39 | Plist parsing |

That table is only the top 17. The full list is longer. Much longer. We ultimately need to implement: NSString, NSMutableString, NSArray, NSMutableArray, NSDictionary, NSMutableDictionary, NSData, NSMutableData, NSSet, NSMutableSet, NSNumber, NSDecimalNumber, NSURL, NSURLComponents, NSURLQueryItem, NSError, NSException, NSDate, NSDateFormatter, NSISO8601DateFormatter, NSCalendar, NSTimeZone, NSLocale, NSCharacterSet, NSMutableCharacterSet, NSAttributedString, NSMutableAttributedString, NSRegularExpression, NSTextCheckingResult, NSPredicate, NSScanner, NSValue, NSUUID, NSNull, NSIndexSet, NSMutableIndexSet, NSSortDescriptor, NSCountedSet, NSOrderedSet, NSCache, NSMapTable, NSHashTable, NSEnumerator, NSNotificationCenter, NSNotification, NSFileManager, NSFileHandle, NSBundle, NSProcessInfo, NSThread, NSLock, NSRecursiveLock, NSCondition, NSConditionLock, NSOperation, NSBlockOperation, NSOperationQueue, NSRunLoop, NSTimer, NSUserDefaults, NSTask, NSPipe, NSHost, NSProgress, NSStream, NSInputStream, NSOutputStream, NSURLRequest, NSMutableURLRequest, NSURLResponse, NSHTTPURLResponse, NSURLSession, NSURLSessionConfiguration, NSURLSessionDataTask, NSURLCredential, NSURLCredentialStorage, NSHTTPCookie, NSHTTPCookieStorage, NSJSONSerialization, NSPropertyListSerialization, NSKeyedArchiver, NSKeyedUnarchiver, NSXMLParser, NSByteCountFormatter, NSNumberFormatter, NSMethodSignature, NSInvocation, NSKeyValueObserving support, NSKeyValueCoding support, NSAutoreleasePool, NSMeasurement, and the host of AppKit stubs that Xcode tools reference.

Count them. That's 93+ classes. Each with its own object layout, its own method list, its own behavioral semantics. NSString alone has over 80 methods that binaries actually call. NSArray has 40+. NSDictionary has 30+. Every class must allocate objects with the correct layout, respond to the correct selectors, and return the correct values — or the binary will crash, produce garbage, or silently misbehave.

Chapter 4 proved that macOS binaries have a B story hiding inside them — the ObjC runtime. Chapter 5 reveals a C story hiding inside the B story: Foundation, the framework that provides the meaning behind every class reference. We built the message dispatch engine. Now we need to fill in every word of its vocabulary.

The ObjC runtime without Foundation is like having a perfectly tuned engine in a car with no wheels. The engine runs. The pistons fire. The crankshaft turns. And the car goes nowhere because there's nothing to transfer the energy to the road.

Ninety-three classes. Let's start.

---

## Scene 5.2: Synthetic Class Registration

The ObjC runtime we built in Chapter 4 reads class metadata from the binary's `__DATA` segment — ISA pointers, `class_ro_t` structures, method lists. It indexes whatever the binary defines. But Foundation classes aren't in the binary. They're supposed to come from a framework that we don't load. We need to register them ourselves, synthetically, from Rust.

Here's what the runtime needs for each class: a name, a mapped address (the ISA value that objects will carry in their first 8 bytes), a superclass, an instance size, and a method table. Each method entry is a triple: selector name, implementation function pointer, and ObjC type encoding string. We package this as a `ClassInfo`:

```rust
pub struct ClassInfo {
    pub name: String,
    pub mapped_addr: u64,
    pub methods: Vec<MethodEntry>,
    pub instance_methods: Vec<MethodEntry>,
    pub class_methods: Vec<MethodEntry>,
    pub superclass_name: Option<String>,
    pub instance_size: u32,
    // ...
}

pub struct MethodEntry {
    pub selector: Arc<str>,
    pub imp: u64,
    pub type_encoding: Arc<str>,
}
```

A `MethodEntry` maps a selector string to an IMP — a function pointer. For our synthetic classes, the IMP points to a Rust `extern "C"` function that implements the method's behavior. `"length"` → `shim_nsstring_length as *const () as u64`. `"UTF8String"` → `shim_nsstring_utf8string as *const () as u64`. `"objectAtIndex:"` → `shim_nsarray_object_at_index as *const () as u64`. Each shim function is a handwritten Rust implementation of one ObjC method.

The registration module lives in `cf-lite/src/foundation/register/`, a directory with 20+ submodules organized by domain:

```
register/
├── mod.rs                    # MethodEntry constructor, re-exports
├── string.rs                 # NSString, __NSCFConstantString, NSMutableString
├── object.rs                 # NSObject — the root of everything
├── collections/
│   ├── array.rs              # NSArray, NSMutableArray, NSConstantArray
│   ├── dictionary.rs         # NSDictionary, NSMutableDictionary
│   └── set.rs                # NSSet, NSMutableSet, NSCountedSet, NSOrderedSet
├── number.rs                 # NSNumber, NSDecimalNumber, NSConstantIntegerNumber
├── datetime.rs               # NSDate, NSDateFormatter, NSCalendar, NSTimeZone, NSLocale
├── data_url.rs               # NSData, NSMutableData, NSURL, NSURLComponents, NSUUID
├── error.rs                  # NSError, NSException
├── system.rs                 # NSFileManager, NSBundle, NSProcessInfo, NSUserDefaults
├── threading.rs              # NSThread, NSLock, NSRecursiveLock, NSCondition, NSRunLoop
├── invocation.rs             # NSMethodSignature, NSInvocation
├── text.rs                   # NSRegularExpression, NSPredicate, NSAttributedString, NSScanner
├── advanced_collections.rs   # NSCache, NSMapTable, NSHashTable, NSIndexSet
├── gap_data.rs               # NSJSONSerialization, NSPropertyListSerialization, NSKeyedArchiver
├── gap_network.rs            # NSURLSession, NSURLRequest, NSURLResponse, NSURLCredential
├── gap_streams.rs            # NSTask, NSPipe, NSHost, NSStream, NSFileHandle
├── measurement.rs            # NSMeasurement, NSByteCountFormatter
├── appkit/                   # NSApplication, NSWindow, NSView, NSEvent, NSMenu, ...
├── framework_stubs.rs        # CoreData, AVFoundation stubs
└── xcode_stubs.rs            # DVTFoundation, IDEFoundation bridges
```

Each file follows the same pattern. Take `register_nsarray_class` as a representative:

```rust
pub fn register_nsarray_class(rt: &mut ObjcRuntime) {
    let info = ClassInfo {
        name: "NSArray".to_string(),
        mapped_addr: nsobj::nsarray_class_addr(),
        superclass_name: Some("NSObject".to_string()),
        instance_size: 24,
        methods: vec![
            me("count", nsarray::shim_nsarray_count as u64, "Q@:"),
            me("objectAtIndex:", nsarray::shim_nsarray_object_at_index as u64, "@@:Q"),
            me("firstObject", nsarray::shim_nsarray_first_object as u64, "@@:"),
            me("lastObject", nsarray::shim_nsarray_last_object as u64, "@@:"),
            me("containsObject:", nsarray::shim_nsarray_contains as u64, "B@:@"),
            me("indexOfObject:", nsarray::shim_nsarray_index_of as u64, "Q@:@"),
            me("componentsJoinedByString:", nsarray::shim_nsarray_joined as u64, "@@:@"),
            me("sortedArrayUsingSelector:", nsarray::shim_nsarray_sorted as u64, "@@::"),
            me("description", nsarray::shim_nsarray_description as u64, "@@:"),
            // ... 30+ more methods
        ],
        class_methods: vec![
            me("array", nsarray::shim_nsarray_empty as u64, "@@:"),
            me("arrayWithObject:", nsarray::shim_nsarray_with_object as u64, "@@:@"),
            me("arrayWithObjects:count:", nsarray::shim_nsarray_with_objects_count as u64, "@@:^@Q"),
            me("alloc", nsarray::shim_nsarray_alloc as u64, "@@:"),
        ],
        // ...
    };
    rt.register_class(info);
}
```

The `me()` helper is a concise constructor for `MethodEntry` — selector string, IMP address, type encoding. The type encodings follow Apple's convention: `"Q@:"` means "returns unsigned long long, takes object receiver and selector." `"@@:@"` means "returns object, takes object receiver, selector, and one object argument." You can read them as a telegraph: first character is return type, `@:` is the implicit receiver+selector pair, remaining characters are argument types.

The `overlay_from()` pattern is how we compose this into a running runtime. We don't register 93 classes one at a time during every binary load. Instead, we build a template once:

```rust
static FOUNDATION_TEMPLATE: OnceLock<ObjcRuntime> = OnceLock::new();

fn get_template() -> &'static ObjcRuntime {
    FOUNDATION_TEMPLATE.get_or_init(|| {
        let mut rt = ObjcRuntime::new();
        FoundationShims::register_nsobject_class(&mut rt);
        FoundationShims::register_nsstring_class(&mut rt);
        FoundationShims::register_nsstring_proper_class(&mut rt);
        FoundationShims::register_nsarray_class(&mut rt);
        FoundationShims::register_nsdictionary_class(&mut rt);
        // ... 90 more registration calls
        rt
    })
}
```

Then, when a binary loads and the ObjC runtime initializes, we *overlay* the template onto the guest runtime:

```rust
pub fn register_foundation_classes(rt: &mut ObjcRuntime) {
    let template = get_template();
    rt.overlay_from(template);
}
```

`overlay_from()` clones the template's class table, protocol table, and tagged-pointer table into the guest runtime. Because `ClassInfo` is wrapped in `Arc` and selectors are `Arc<str>`, "cloning" is mostly incrementing reference counts — no deep copies, no string allocations. The entire Foundation registration takes about 11 microseconds. That's 93 classes, 1,100+ selectors, thousands of method entries — all materialized in the time it takes to run a few hundred instructions.

The template caching was a critical optimization. Before it, Foundation registration was the single largest phase in the loader's startup — over 50% of total time. We were allocating strings, building hash maps, interning selectors, all from scratch on every load. The `OnceLock` template reduced that to a single initialization plus a cheap overlay. The Arc sharing means the template's data is never copied — the guest runtime just points to the same allocations.

The class data symbols also need registration. Every binary that references `NSString` has a fixup for `_OBJC_CLASS_$_NSString` — a data symbol that the linker expects to resolve to the class's ISA address. We register these in a parallel table:

```rust
register_class!(table, "NSString", nsobj::nsstring_class_addr(),
                nsobj::nsstring_metaclass_addr());
register_class!(table, "NSArray", nsobj::nsarray_class_addr(),
                nsobj::nsarray_metaclass_addr());
register_class!(table, "NSDictionary", nsobj::nsdictionary_class_addr(),
                nsobj::nsdictionary_metaclass_addr());
// ... for all 93+ classes
```

Each class gets two data symbols: `_OBJC_CLASS_$_ClassName` (the class ISA) and `_OBJC_METACLASS_$_ClassName` (the metaclass ISA). When the fixup resolver encounters `_OBJC_CLASS_$_NSString` in a GOT slot, it finds our registered address and writes it in. The binary's class references now point to our synthetic class objects instead of Apple's.

It works. But calling `[NSString stringWithFormat:@"hello"]` still returns nil, because we haven't told the runtime what `stringWithFormat:` *does*. We've registered the shape of the class — its name, its ISA, its selector names. We haven't provided the substance.

That's next. And it starts with the hardest class.

---

## Scene 5.3: NSString Internals

Every macOS binary uses strings. Every one. `NSString` is not just the most-referenced Foundation class — it's the one that must work first, because every other class depends on it. `NSArray`'s `description` method returns an `NSString`. `NSError`'s `localizedDescription` returns an `NSString`. `NSFileManager`'s `currentDirectoryPath` returns an `NSString`. If strings don't work, *nothing* works.

Start with what the binary gives us. When the compiler sees `@"Hello, world"` — an ObjC string literal — it doesn't generate a constructor call. It emits a static struct in the `__cfstring` section of the binary:

```
__CFConstantString at 0x100008020:
  +0x00: ISA          (u64) → ___CFConstantStringClassReference
  +0x08: flags        (u32) → 0x07C8 (UTF-8, immutable)
  +0x0C: padding      (u32) → 0
  +0x10: c_string_ptr (u64) → pointer to "Hello, world\0" in __cstring
  +0x18: length       (u64) → 12
```

Four fields. Thirty-two bytes. The `ISA` points to `___CFConstantStringClassReference` — a data symbol the binary expects the linker to resolve. The `flags` value `0x07C8` encodes "this is a UTF-8, immutable string." The `c_string_ptr` is a pointer to the actual character data in the `__cstring` section. The `length` is the byte count.

This is the memory layout that `stringWithFormat:`, `length`, `UTF8String`, and every other NSString method must understand. Our shim functions read these offsets:

```rust
/// CFString layout: ISA(8) + flags(4) + pad(4) + c_string_ptr(8) + length(8) = 32 bytes
pub const CF_STRING_FLAGS_UTF8: u32 = 0x07C8;

pub fn cfstring_parts(obj: u64) -> Option<(*const c_char, usize)> {
    if obj == 0 { return None; }
    unsafe {
        let ptr = *((obj as usize + 16) as *const u64) as *const c_char;
        let len = *((obj as usize + 24) as *const u64) as usize;
        if ptr.is_null() { return None; }
        Some((ptr, len))
    }
}
```

`cfstring_parts` is the foundation (lowercase f) of everything. Given a pointer to any CFString-layout object — whether it's a compile-time `@"..."` literal or a dynamically-created `NSString` — it extracts the C string pointer and length. This one function is called by `UTF8String`, `length`, `description`, `isEqualToString:`, `hasPrefix:`, `hasSuffix:`, `compare:`, `hash`, string formatting, `NSLog`, collection descriptions, error messages, everything. If `cfstring_parts` breaks, the loader breaks.

Now let's look at how methods dispatch through this layout. Here's `UTF8String`:

```rust
/// `-[NSString UTF8String]`
pub extern "C" fn shim_nsstring_utf8string(obj: u64, _cmd: u64) -> *const c_char {
    if obj == 0 { return std::ptr::null(); }
    match cfstring_parts(obj) {
        Some((ptr, _)) => ptr,
        None => std::ptr::null(),
    }
}
```

And `length`:

```rust
/// `-[NSString length]`
pub extern "C" fn shim_nsstring_length(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    match cfstring_parts(obj) {
        Some((ptr, byte_len)) => {
            // NSString.length returns UTF-16 code unit count, not byte count
            let cstr = unsafe { CStr::from_ptr(ptr) };
            cstr.to_str().map(|s| s.encode_utf16().count() as u64).unwrap_or(0)
        }
        None => 0,
    }
}
```

There's a subtlety in `length` that took a day to find. Apple's `NSString.length` doesn't return the byte count. It returns the number of UTF-16 code units. ASCII characters are one unit each. Emoji and CJK characters are often two. We store UTF-8 bytes but must report UTF-16 units, because the binary's code does things like `for (NSUInteger i = 0; i < [str length]; i++) { unichar c = [str characterAtIndex:i]; }` and that loop counts in UTF-16, not UTF-8. Get the count wrong and the loop over-reads or under-reads. Get it right and strings just work.

Every shim function follows the same discipline: first argument is the receiver (`obj`), second is the selector (`_cmd`, usually ignored), remaining arguments match the ObjC method signature. The return type matches ObjC's expectation — `u64` for objects (pointers), `u64` for integers, `*const c_char` for C strings. And every shim checks `obj == 0` first, because ObjC's messaging-nil convention says sending any message to nil returns zero. Our stubs must honor that.

Now for the big one: `stringWithFormat:`. This is a class method that takes a format string and variadic arguments, like `printf` for ObjC:

```objc
NSString *msg = [NSString stringWithFormat:@"Hello, %@! You have %d messages.", name, count];
```

The `%@` format specifier is ObjC-specific — it calls `-description` on the argument and interpolates the result. `%d`, `%s`, `%f`, `%x` follow standard `printf` rules. The format string is an `NSString` (the first argument after `_cmd`), and the variadic arguments follow on the stack in Apple's ARM64 calling convention.

Our implementation lives in the format module:

```rust
/// `-[NSString stringWithFormat:]` (class method, variadic)
pub extern "C" fn shim_nsstring_string_with_format(
    _cls: u64, _cmd: u64, format_obj: u64, ...
) -> u64 {
    if format_obj == 0 { return 0; }

    let Some(fmt_str) = extract_cfstring_cstr(format_obj) else { return 0; };

    // Parse the format string, consuming va_args for each specifier
    let result = format::expand_format_string(fmt_str, /* va_list */);

    // Allocate a new NSString with the formatted result
    let cstr = CString::new(result).unwrap_or_default();
    shim_cf_string_create_with_cstring(0, cstr.as_ptr(), 0)
}
```

The `expand_format_string` function is the real complexity. It walks the format string character by character, recognizes `%@`, `%d`, `%ld`, `%lld`, `%s`, `%f`, `%x`, `%p`, `%c`, `%%`, and several dozen more format specifiers including width, precision, and flags. For `%@`, it reads a `u64` argument from the va_list (an ObjC object pointer) and calls our `format_objc_object()` function, which tries to extract a string via `cfstring_parts`, falls back to calling `-description` through the ObjC runtime, and ultimately produces a `String`. For `%d`, it reads an `i32`. For `%s`, it reads a `*const c_char` and converts to Rust's `&str`.

Let's test it. We build a corpus tool called `string_processor` that exercises NSString:

```
$ macho-inspect --exec corpus/string_processor/string_processor
[CHECK 1/12] NSString literal length: PASS
[CHECK 2/12] UTF8String round-trip:   PASS
[CHECK 3/12] stringWithFormat basic:   PASS
[CHECK 4/12] stringWithFormat %@:      PASS
[CHECK 5/12] isEqualToString:          PASS
[CHECK 6/12] hasPrefix/hasSuffix:      PASS
...
12/12 checks passed
```

There it is. `[NSString stringWithFormat:@"Hello, %@", name]` → correct output. The format string is parsed. The `%@` is expanded by calling `-description` on the argument. The result is assembled into a new `NSString`. The binary gets back a valid string with the right content.

This is the Midpoint moment. The first Foundation class *works*. We've gone from Chapter 4's "dispatch succeeds but returns nil" to "dispatch succeeds, formats a string, and returns a valid object." Real behavior, not a stub. The simplest thing that could work — formatting a greeting string — actually works.

Victory. For about ten seconds.

Because now we have to do it ninety-two more times. NSString was the hardest class to get *first*, but it's only one class. NSArray needs a different layout, different methods, different semantics. NSDictionary needs key hashing and lookup. NSData needs byte buffers. NSFileManager needs filesystem access. NSTask needs subprocess execution. Each class is a full implementation — not a thin wrapper, but a behavioral replica of what Apple's Foundation provides.

The false victory isn't that NSString is wrong. NSString is right. The false victory is thinking we're nearly done because the hardest piece is finished. We have one class. We need ninety-three. And the distance between one and ninety-three is not ninety-two more of the same thing. Each class has unique problems. Each class reveals new assumptions the binary makes about Foundation's behavior. Each class is its own debugging session.

Let's keep going.

---

## Scene 5.4: Collections

Collections are the second pillar. After strings, the most common Foundation operations involve putting objects into arrays, dictionaries, and sets, then getting them back out. Every binary uses `NSArray`. Almost every binary uses `NSDictionary`. The compiler emits collection literals as constructor calls — `@[a, b, c]` becomes `[NSArray arrayWithObjects:count:]`, and `@{key: value}` becomes `[NSDictionary dictionaryWithObjects:forKeys:count:]`.

### NSArray

An NSArray is the simplest collection: an ordered list of objects. Our layout:

```rust
/// NSArray layout: ISA(8) + count(8) + elements_ptr(8) = 24 bytes
const NSARRAY_COUNT_OFF: usize = 8;
const NSARRAY_ELEMS_OFF: usize = 16;
const NSARRAY_SIZE: usize = 24;
```

The `elements_ptr` points to a separately allocated `u64[]` — an array of ObjC object pointers. `count` is the number of elements. When a binary calls `[NSArray arrayWithObjects:count:]`, we allocate 24 bytes for the array header, allocate `count * 8` bytes for the element buffer, copy the object pointers in, retain each one, and return the array:

```rust
pub extern "C" fn shim_nsarray_with_objects_count(
    _cls: u64, _cmd: u64, objects: u64, count: u64,
) -> u64 {
    let obj = pal_alloc().calloc(1, NSARRAY_SIZE) as u64;
    if obj == 0 { return 0; }

    unsafe {
        *(obj as *mut u64) = nsobj::nsarray_class_addr();
        *((obj as usize + NSARRAY_COUNT_OFF) as *mut u64) = count;
    }

    if count > 0 && objects != 0 {
        let buf = pal_alloc().malloc((count as usize) * 8);
        for i in 0..count as usize {
            let item = unsafe { *((objects as usize + i * 8) as *const u64) };
            retain_collection_object(item);
            unsafe { *((buf as usize + i * 8) as *mut u64) = item; }
        }
        unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *mut u64) = buf as u64; }
    }

    obj
}
```

The `retain_collection_object` call is important. ObjC collections retain their elements — that's the ownership contract. We call `objc_retain` on each object as it enters the array. Without this, objects could be deallocated while the array still holds pointers to them, producing use-after-free crashes. The symmetrical `release_collection_object` fires when elements are removed or the array is deallocated.

`NSMutableArray` extends this with `addObject:`, `removeObjectAtIndex:`, `insertObject:atIndex:`, `replaceObjectAtIndex:withObject:`, and more. Internally it reallocs the element buffer when needed. The key behavioral contract: mutation methods must not crash on valid indices and must preserve retain counts through additions and removals.

But the critical method for collections isn't `objectAtIndex:` — it's `countByEnumeratingWithState:objects:count:`, the engine behind `for-in` loops:

```objc
for (NSString *item in array) {
    NSLog(@"%@", item);
}
```

The compiler transforms this into calls to `countByEnumeratingWithState:objects:count:` with an `NSFastEnumerationState` struct:

```rust
pub extern "C" fn shim_nsarray_count_by_enumerating(
    obj: u64, _cmd: u64, state: u64, buffer: u64, len: u64,
) -> u64 {
    if obj == 0 || state == 0 { return 0; }

    let count = unsafe { *((obj as usize + NSARRAY_COUNT_OFF) as *const u64) };
    let elems = unsafe { *((obj as usize + NSARRAY_ELEMS_OFF) as *const u64) };

    // state+0 is the iteration state; 0 means first call
    let current = unsafe { *((state as usize) as *const u64) };
    if current >= count { return 0; }

    let remaining = count - current;
    let batch = remaining.min(len);

    // Point itemsPtr (state+8) directly at our element buffer
    unsafe {
        *((state as usize + 8) as *mut u64) = elems + current * 8;
        *((state as usize + 16) as *mut u64) = &NSARRAY_FAST_ENUM_MUTATIONS as *const u64 as u64;
        *(state as *mut u64) = current + batch;
    }

    batch
}
```

This is the NSFastEnumeration protocol in action. The runtime calls it repeatedly, asking for batches of element pointers. We point `itemsPtr` directly at our internal element buffer — no copying — and return the batch size. When the current offset reaches the count, we return 0 to signal exhaustion. The `mutationsPtr` at state+16 is a pointer to a mutation counter that for-in checks to detect concurrent modification; we point it at a static zero because our immutable NSArray never mutates.

### NSDictionary

NSDictionary stores key-value pairs. Our layout uses parallel arrays:

```rust
/// NSDictionary layout: ISA(8) + count(8) + keys_ptr(8) + vals_ptr(8) = 32 bytes
const COUNT_OFF: usize = 8;
const KEYS_OFF: usize = 16;
const VALS_OFF: usize = 24;
```

Key lookup is the critical path. When a binary calls `[dict objectForKey:@"name"]`, we need to find the key in our keys array and return the corresponding value. Apple's NSDictionary uses hash tables internally, but our implementation uses linear search with an important optimization: pointer equality first, then string equality:

```rust
fn find_key_index(obj: u64, key: u64) -> Option<usize> {
    let count = dict_count(obj) as usize;
    let keys = dict_keys(obj);
    if count == 0 || keys == 0 || key == 0 { return None; }

    // Fast path: pointer equality
    for i in 0..count {
        let k = unsafe { *((keys as usize + i * 8) as *const u64) };
        if k == key { return Some(i); }
    }

    // Slow path: string content equality
    let needle = cfstring::extract_cfstring_cstr(key)?;
    for i in 0..count {
        let k = unsafe { *((keys as usize + i * 8) as *const u64) };
        if let Some(s) = cfstring::extract_cfstring_cstr(k) {
            if s == needle { return Some(i); }
        }
    }

    None
}
```

Pointer equality works here because many dictionary lookups use the same string literal objects — the binary's `@"name"` constant gets reused across calls. String content equality is the fallback for dynamically constructed keys. For our corpus tools with dictionaries typically under 50 entries, linear search is fast enough. Apple uses a hash table for dictionaries with thousands of entries; our tool binaries never approach that scale.

### NSSet

NSSet follows the same pattern: a bag of unique objects backed by a `Vec<u64>`. `NSMutableSet` adds `addObject:` and `removeObject:` with uniqueness enforcement. `NSCountedSet` tracks how many times each object was added. `NSOrderedSet` maintains insertion order.

Each collection class is a surface area commitment. Not just the data structure — the protocol conformance (`NSFastEnumeration`), the description format (`"(\n    item1,\n    item2\n)"`), the edge cases (nil arguments, empty collections, out-of-bounds indices), the mutable variants with their different class names and extended method lists.

By the time we've implemented all the collection classes — NSArray, NSMutableArray, NSDictionary, NSMutableDictionary, NSSet, NSMutableSet, NSCountedSet, NSOrderedSet, NSIndexSet, NSMutableIndexSet — we have about 200 shim functions. Each one tested. Each one matching Apple's behavior. The montage continues.

---

## Scene 5.5: NSFileManager and Friends

Foundation isn't just in-memory data structures. It reaches beyond the process boundary into the filesystem, the application bundle, and the property list format. Three classes form this bridge: `NSFileManager`, `NSBundle`, and `NSPropertyListSerialization`.

### NSFileManager

`NSFileManager` is a singleton that wraps filesystem operations. Our implementation delegates everything to Rust's `std::fs`:

```rust
//! NSFileManager layout: ISA(8) = 8 bytes (singleton, all state is host FS)

static DEFAULT_MANAGER: OnceLock<u64> = OnceLock::new();

fn get_or_create_default_manager() -> u64 {
    *DEFAULT_MANAGER.get_or_init(|| {
        let obj = pal_alloc().calloc(1, 8) as u64;
        if obj != 0 {
            unsafe { *(obj as *mut u64) = nsobj::nsfilemanager_class_addr(); }
        }
        obj
    })
}
```

The singleton pattern is important. Apple's Foundation has exactly one `[NSFileManager defaultManager]`. Multiple calls return the same pointer. Our `OnceLock` guarantees the same semantics — allocate once, return the same address forever.

The methods are straightforward translations:

```rust
pub extern "C" fn shim_nsfilemanager_file_exists_at_path(
    _obj: u64, _cmd: u64, path: u64,
) -> u8 {
    let utf8 = cfstring::shim_nsstring_utf8string(path, 0);
    if utf8.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(utf8) }.to_str().unwrap_or("");
    std::path::Path::new(s).exists() as u8
}

pub extern "C" fn shim_nsfilemanager_contents_at_path(
    _obj: u64, _cmd: u64, path: u64,
) -> u64 {
    let utf8 = cfstring::shim_nsstring_utf8string(path, 0);
    if utf8.is_null() { return 0; }
    let s = unsafe { CStr::from_ptr(utf8) }.to_str().unwrap_or("");
    match std::fs::read(s) {
        Ok(bytes) => nsdata::make_nsdata(&bytes),
        Err(_) => 0,
    }
}
```

`fileExistsAtPath:` calls `Path::exists()`. `contentsAtPath:` calls `std::fs::read()` and wraps the bytes in `NSData`. `contentsOfDirectoryAtPath:error:` calls `std::fs::read_dir()` and builds an `NSArray` of path strings. `createDirectoryAtPath:withIntermediateDirectories:attributes:error:` calls `std::fs::create_dir_all()`. Every filesystem operation is a thin translation from ObjC semantics to Rust's standard library. No magic — just plumbing.

### NSBundle

`NSBundle` is more interesting. It represents an application bundle — the `.app` directory structure that macOS uses to package executables with their resources. When a binary calls `[NSBundle mainBundle]`, it expects to get back an object that can answer questions like "what's the executable path?" and "what does Info.plist say?"

```rust
fn get_or_create_main_bundle() -> u64 {
    *MAIN_BUNDLE.get_or_init(|| {
        let obj = pal_alloc().calloc(1, 16) as u64;
        // ...
        // Derive bundle root from executable path
        let exe_path = libsystem_shims::process::process_path()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "/usr/local/bin/unknown".to_string());
        // Walk up from executable to find .app bundle root
        let bundle_root = detect_app_bundle_root(&exe_path);
        // ...
        obj
    })
}
```

The critical method is `infoDictionary` — it reads `Contents/Info.plist` from the bundle root and returns it as an `NSDictionary`. This is how `xcodebuild -version` finds the Xcode version string: it creates an `NSBundle` for `/Applications/Xcode.app`, reads `infoDictionary`, and pulls out `CFBundleShortVersionString`.

### NSPropertyListSerialization

Which brings us to plists. Apple uses property lists — XML or binary encoded files — for configuration throughout the system. `Info.plist`, `Preferences`, `.entitlements`, LaunchAgents — they're everywhere. Our implementation handles both formats:

```rust
pub fn parse_plist_file(path: &str) -> u64 {
    let data = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => return 0,
    };

    if data.starts_with(b"bplist") {
        parse_binary_plist(&data)
    } else {
        parse_xml_plist(&data)
    }
}
```

Binary plists start with the magic bytes `bplist00`. XML plists start with `<?xml`. We parse both and produce the same `NSDictionary` / `NSArray` / `NSString` object graph. The binary plist parser was particularly important — Xcode's `Info.plist` is binary format, and our initial XML-only parser returned nil for it, causing `xcodebuild -version` to print nothing instead of `Xcode 26.4`.

These three classes — NSFileManager, NSBundle, NSPropertyListSerialization — form Foundation's bridge to the outside world. Collections and strings operate on in-memory objects. File operations connect those objects to the filesystem. Bundle operations connect them to the application structure. Plist operations connect them to Apple's universal configuration format. Together, they give the binary enough real-world access to do useful work: find its own resources, read its own configuration, inspect the filesystem it's running on.

---

## Scene 5.6: NSURLSession

Some macOS tools talk to the network. `notarytool` submits binaries to Apple's notarization service. `altool` uploads apps to App Store Connect. `xcodebuild` checks developer account credentials. When they do, they use `NSURLSession` — Apple's HTTP client API.

Apple's `NSURLSession` is asynchronous by design. You create a session, create a data task from a request, provide a completion handler block, and call `resume`. The task executes on a background dispatch queue, and when the response arrives, your block gets called with the data, response, and error. It's elegant. It's also nightmarishly complex to replicate — dispatch queues, block invocation, callback scheduling, connection pooling, TLS certificate validation, HTTP/2 multiplexing, cookie storage, authentication challenges.

We don't replicate it. We simplify it:

```rust
//! NSURLSession layout: ISA(8) + config_ptr(8) = 16 bytes
//! NSURLSessionDataTask layout: ISA(8) + request_ptr(8) + state(8) + block_ptr(8) = 32 bytes

fn http_agent() -> &'static ureq::Agent {
    static HTTP_AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    HTTP_AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent()
    })
}
```

The `ureq` crate is a synchronous HTTP client built on `rustls` for TLS. No async runtime, no connection pool management, no dispatch queues. When a task's `resume` method is called, we perform the HTTP request synchronously and immediately invoke the completion handler with the result:

```rust
pub extern "C" fn shim_nsurlsessiondatatask_resume(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }

    let request = unsafe { *((obj as usize + 8) as *const u64) };
    let block_ptr = unsafe { *((obj as usize + 24) as *const u64) };

    // Extract URL and method from NSURLRequest
    let url_str = nsurlrequest::get_request_url_string(request);
    let method = nsurlrequest::get_request_method(request);

    // Perform synchronous HTTP request
    let result = perform_http_request(&url_str, &method, /* body */);

    match result {
        Ok((data, response)) => {
            if block_ptr != 0 {
                invoke_completion_handler(block_ptr, data, response, 0);
            }
        }
        Err(e) => {
            let error = nserror::make_nserror("NSURLErrorDomain", -1, &e);
            if block_ptr != 0 {
                invoke_completion_handler(block_ptr, 0, 0, error);
            }
        }
    }

    0
}
```

The simplification is deliberate. Apple's `NSURLSession` exposes dozens of delegate callbacks, authentication challenges, background transfers, upload tasks, download tasks, WebSocket tasks. Our corpus tools use exactly one pattern: create a request, get a response. `ureq` handles that pattern in a single function call. The 95% case works. The 5% case — streaming downloads, custom authentication, certificate pinning — would require a different design. But the tools we need to run don't use those features.

This is a pattern that recurs throughout Foundation shimming. Apple's API surfaces are enormous. Our implementations are minimal. We implement what binaries *use*, not what the API *defines*. The gap between the two is usually 80% or more of the API surface. That gap is how 93 classes become feasible — we're implementing the *used* subset of each class, not the full class.

The shared `ureq::Agent` is a small but important detail. Creating a new TLS connection for every HTTP request adds hundreds of milliseconds of handshake overhead. The `OnceLock<Agent>` means connection pooling happens automatically — subsequent requests to the same host reuse the TLS session. One `OnceLock`. Free connection pooling. Sometimes the simplification makes things *better*.

Real HTTP GET and POST through Foundation's `NSURLSession` API works. A macOS binary can call `[[NSURLSession sharedSession] dataTaskWithURL:completionHandler:]`, and real bytes flow over the network. We've crossed from simulation into interoperability — the binary isn't just manipulating synthetic objects in memory, it's talking to real servers.

---

## Scene 5.7: NSTask and Process Control

`NSTask` is the hardest Foundation class. Not the most complex API — `NSString` has more methods. Not the most subtle semantics — `NSDictionary`'s key equality rules are trickier. `NSTask` is the hardest because it manages a full subprocess lifecycle: spawn a child process, wire up its stdin/stdout/stderr, pass it environment variables, set its working directory, wait for it to exit, and retrieve its termination status. Every step involves real operating system resources — file descriptors, process IDs, signal handling, pipe buffers.

The object layout tells the story:

```rust
//! NSTask layout (72 bytes):
//!   offset  0: isa              (u64)
//!   offset  8: launch_path_ptr  (u64 — NSString)
//!   offset 16: arguments_ptr    (u64 — NSArray of NSString)
//!   offset 24: environment_ptr  (u64 — NSDictionary, 0=inherit)
//!   offset 32: stdout_pipe_ptr  (u64 — NSPipe/NSFileHandle)
//!   offset 40: exit_status      (i32, valid after waitUntilExit)
//!   offset 44: pid              (i32)
//!   offset 48: stdin_pipe_ptr   (u64 — NSPipe/NSFileHandle)
//!   offset 56: stderr_pipe_ptr  (u64 — NSPipe/NSFileHandle)
//!   offset 64: current_dir_ptr  (u64 — NSString)
```

Seventy-two bytes. Nine fields. Each one is a piece of state that the binary's code sets before launch, querying some of them after exit. The pattern is always the same:

```objc
NSTask *task = [[NSTask alloc] init];
[task setLaunchPath:@"/usr/bin/xcrun"];
[task setArguments:@[@"--show-sdk-path"]];
[task setStandardOutput:pipe];
[task launch];
[task waitUntilExit];
NSData *output = [[pipe fileHandleForReading] readDataToEndOfFile];
```

The binary creates the task, configures it, launches it, waits, and reads the output. Our `launch` implementation uses `posix_spawn`:

```rust
pub extern "C" fn shim_nstask_launch(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }

    let launch_path = get_string_field(obj, LAUNCH_PATH_OFF);
    let args = collect_arguments(obj);
    let env = collect_environment(obj);
    let cwd = get_string_field(obj, CURRENT_DIR_OFF);

    // Set up posix_spawn file actions for pipe redirection
    let mut file_actions = std::mem::MaybeUninit::uninit();
    unsafe { libc::posix_spawn_file_actions_init(file_actions.as_mut_ptr()); }

    // Wire stdout pipe: dup2 write-end to fd 1
    let stdout_pipe = unsafe { *((obj as usize + STDOUT_OFF) as *const u64) };
    if stdout_pipe != 0 {
        let write_fd = nspipe::pipe_write_fd(stdout_pipe);
        unsafe {
            libc::posix_spawn_file_actions_adddup2(
                file_actions.as_mut_ptr(), write_fd, libc::STDOUT_FILENO
            );
        }
    }

    // Similarly for stdin and stderr...

    // Set working directory if specified
    if let Some(dir) = cwd {
        unsafe {
            libc::posix_spawn_file_actions_addchdir_np(
                file_actions.as_mut_ptr(), dir.as_ptr()
            );
        }
    }

    // Spawn the child process
    let mut pid: libc::pid_t = 0;
    let result = unsafe {
        libc::posix_spawn(
            &mut pid, launch_path.as_ptr(),
            file_actions.as_ptr(), std::ptr::null(),
            argv.as_ptr(), envp.as_ptr(),
        )
    };

    if result == 0 {
        unsafe { *((obj as usize + PID_OFF) as *mut i32) = pid; }

        // Parent must close write-end of stdout/stderr pipes
        // so readers see EOF when child exits
        close_parent_pipe_ends(obj);
    }

    0
}
```

There's a critical detail that took two debugging sessions to find: the parent process must close its copies of the pipe write-ends after spawning the child. If the parent keeps the write-end of the stdout pipe open, `readDataToEndOfFile` on the read-end will never see EOF — it blocks forever, waiting for more data that will never come. The child exits, but the pipe stays open because the parent still holds a file descriptor to the write-end. The fix is a single line — `close_parent_pipe_ends(obj)` — but finding it required tracing why `xcrun --show-sdk-path` hung indefinitely.

`waitUntilExit` calls `waitpid`:

```rust
pub extern "C" fn shim_nstask_wait_until_exit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let pid = unsafe { *((obj as usize + PID_OFF) as *const i32) };
    if pid <= 0 { return 0; }

    let mut status: i32 = 0;
    unsafe { libc::waitpid(pid, &mut status, 0); }

    let exit_code = if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        -1
    };

    unsafe { *((obj as usize + EXIT_STATUS_OFF) as *mut i32) = exit_code; }
    0
}
```

And `NSPipe` wraps a Unix pipe:

```rust
pub fn alloc_pipe() -> u64 {
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return 0;
    }
    let obj = pal_alloc().calloc(1, 24) as u64;
    // Store read fd at offset 8, write fd at offset 16
    unsafe {
        *(obj as *mut u64) = nsobj::nspipe_class_addr();
        *((obj as usize + 8) as *mut i32) = fds[0];
        *((obj as usize + 16) as *mut i32) = fds[1];
    }
    obj
}
```

`NSTask` is peak complexity for Foundation shimming because it's where our synthetic framework touches the most real operating system surface. We're not manipulating in-memory objects. We're creating real processes with real file descriptors connected to real pipes. The NSPipe object wraps `pipe(2)`. The launch method calls `posix_spawn(2)`. The wait method calls `waitpid(2)`. The output comes from `read(2)` on a file descriptor. Every step is a syscall, and syscalls carry their own failure modes — `EMFILE` (too many open files), `ECHILD` (no child processes), `EINTR` (signal interrupted), `EPIPE` (broken pipe). The surface area isn't just the ObjC methods — it's the union of the ObjC API and the POSIX API underneath it.

The `subprocess_capture` and `xcrun_query` corpus tools exercise NSTask end-to-end: create task, set launch path, set arguments, attach pipe, launch, wait, read output, check exit status. Both pass. Real subprocess execution through Foundation's API, with real pipe I/O, on a synthetic framework built from Rust shims. Peak difficulty. Peak satisfaction.

---

## Scene 5.8: Toll-Free Bridging

There's an architectural mystery at the heart of Apple's frameworks. Core Foundation is a C API: `CFStringRef`, `CFArrayRef`, `CFDictionaryRef` — opaque types manipulated through C functions. Foundation is an ObjC API: `NSString`, `NSArray`, `NSDictionary` — classes with methods you call through messages. Two APIs. Two naming schemes. Two programming models.

And they're the same objects.

`CFStringRef` and `NSString *` are the same pointer. You can pass a `CFStringRef` to any function that expects `NSString *`, and vice versa, with a simple cast:

```objc
CFStringRef cfStr = CFStringCreateWithCString(NULL, "hello", kCFStringEncodingUTF8);
NSString *nsStr = (__bridge NSString *)cfStr;
NSUInteger len = [nsStr length];  // works — same object
```

This is toll-free bridging. Apple's deepest architectural decision: the C and ObjC layers share a single object representation. A `CFStringRef` has an ISA pointer at offset 0, flags at offset 8, a character buffer at offset 16, and a length at offset 24. It's a valid ObjC object. You can send it messages. You can put it in an `NSArray`. You can pass it to `NSLog(@"%@", cfStr)`. The runtime sees the ISA, dispatches the method, and everything works — because the memory layout is identical whether you call it `CFStringRef` or `NSString *`.

For our loader, this design choice is a gift. We don't need to implement two separate string types. One implementation — the 32-byte `__CFConstantString` layout — serves both the C and ObjC APIs. `CFStringGetLength()` and `-[NSString length]` read the same offset. `CFStringGetCStringPtr()` and `-[NSString UTF8String]` extract the same pointer. The ISA makes the object dispatch-capable for ObjC; the flags and data make it accessible for C.

The implementation is straightforward: we assign the same ISA to both APIs:

```rust
/// `___CFConstantStringClassReference` — used by @"..." literals AND CFStringCreate*
pub fn cfconstant_string_class_addr() -> u64 {
    CFSTRING_INIT.call_once(|| {
        unsafe {
            let ptr = CFCONSTANT_STRING_CLASS.0.get() as *mut u8;
            let self_addr = ptr as u64;
            // ISA points to self — this IS the class object
            std::ptr::copy_nonoverlapping(self_addr.to_le_bytes().as_ptr(), ptr, 8);
        }
    });
    CFCONSTANT_STRING_CLASS.0.get() as u64
}
```

The class address for `___CFConstantStringClassReference` is the same address we register as `__NSCFConstantString` in the ObjC runtime. One class. Two names. Same ISA. When a binary creates a string through `CFStringCreateWithCString`, it gets an object with this ISA. When a binary writes `@"hello"`, the compiler emits a struct with this ISA. Either way, the ObjC runtime finds `__NSCFConstantString` in the class table and dispatches from its method list.

The same principle extends to every toll-free bridged pair:

| CoreFoundation Type | Foundation Type | Shared Layout |
|---------------------|----------------|---------------|
| CFStringRef | NSString * | 32 bytes: ISA + flags + buffer + length |
| CFArrayRef | NSArray * | 24 bytes: ISA + count + elements_ptr |
| CFDictionaryRef | NSDictionary * | 32 bytes: ISA + count + keys + values |
| CFDataRef | NSData * | 24 bytes: ISA + length + bytes_ptr |
| CFURLRef | NSURL * | 24 bytes: ISA + string + baseURL |
| CFBooleanRef | NSNumber * | 24 bytes: ISA + value + type_tag |

For each pair, we implement the C API functions (`CFArrayGetCount`, `CFDictionaryGetValue`, `CFDataGetBytePtr`) as thin readers of the same memory layout that the ObjC shims read. The C functions don't go through `objc_msgSend` — they don't need to. They read the struct directly. The ObjC methods go through dispatch and eventually call the same reading logic.

This is the elegance revelation that design determines what's possible. Apple's decision to use the same memory layout for both APIs means we only build each object once. If they'd used separate representations — a C struct for `CFStringRef` and an ObjC object for `NSString` — we'd need conversion functions, reference counting synchronization, and two code paths for everything. Toll-free bridging eliminates all of that. One object, two interfaces, zero conversion overhead.

We register the Core Foundation functions in the shim table alongside the Foundation class methods:

```rust
register_shims!(table,
    ("_CFStringCreateWithCString", cfstring::shim_cf_string_create_with_cstring as u64, "CFStringCreateWithCString"),
    ("_CFStringGetLength", cfstring::shim_cf_string_get_length as u64, "CFStringGetLength"),
    ("_CFStringGetCStringPtr", cfstring::shim_cf_string_get_cstring_ptr as u64, "CFStringGetCStringPtr"),
    ("_CFArrayGetCount", cf_core::shim_cf_array_get_count as u64, "CFArrayGetCount"),
    ("_CFArrayGetValueAtIndex", cf_core::shim_cf_array_get_value_at_index as u64, "CFArrayGetValueAtIndex"),
    ("_CFDictionaryGetValue", cf_core::shim_cf_dictionary_get_value as u64, "CFDictionaryGetValue"),
    ("_CFDataGetBytePtr", cf_core::shim_cf_data_get_byte_ptr as u64, "CFDataGetBytePtr"),
    ("_CFDataGetLength", cf_core::shim_cf_data_get_length as u64, "CFDataGetLength"),
    ("_CFRetain", cfstring::shim_cf_release as u64, "CFRetain"),    // no-op for now
    ("_CFRelease", cfstring::shim_cf_release as u64, "CFRelease"),  // ref-counted
    ("_CFGetTypeID", cf_core::shim_cf_get_type_id as u64, "CFGetTypeID"),
    // ... 40+ more CF functions
);
```

Each CF function resolves to a Rust function that reads the shared object layout. No dispatch overhead. No class lookup. Just offset arithmetic on the same 24-32 byte structs that Foundation methods operate on. Two APIs, one truth.

The `CFGetTypeID` function is worth noting:

```rust
pub extern "C" fn shim_cf_get_type_id(obj: u64) -> u64 {
    if obj == 0 { return 0; }
    let isa = unsafe { *(obj as *const u64) };

    if isa == nsobj::nsstring_class_addr() || isa == cfconstant_string_class_addr() {
        return 7; // kCFStringTypeID
    }
    if isa == nsobj::nsarray_class_addr() { return 19; }
    if isa == nsobj::nsdictionary_class_addr() { return 18; }
    if isa == nsobj::nsdata_class_addr() { return 15; }
    if isa == nsobj::nsurl_class_addr() { return 29; }
    if isa == nsobj::nsnumber_class_addr() { return 22; }
    // ...
    0
}
```

It reads the ISA and maps it to the same type ID constants that Apple's Core Foundation uses. Binary code that calls `CFGetTypeID(obj)` and switches on the result gets the same type IDs it would on macOS. Toll-free bridging doesn't just share layout — it shares identity.

---

## Scene 5.9: The 2,682 Tests

Let's count.

The `cf-lite` crate — our Foundation implementation — has 2,682 unit tests. Not integration tests that run whole binaries. Not fuzzing harnesses that exercised random inputs. Unit tests. Each one constructs an object, calls one or more methods, and asserts the result matches Apple's behavior.

Here's what that looks like for NSString:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nsstring_length_ascii() {
        let s = make_test_nsstring("hello");
        assert_eq!(shim_nsstring_length(s, 0), 5);
    }

    #[test]
    fn test_nsstring_length_emoji() {
        let s = make_test_nsstring("hello 👋");
        // length returns UTF-16 code unit count: 6 ASCII + 2 for surrogate pair
        assert_eq!(shim_nsstring_length(s, 0), 8);
    }

    #[test]
    fn test_nsstring_utf8string_roundtrip() {
        let s = make_test_nsstring("café");
        let ptr = shim_nsstring_utf8string(s, 0);
        let result = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        assert_eq!(result, "café");
    }

    #[test]
    fn test_nsstring_nil_receiver() {
        assert_eq!(shim_nsstring_length(0, 0), 0);
        assert!(shim_nsstring_utf8string(0, 0).is_null());
    }
}
```

Each test module follows the same pattern: create objects using test helpers, call shim functions, assert expected results. The test helpers (`shim_test_helpers.rs`) provide convenience constructors — `make_test_nsstring`, `make_test_nsarray`, `make_test_nsdictionary` — that allocate objects in the same layout the real shims use. Every test runs in isolation. Every test cleans up after itself.

The test counts by domain:

| Module | Test Count | What They Cover |
|--------|-----------|-----------------|
| cfstring/nsstring | 340+ | length, UTF8String, compare, format, search, path ops, URL encoding |
| nsarray | 180+ | creation, access, enumeration, sorting, filtering, description |
| nsdictionary | 160+ | creation, lookup, enumeration, key types, description |
| nsmutablearray | 120+ | add, remove, insert, replace, sort, filter mutation |
| nsmutabledictionary | 90+ | set, remove, merge, copy |
| nsdata | 85+ | creation, bytes, subdata, base64, isEqual |
| nsset/nsmutableset | 110+ | add, contains, union, intersection, minus, enumerate |
| nsnumber | 95+ | integer, float, double, bool, compare, description |
| nsdate/nscalendar | 75+ | creation, intervals, components, formatting |
| nsfilemanager | 60+ | exists, attributes, create, remove, move, copy |
| nsurl | 165+ | creation, components, file URLs, relative, resolve |
| nsregularexpression | 55+ | match, replace, capture groups, fast enumeration |
| nstask | 45+ | launch, pipe, waitUntilExit, exit status, environment |
| nsurlsession | 35+ | GET, POST, response parsing, status codes |
| nsjsonserialization | 50+ | parse, generate, nested, unicode |
| format module | 85+ | %@, %d, %s, %f, %x, width, precision, flags |
| all other modules | 930+ | the remaining 60+ classes |

These tests are not aspirational. They're regression gates. Every change to `cf-lite` runs the full suite. Every pull request must pass all 2,682. The tests are the specification — if Apple's Foundation returns a specific value for a specific input, our test encodes that expectation, and our shim delivers it.

The test-driven development loop for each class follows a predictable cycle:

1. **Find a binary that uses the class.** Scan corpus tools or Xcode binaries for the selector.
2. **Run the binary. Watch it crash or return garbage.** The crash tells you which method is missing.
3. **Write the test first.** Construct the object, call the method, assert what Apple would return.
4. **Implement the shim.** Make the test pass.
5. **Run the binary again.** It gets further. Hits the next missing method. Repeat.

This cycle repeated over a thousand times across the 93 classes. Each iteration added one method, one test, one small step toward compatibility. The rhythm is relentless — run, crash, investigate, implement, test, run, crash again at the next call site. The total test count is the accumulated evidence of that grinding, incremental work.

Beyond unit tests, we have the corpus: 39 command-line tools compiled for macOS, each exercising a different slice of Foundation. They're integration tests at the binary level — real macOS executables running on our synthetic Foundation:

```
$ macho-inspect --exec corpus/string_processor/string_processor     → 12/12 PASS
$ macho-inspect --exec corpus/json_formatter/json_formatter         → 8/8 PASS
$ macho-inspect --exec corpus/date_calculator/date_calculator       → 10/10 PASS
$ macho-inspect --exec corpus/file_lister/file_lister               → 7/7 PASS
$ macho-inspect --exec corpus/url_builder/url_builder               → 9/9 PASS
$ macho-inspect --exec corpus/regex_search/regex_search             → 6/6 PASS
$ macho-inspect --exec corpus/subprocess_capture/subprocess_capture → 7/7 PASS
$ macho-inspect --exec corpus/kv_store/kv_store                     → 8/8 PASS
$ macho-inspect --exec corpus/collection_pipeline/collection_pipeline → 11/11 PASS
$ macho-inspect --exec corpus/xml_parse/xml_parse                   → 6/6 PASS
...
39/39 corpus tools pass
```

Every path through Foundation that these tools exercise — string formatting, JSON parsing, file operations, date arithmetic, URL construction, regex matching, subprocess execution, collection manipulation, XML parsing — works. Not approximately. Exactly. The outputs match what the binaries produce when run natively on macOS.

Let's put the numbers together. We started this chapter with `[NSString stringWithFormat:]` returning nil. We now have:

- **93+ synthetic Foundation classes** registered in the ObjC runtime
- **1,129 unique selectors** wired to Rust shim implementations
- **2,682 unit tests** passing in `cf-lite`
- **39 corpus tools** running correctly against the synthetic Foundation
- A cached template system that registers all 93 classes in **11 microseconds**

Each class is a full behavioral implementation — not a stub that returns zero, but a functioning replica that allocates objects, manages memory, performs operations, and returns correct results. NSString formats. NSArray enumerates. NSDictionary looks up keys. NSFileManager reads directories. NSTask spawns processes. NSURLSession makes HTTP requests. NSJSONSerialization parses JSON. The binary's expectations are met.

The runtime dispatches. Foundation delivers. The engine has fuel.

---

Everything works.

`xcodebuild -version` prints `Xcode 26.4 / Build version 17E192`. The version string flows through NSBundle reading Info.plist, NSPropertyListSerialization parsing binary plist, NSDictionary looking up `CFBundleShortVersionString`, NSString formatting the output. Every link in that chain is our code — synthetic classes with Rust implementations behind ObjC selectors.

The corpus tools run on macOS. Real macOS binaries, compiled by Apple's compiler, linked against Apple's SDK, running on our loader with our Foundation. 39 tools. Hundreds of checks. All green.

We've earned this. Ninety-three classes, each one debugged method by method, test by test, crash by crash. The false victory of Scene 5.3 — one class works! — gave way to the grinding montage of Scenes 5.4 through 5.8, each one adding another layer of capability, each one revealing another corner of the platform's surface area. The true victory is not that one class works. The true victory is that they all work *together* — strings flow into arrays, arrays flow into dictionaries, dictionaries flow into JSON serialization, JSON flows into URL requests, URL responses flow back into strings. The whole graph holds.

Time to ship it.

Build the Docker image. Copy the corpus tools. Run the same tests.

```
$ docker run --rm burdettie-os macho-inspect --exec /opt/corpus/string_processor/string_processor
SIGSEGV at PC 0xffffb3e1e808
```

Crash.

Not just one tool. All of them. Every tool that touches `stat`, `printf`, `readdir`, or locale settings. The Foundation shims work, but the C library layer underneath them assumes macOS struct layouts. On Linux, `struct stat` is 128 bytes, not 144. `st_size` lives at offset 48, not 96. The binary reads offset 96, gets a timestamp instead of a file size, tries to allocate 1.7 gigabytes, and crashes.

Everything works on macOS. Nothing works on Linux. Same CPU. Same instructions. Different ABI. Different universe.

That's Chapter 6.
