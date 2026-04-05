# Chapter 4: The Runtime

## Scene 4.1: The ObjC Object Model

C functions work. The linker is proven. `strings` reads files, `nm` dumps symbol tables, `size` reports segment sizes. Every GOT slot filled, every chained fixup resolved, every PAC signature applied. The loader handles C-level binaries with full fidelity.

Now try something that matters:

```
$ macho-inspect --exec /opt/xcode-tools/xcode-tc/xcodebuild -version
SIGSEGV at PC 0x00000001a42f8c30
  x0  = 0x0000000100010000
  x1  = 0x00000001a4250038   ("init")
  x16 = 0x0000000000000000
```

The first ObjC call crashes. `x0` holds a receiver — an object pointer. `x1` holds a selector — a C string pointer to `"init"`. `x16` is zero, which means `objc_msgSend` tried to branch to a null function pointer. The message dispatch function resolved the selector to... nothing.

This isn't a linking failure. The GOT entry for `_objc_msgSend` was filled correctly — our resolver found it in `libobjc.A.dylib` (or, more precisely, in our shim table). The crash happens *inside* the dispatch. The runtime tried to look up the method `init` on a class and found no method table, no class metadata, no runtime state at all. Because we never built one.

On macOS, the Objective-C runtime is initialized by `dyld` during the `map_images` notification — long before `main()` runs. Every class in every loaded image gets registered in a global table. Every selector gets interned. Every category gets merged. By the time your code calls `[NSObject init]`, the runtime has already indexed every class, built every method cache, and wired up every superclass chain. It's invisible infrastructure. You never think about it because it always works.

We don't have Apple's ObjC runtime. We're on Linux. There is no `libobjc.A.dylib` in the shared cache waiting to be called. There's no `_objc_init` that will magically populate the class tables. If we want `objc_msgSend` to dispatch correctly, we need to build the runtime from scratch.

This is the B story that's been hiding inside the binary. Chapters 1 through 3 dealt with the A story — parsing, mapping, linking. The mechanical infrastructure of loading. But inside every non-trivial macOS binary, there's an entirely separate system that needs initialization: the Objective-C runtime. It has its own data sections (`__objc_classlist`, `__objc_selrefs`, `__objc_catlist`, `__objc_protolist`). Its own initialization sequence (register classes, intern selectors, merge categories, run `+load`). Its own dispatch mechanism (`objc_msgSend`). The A story gets the binary into memory. The B story makes it *think*.

To build it, we first need to understand what ObjC objects actually *are* at the binary level. Not what the API tells you they are. What the bytes say.

### Objects Are Memory Layouts

Every Objective-C object — every `NSString`, every `NSArray`, every `UIView`, every custom class you've ever written — has the same fundamental structure in memory. The first eight bytes of every object are a pointer to its class. This pointer is called the ISA.

```
Object at 0x100010000:
  +0x00: 0x00000001a4230100   ← ISA (points to class metadata)
  +0x08: ...                  ← instance variables start here
```

That's it. An ObjC object is a blob of memory whose first word is a pointer to its class. Everything else — method dispatch, type checking, `respondsToSelector:`, `isKindOfClass:`, the entire dynamic dispatch machinery — flows from that single pointer.

The ISA doesn't point to another object. It points to a `class_t` structure — a C struct that the ObjC runtime uses to describe everything about a class: its name, its methods, its instance variables, its superclass. Here's what `class_t` looks like in memory:

```
class_t at 0x00000001a4230100:
  +0x00: ISA            (u64) → metaclass pointer
  +0x08: superclass     (u64) → parent class_t
  +0x10: cache          (u64) → method cache (buckets + mask)
  +0x18: cache_mask     (u64) → cache hash mask
  +0x20: data           (u64) → class_ro_t pointer (read-only data)
```

Let's draw the full ISA and superclass relationships for a simple three-class hierarchy:

```
Instance of Dog
  ISA → Dog (class)
           ISA → Dog (metaclass)
                    ISA → NSObject (metaclass)
                             ISA → NSObject (metaclass) ← points to itself
           superclass → Animal (class)
                           superclass → NSObject (class)
                                          superclass → nil (root)
```

The ISA chain goes: object → class → metaclass → root metaclass → self. The superclass chain goes: class → parent class → ... → NSObject → nil. Method dispatch walks the superclass chain for instance methods and the metaclass superclass chain for class methods. The two chains form a diamond pattern that Brad Cox designed in the early 1980s and Apple has preserved unchanged ever since.

Why metaclasses? Because in ObjC, classes are objects too. You can send messages to a class — `[NSString alloc]`, `[NSArray array]`, `[NSDate date]`. For messages to classes to work, classes need to have an ISA that points to something with a method list. That something is the metaclass. The metaclass of `NSString` holds `+alloc`, `+new`, `+string`, and every other class method. When you send a message to `NSString`, `objc_msgSend` reads `NSString`'s ISA (which points to the `NSString` metaclass), and dispatches from there. Classes and metaclasses are parallel hierarchies that mirror each other.

The `class_t` has its *own* ISA — pointing to the *metaclass*. The metaclass holds class methods (methods you call on the class itself, like `[NSObject alloc]`). The metaclass's ISA points to the root metaclass, which points to itself. It's turtles, but not all the way down — the chain terminates.

The `data` field at offset `+0x20` points to `class_ro_t` — the read-only class data. This is where the substance lives:

```
class_ro_t:
  +0x00: flags          (u32) → RO_META, RO_ROOT, etc.
  +0x04: instance_start (u32) → offset of first ivar
  +0x08: instance_size  (u32) → total object size in bytes
  +0x10: name           (u64) → pointer to class name C string
  +0x18: base_methods   (u64) → pointer to method_list_t
  +0x20: base_protocols (u64) → pointer to protocol_list_t
  +0x28: ivars          (u64) → pointer to ivar_list_t
  +0x30: weak_ivar_layout (u64)
  +0x38: base_properties (u64) → pointer to property_list_t
```

The `name` field is a pointer to a null-terminated C string — literally the class name, like `"NSObject"` or `"NSMutableArray"`. The `base_methods` field points to a `method_list_t`, which is a count-prefixed array of method descriptors:

```
method_list_t:
  +0x00: flags_and_count (u32) → count in lower 24 bits
  +0x04: method[0].name       → selector string pointer
  +0x0C: method[0].types      → type encoding string pointer
  +0x14: method[0].imp        → function pointer (IMP)
  +0x1C: method[1].name
  ...
```

Each method entry maps a selector name to an IMP — a plain C function pointer. `"init"` maps to the address of the function that implements `-[NSObject init]`. `"alloc"` maps to the function that implements `+[NSObject alloc]`. The method list is the lookup table that `objc_msgSend` searches when it receives a message.

This is where the paradigm shift happens. If you come from a high-level ObjC or Swift background, you think of objects as API abstractions — things you interact with through methods and properties, governed by protocols and inheritance. At the binary level, they're none of that. They're memory layouts. A class is a C struct with pointers to other C structs. A method is a (name, function pointer) pair. An object is eight bytes of ISA followed by its ivars.

Consider what this means for method dispatch. When you write `[myObject doSomething]` in ObjC, the compiler doesn't generate a direct function call. It generates a call to `objc_msgSend(myObject, @selector(doSomething))`. At runtime, `objc_msgSend` reads `myObject`'s first eight bytes to get the ISA, follows the ISA to the `class_t`, follows the `data` pointer to `class_ro_t`, and searches the `base_methods` list for a method whose selector matches `"doSomething"`. If found, it calls the IMP. If not, it follows the `superclass` pointer and repeats.

That chain — object → ISA → class_t → class_ro_t → method_list_t → IMP — is the entire ObjC dispatch mechanism, and every link in it is a raw pointer in the binary's `__DATA` segment. If any pointer is wrong, dispatch fails. If the method list is malformed, dispatch fails. If the superclass chain is broken, dispatch fails. The binary must describe the full class hierarchy perfectly for any ObjC code to run.

On macOS, you never see this because `dyld` and `libobjc` conspire to make it invisible. The metadata is parsed, validated, patched, and indexed all before `main()`. By the time you call your first method, the class tables are warm and every pointer is correct. But we're building the runtime. Every pointer is our responsibility.

Let's look at this concretely by examining what a real binary contains. The `macho-objc` crate parses these structures from the `__DATA` segment:

```rust
/// A reference to an ObjC class found in __objc_classlist.
pub struct ObjcClassRef {
    pub pointer_addr: u64,
    pub class_data_addr: u64,
    pub name: Option<String>,
    pub methods: Vec<ObjcMethodInfo>,
    pub class_methods: Vec<ObjcMethodInfo>,
    pub ivars: Vec<ObjcIvarInfo>,
    pub properties: Vec<ObjcPropertyInfo>,
    pub protocol_names: Vec<String>,
    pub instance_size: u32,
    pub ro_flags: u32,
    pub superclass_addr: u64,
    pub metaclass_addr: u64,
}
```

Every field in this struct comes directly from parsing the binary's `__objc_classlist` section — following pointers from the section entries to `class_t` structs to `class_ro_t` structs to method lists, ivar lists, and property lists. The parser reads raw bytes and reconstructs the compiler's intent. No runtime API. No reflection. Just bytes.

The `ro_flags` field tells us two critical things. Bit 0 (`RO_META`) indicates this is a metaclass — it holds class methods, not instance methods. Bit 1 (`RO_ROOT`) indicates this is a root class — it has no superclass. `NSObject` is the only root class in practice (well, `NSObject` and `NSProxy`, if you're feeling pedantic). Every other class has a superclass, forming a tree rooted at `NSObject`.

This tree matters because method dispatch walks it. When you send `init` to an `NSMutableArray`, the runtime first checks `NSMutableArray`'s method list. If `init` isn't there, it checks `NSArray`. Then `NSObject`. The superclass chain is the fallback path — and building it correctly is essential for dispatch to work.

The `instance_size` field tells the allocator how many bytes to reserve when creating an object of this class. `NSObject` has an `instance_size` of 8 — just the ISA pointer, nothing else. `NSMutableArray` might be 32 or 48 bytes, depending on the version. When `objc_msgSend` dispatches `alloc`, the implementation reads this field to know how much memory to request from the heap.

Here's the core insight: all of this — the ISA chains, the method lists, the superclass tree, the instance sizes — is encoded in the Mach-O binary's `__DATA` and `__DATA_CONST` segments. The compiler emits it. The linker lays it out. The runtime reads it. And now we need to read it too, parse it into our own data structures, and build a class table that `objc_msgSend` can search.

The binary tells the truth about ObjC in a way that headers and documentation never do. The header `<objc/runtime.h>` gives you opaque types — `Class`, `SEL`, `IMP`, `id`. The binary gives you offsets and sizes and pointer chains. The header says "a class is a Class." The binary says "a class is 40 bytes starting with an ISA pointer, followed by a superclass pointer, followed by a cache, followed by a data pointer that leads to a name string and a method list." One is an abstraction. The other is the implementation.

We're building a runtime. We need the implementation.

---

## Scene 4.2: Selector Interning

Before we can dispatch messages, we need to solve a seemingly simple problem: how do you compare method names efficiently?

Every ObjC method call specifies a selector — the name of the method being invoked. When you write `[obj init]`, the compiler emits a call to `objc_msgSend(obj, @selector(init))`. The second argument is a `SEL` — a selector. And a `SEL` is just a pointer to a C string.

But not *any* C string pointer. A uniqued one.

The `__objc_selrefs` section in a Mach-O binary contains pointers to selector strings. Every selector the binary uses — `init`, `alloc`, `dealloc`, `stringWithFormat:`, `addObject:`, `count` — has an entry in this section. At compile time, these point to string literals in `__objc_methnames`:

```
__TEXT,__objc_methnames:
  0x100002000: "init\0"
  0x100002005: "alloc\0"
  0x10000200b: "dealloc\0"
  0x100002013: "stringWithFormat:\0"
  ...

__DATA,__objc_selrefs:
  +0x00: 0x0000000100002000   → "init"
  +0x08: 0x0000000100002005   → "alloc"
  +0x10: 0x000000010000200b   → "dealloc"
  +0x18: 0x0000000100002013   → "stringWithFormat:"
  ...
```

On macOS, during `map_images`, the ObjC runtime *rewrites* every entry in `__objc_selrefs`. It replaces each pointer with a pointer to the runtime's own interned copy of that string. After fixup, every `"init"` pointer in every loaded image points to the *same* address. Not the same string — the same *pointer value*.

This is selector interning, and it's the key to fast dispatch. With interning, method lookup can compare selectors using pointer equality (`sel_a == sel_b`) instead of string comparison (`strcmp(sel_a, sel_b) == 0`). Pointer comparison is a single CPU instruction. String comparison is a loop. For a function that gets called millions of times per second — which `objc_msgSend` does — that difference is everything.

Think about the scale. A typical macOS application loads dozens of frameworks, each containing hundreds of selectors. Many of those selectors are the same: `init`, `dealloc`, `description`, `isEqual:`, `hash`, `copy`, `mutableCopy`. Without interning, every message dispatch would require a `strcmp` against every method in the class's method list. With interning, it requires a pointer comparison — and since our `method_index` HashMap uses the selector string as a key, the lookup is amortized O(1).

The Apple runtime's interning is even more aggressive than ours. Apple's `SEL` type is defined as `const char *`, but the runtime guarantees that any two `SEL` values for the same selector name are the same pointer. This invariant is so strong that Apple's `objc_msgSend` uses it in the method cache: the cache is a hash table keyed by `SEL` values, and the lookup uses pointer equality for both the hash probe and the comparison. No string hashing, no string comparison, ever, on the hot path.

Our implementation mirrors this design. The `ObjcRuntime` struct maintains a selector table:

```rust
/// Intern a selector string, returning a stable pointer to the C string.
pub fn intern_selector(&mut self, name: &str) -> usize {
    if let Some(selector) = self.selectors.get(name) {
        return selector.as_ptr() as usize;
    }
    let selector = Arc::new(
        std::ffi::CString::new(name)
            .unwrap_or_else(|_| std::ffi::CString::new("").unwrap()),
    );
    let ptr = selector.as_ptr() as usize;
    self.selectors.insert(Arc::from(name), selector);
    ptr
}
```

The first time `intern_selector("init")` is called, it allocates a `CString`, stores it in a `HashMap`, and returns the raw pointer. Every subsequent call with `"init"` returns the same pointer. The `Arc<CString>` ensures the string lives as long as the runtime — the pointer is stable for the lifetime of the process.

After class registration, we fix up the binary's `__objc_selrefs` section in mapped memory:

```rust
pub fn fixup_selrefs(&mut self, metadata: &ObjcMetadata, slide: i64) {
    let mut fixed = 0usize;
    for (name, &vmaddr) in metadata.selectors.iter().zip(metadata.selref_addrs.iter()) {
        let slot_addr = (vmaddr as i64 + slide) as u64;
        let interned = self.intern_selector(name);
        unsafe {
            let ptr = slot_addr as *mut u64;
            ptr.write(interned as u64);
        }
        fixed += 1;
    }
}
```

For each selector reference in the binary, we compute its runtime address (applying the ASLR slide), intern the selector name, and write the interned pointer into the selref slot. After this pass, the binary's `__objc_selrefs` section contains pointers to our runtime's interned strings. When the binary's code loads a selector via `@selector(init)`, it reads from `__objc_selrefs` and gets back a pointer that matches what every other image uses for `"init"`.

The elegance of this design is that it converts string comparison into pointer comparison at load time, once. Every subsequent dispatch — and there will be millions — benefits. It's a classic time-space tradeoff: spend a few microseconds during loading to save nanoseconds on every message send forever after.

`macho-objc` parses the selector data from the binary:

```rust
pub fn parse_objc_metadata(parsed: &ParsedMachO, data: &[u8]) -> ObjcMetadata {
    // ...
    let (selectors, selref_addrs) = parse_selrefs(parsed, data);
    // ...
}
```

The `selectors` vector contains the string names. The `selref_addrs` vector contains the virtual addresses of the slots in `__objc_selrefs`. Together, they're the recipe for fixup: for each `(name, addr)` pair, write the interned pointer to the address.

A small example to make this concrete. Suppose two images both use `@selector(init)`:

```
Image A __objc_selrefs:
  slot at 0x100004000 → "init" (in Image A's __objc_methnames)

Image B __objc_selrefs:
  slot at 0x200004000 → "init" (in Image B's __objc_methnames)
```

Before interning, slot A and slot B point to different addresses — each image has its own copy of the string `"init"`. After interning:

```
Image A __objc_selrefs:
  slot at 0x100004000 → 0x0000600000001000 (runtime's CString)

Image B __objc_selrefs:
  slot at 0x200004000 → 0x0000600000001000 (same pointer!)
```

Both slots now contain the same value. The method lookup in `objc_msgSend` can compare the `SEL` from the call site directly against the `SEL` stored in each method entry. One comparison, one instruction, O(1). That's the payoff of interning.

---

## Scene 4.3: Message Dispatch

With the object model understood and selectors interned, we can tackle the core algorithm of the Objective-C runtime: message dispatch.

Every ObjC method call compiles to a call to `objc_msgSend`. Every `[receiver method]` becomes `objc_msgSend(receiver, @selector(method), args...)`. This is not a detail — it's the entire execution model. ObjC doesn't have function calls in the C++ sense. It has *messages*. The distinction matters more than you might expect.

In C++, a virtual method call goes through a vtable. The compiler knows the object's vtable layout at compile time. A call to `obj->doSomething()` compiles to something like:

```
ldr    x8, [x0]           ; load vtable pointer from object
ldr    x8, [x8, #48]      ; load method pointer at known offset
blr    x8                  ; call the method
```

Three instructions. The vtable offset `#48` is fixed at compile time — `doSomething` is always at position 6 in the vtable (6 × 8 = 48). The compiler can emit this because C++ class hierarchies are frozen at compile time. You cannot add methods to a class after compilation. You cannot change which method a vtable slot points to. The layout is static.

This static binding has consequences. In C++, adding a virtual method to a base class changes the vtable layout, breaking every subclass's binary compatibility. This is the "fragile base class problem," and it's why C++ has strict ABI rules about class hierarchy changes. Adding a virtual method requires recompiling every subclass. Removing one is even worse.

ObjC sidesteps this entirely. There are no vtable offsets. Methods are dispatched by name, at runtime, every time. Adding a method to a class — even at runtime, even from a different library — never breaks anything because nothing ever assumed a fixed offset. Method resolution is deferred until the message is sent. When you write `[obj init]`, the compiler does *not* know which function will be called. It can't. The method might come from:

- The object's direct class
- A superclass
- A category loaded at runtime
- A method added via `class_addMethod` after the class was created
- A forwarding handler installed by `forwardInvocation:`
- A `resolveInstanceMethod:` callback that lazily installs the method

The method isn't determined until `objc_msgSend` runs. This is true late binding — not the C++ kind where the binding is late relative to compile time but fixed at link time, but genuinely deferred to the instant of the call.

Here's the algorithm, step by step:

**Step 1: Read the ISA.** The first eight bytes of the receiver are the ISA pointer. On ARM64, the ISA may have PAC bits or tagged pointer bits set, so the runtime masks it:

```rust
let raw_isa = unsafe { *(obj as *const u64) };
let isa = raw_isa & 0x007ffffffffffff8;  // mask out non-pointer bits
```

The mask `0x007ffffffffffff8` strips the top bit (tagged pointer flag), the three low bits (alignment padding that some Apple runtimes use for inline refcounts), and the PAC signature bits in the upper byte.

**Step 2: Check the cache.** Apple's production runtime maintains a hash table cache on each class — a power-of-two array of `(selector, IMP)` pairs. The cache check is the hot path: hash the selector, index into the cache, compare. If it hits, dispatch immediately. The cache is the reason `objc_msgSend` is fast enough for production use — after the first call to a method, it goes straight to the IMP without touching the method list.

In our implementation, we skip the cache for simplicity. Our method lookup uses the `method_index` HashMap — which gives us O(1) amortized lookup without implementing a custom hash table in assembly:

```rust
pub fn find_method(&self, selector: &str) -> Option<&MethodEntry> {
    self.method_index
        .get(selector)
        .and_then(|&idx| self.methods.get(idx))
}
```

This is slightly slower than Apple's cache (HashMap lookup instead of open-addressed probing), but correct. For a lab-grade runtime, correctness matters more than nanoseconds.

**Step 3: Walk the superclass chain.** If the method isn't found on the direct class, walk up the superclass chain:

```rust
pub fn lookup_method_entry(&self, isa: u64, selector: &str) -> Option<&MethodEntry> {
    let is_meta = self.metaclass_addrs.contains(&isa);
    let mut current_addr = Some(isa);
    let mut depth = 0;
    while let Some(addr) = current_addr {
        if depth > 64 { break; }
        if let Some(class) = self.get_class_by_addr(addr) {
            let entry = if is_meta {
                class.find_class_method(selector)
            } else {
                class.find_method(selector)
            };
            if let Some(entry) = entry {
                return Some(entry);
            }
            current_addr = class.superclass_addr;
        } else {
            break;
        }
        depth += 1;
    }
    None
}
```

Start at the ISA class. Check its method index. If not found, follow `superclass_addr` to the parent class. Check again. Keep going until you find the method or reach the root class (`NSObject`, where `superclass_addr` is `None`).

The `is_meta` check is important. When the receiver is a class object (not an instance), ObjC dispatch searches *class methods*, not instance methods. This is how `[NSString alloc]` works — `NSString` is itself an object whose ISA points to `NSString`'s metaclass, and `alloc` is a class method stored in the metaclass's method list. Our runtime tracks metaclass addresses in a `HashSet` so it knows which ISA values require the class-method path.

The depth limit of 64 is a safety valve. A well-formed class hierarchy is typically 5-10 levels deep. If we've walked 64 classes without finding the method, something is wrong — likely a circular superclass chain from corrupted metadata. Better to fail cleanly than loop forever.

**Step 4: Invoke the IMP.** Once the method entry is found, extract the IMP (a `u64` that's actually a function pointer) and branch to it. The IMP receives the original arguments — the receiver in `x0`, the selector in `x1`, and any additional arguments in `x2`-`x7`. The IMP doesn't know or care that it was reached through message dispatch. It's just a C function.

This dispatch algorithm — ISA → method list → superclass chain → IMP — is what makes ObjC dynamic. Every step is a runtime decision. Nothing is baked in at compile time. And that dynamism enables everything that makes Cocoa *Cocoa*: categories that extend classes across framework boundaries, KVO that synthesizes subclasses at runtime, `NSProxy` objects that forward every message to a remote process.

Compare the cost. C++ vtable dispatch is three instructions — load, load, branch. ObjC message dispatch is a hash table lookup, possibly walking a chain, with pointer dereferencing at each step. C++ wins on raw speed. ObjC wins on flexibility.

Apple's compromise is the method cache. After the first dispatch of a given selector on a given class, the resolved IMP goes into the cache. Subsequent calls hit the cache — which *is* close to three instructions (hash, compare, branch). The first call pays the cost. Every subsequent call runs at near-vtable speed. It's not as fast as C++ static dispatch, but it's close enough that nobody notices, and the flexibility it buys is the entire Foundation framework.

Apple's cache implementation is worth a brief look even though we don't replicate it. Each class has a `cache_t` struct containing a power-of-two array of `bucket_t` entries. Each bucket holds a `(SEL, IMP)` pair. The lookup hashes the selector pointer (just the pointer value — a few shifts and XORs), indexes into the array, and does an open-addressed linear probe until it finds a matching selector or an empty slot. The hash function is fast, the table is small (starts at 4 entries, grows as needed), and the common case — the first probe hits — is just three loads and a compare.

Apple inlines this cache check directly into `objc_msgSend`'s assembly. The slow path — walking the method list and superclass chain — is a separate function (`lookUpImpOrForward`) called only on cache miss. In practice, cache hit rates exceed 95% for most applications, meaning `objc_msgSend` almost never touches the method list. The cache transforms a fundamentally dynamic operation into something that performs like a static one.

The algorithm is more elegant than you'd expect from a 1980s language runtime. It's not a hack. It's a carefully designed mechanism that balances performance against dynamism, with a cache layer that amortizes the cost. When you understand *why* ObjC dispatch works this way — not just *how* — you start to see why Apple hasn't replaced it with something "simpler." Nothing simpler gives you categories, KVO, and message forwarding all at once.

There's a deeper point here about the relationship between ObjC dispatch and the platform it enables. Every Cocoa design pattern relies on late binding. The responder chain sends messages up a chain of objects and depends on each object *not knowing* in advance whether it handles the message. Delegation sends messages to a delegate object whose class is unknown at compile time. Target-action sends a selector (a string!) chosen at design time in Interface Builder to a target determined at runtime. Notifications post named events to any object that registered as an observer. All of this works because `objc_msgSend` separates the act of sending a message from the act of determining which code runs.

ObjC's dispatch mechanism isn't just an implementation detail. It's the foundation of the platform's architecture. And now we have our own version of it.

---

## Scene 4.4: Building objc_msgSend

Understanding the dispatch algorithm is one thing. Implementing it is another. Because `objc_msgSend` has a requirement that no ordinary function can satisfy: it must be *invisible* to the callee.

When the compiler emits `objc_msgSend(receiver, @selector(init), arg1, arg2)`, the registers are set up for a four-argument function call:

```
x0 = receiver
x1 = selector
x2 = arg1
x3 = arg2
```

After dispatch, the IMP must see those *exact same registers*. The IMP for `-[NSObject init]` is a C function that expects `self` in `x0` and `_cmd` in `x1`. If `objc_msgSend` clobbers any register — even a floating-point register used for a `double` argument — the IMP gets wrong values and the program crashes or produces silent corruption.

This is why `objc_msgSend` must be written in assembly. A Rust `extern "C"` function is allowed to clobber `x8`-`x15`, `d8`-`d15`, and various other registers according to the calling convention. The compiler uses them for temporaries, spill slots, stack frame management. You can't write a correct `objc_msgSend` in C or Rust because the *act of looking up the method* would destroy the argument registers that the method needs to receive.

Apple's `objc_msgSend` is famously hand-tuned assembly — different versions for ARM64, x86_64, and the now-retired i386 and ARM32. It's one of the most called functions on any Apple platform. It has to be perfect.

Ours doesn't need to be Apple-fast. It needs to be correct. Here's the trampoline, in `crates/objc-runtime/src/trampoline.s`:

```arm64
// Flow:
//   1. Save x0-x7, x8, d0-d7, lr to stack (176 bytes)
//   2. Call objc_msg_lookup_imp(x0=self, x1=sel) → returns IMP in x0
//   3. Move IMP to x16 (scratch register per ARM64 ABI)
//   4. Restore x0-x7, x8, d0-d7, lr
//   5. Branch to x16 (tail call — IMP sees original args)

.global shim_objc_msg_send_asm
.p2align 4

shim_objc_msg_send_asm:
    // ── nil check ──────────────────────────────────
    cbz     x0, .Lnil_return

    // ── save state ─────────────────────────────────
    sub     sp, sp, #176

    stp     x0, x1, [sp, #0]       // integer args
    stp     x2, x3, [sp, #16]
    stp     x4, x5, [sp, #32]
    stp     x6, x7, [sp, #48]

    stp     d0, d1, [sp, #64]      // floating-point args
    stp     d2, d3, [sp, #80]
    stp     d4, d5, [sp, #96]
    stp     d6, d7, [sp, #112]

    stp     x30, x8, [sp, #128]    // link register + struct return ptr

    // ── lookup ─────────────────────────────────────
    bl      objc_msg_lookup_imp    // Rust: (self, sel) → IMP

    mov     x16, x0                // IMP → scratch register

    // ── restore state ──────────────────────────────
    ldp     x30, x8, [sp, #128]
    ldp     d0, d1, [sp, #64]
    ldp     d2, d3, [sp, #80]
    ldp     d4, d5, [sp, #96]
    ldp     d6, d7, [sp, #112]
    ldp     x0, x1, [sp, #0]
    ldp     x2, x3, [sp, #16]
    ldp     x4, x5, [sp, #32]
    ldp     x6, x7, [sp, #48]

    add     sp, sp, #176

    // ── tail call ──────────────────────────────────
    br      x16

.Lnil_return:
    mov     x0, #0
    fmov    d0, xzr
    ret
```

Let's walk through this instruction by instruction, because every line matters and a single mistake means a SIGSEGV or — worse — silent data corruption.

**`cbz x0, .Lnil_return`** — Check if the receiver is nil. ObjC has defined nil-messaging semantics: sending any message to `nil` returns zero. Not a crash. Not an exception. Zero. This behavior is load-bearing — Cocoa code relies on it extensively. `[nil count]` returns 0. `[nil isEqual:something]` returns NO (0). `[nil frame]` returns `CGRectZero`. The nil check must be the very first instruction, before we touch the stack.

**`sub sp, sp, #176`** — Reserve 176 bytes on the stack. Why 176? We need to save:
- `x0`-`x7`: 8 registers × 8 bytes = 64 bytes (integer arguments)
- `d0`-`d7`: 8 registers × 8 bytes = 64 bytes (floating-point arguments)
- `x30` (link register) + `x8` (struct return): 2 × 8 = 16 bytes
- Padding: 32 bytes for 16-byte stack alignment

Total: 176 bytes, 16-byte aligned as required by the ARM64 ABI.

**`stp x0, x1, [sp, #0]`** — Store pair. ARM64's `stp` instruction writes two 64-bit registers to memory in a single operation. We use four `stp` instructions to save all eight integer argument registers, four more for the floating-point registers, and one for `x30` (the return address) and `x8`.

Why save `x8`? On ARM64, `x8` is the "indirect result location register." When a function returns a large struct (more than 16 bytes — like `CGRect`, which is 32 bytes), the caller passes the address of the return buffer in `x8`. If we don't save and restore `x8`, methods returning large structs will write their return value to wherever `x8` happened to point after the lookup — which is certainly not the caller's buffer. This is the kind of bug that produces intermittent memory corruption: most methods return small values and don't use `x8`, so the bug only manifests for methods returning `CGRect` or `CGSize` or `NSRange`.

Apple handles this differently: on ARM64, `objc_msgSend_stret` is an alias for `objc_msgSend`, not a separate function. Because `x8` is always preserved, there's no need for a separate entry point. On x86, `objc_msgSend_stret` *is* different because the struct return pointer is passed on the stack. Our trampoline takes the ARM64 approach — one entry point handles everything, with `objc_msgSend_stret` and `objc_msgSend_fp2ret` as aliases:

```arm64
.global shim_objc_msg_send_stret_asm
.global shim_objc_msg_send_fp2ret_asm

shim_objc_msg_send_asm:
shim_objc_msg_send_stret_asm:
shim_objc_msg_send_fp2ret_asm:
    cbz     x0, .Lnil_return
    ...
```

Three global symbols, one function body. Labels are free.

**`bl objc_msg_lookup_imp`** — Branch with link to our Rust lookup function. This is where assembly meets Rust. The `bl` instruction saves the return address in `x30` (which we already saved on the stack) and branches to `objc_msg_lookup_imp`. At this point, `x0` still holds the original receiver and `x1` still holds the selector — we saved them but haven't clobbered them. The Rust function receives `(self: u64, sel: u64)` and returns the IMP in `x0`.

The Rust side of this bridge is the `objc_msg_lookup_imp` function we saw in Scene 4.3:

```rust
#[no_mangle]
pub extern "C" fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64 {
    // ... read ISA, check tagged pointers, walk class chain ...
}
```

The `#[no_mangle]` attribute ensures the Rust compiler doesn't modify the function name. The `extern "C"` ensures C calling convention. The linker resolves the `bl objc_msg_lookup_imp` in the assembly to this Rust function — assembly and Rust, linked together.

**`mov x16, x0`** — Move the IMP from `x0` (where `objc_msg_lookup_imp` returned it) to `x16`. Why `x16`? In the ARM64 procedure call standard, `x16` and `x17` are "intra-procedure-call scratch registers" — they're specifically designated for use by trampolines and veneers. No caller expects their values to be preserved. It's the safe place to stash a function pointer while we restore the argument registers.

**Restore block** — The `ldp` instructions mirror the `stp` saves in reverse. After this block, every register has exactly the value it had when `objc_msgSend` was entered. The IMP in `x16` is the only new value.

**`br x16`** — Branch to register. Not `blr` (branch with link) — `br`. This is a tail call. We don't create a new stack frame. The IMP returns directly to `objc_msgSend`'s caller, as if `objc_msgSend` never existed. This is essential for correctness: if we used `blr`, the IMP's return would come back to our trampoline, and we'd need to forward the return value — including possible floating-point returns in `d0`, struct returns via `x8`, and multi-register returns in `x0`/`x1`. The tail call avoids all of this.

**`.Lnil_return`** — The nil path. Zero `x0` (for integer return values), zero `d0` (for floating-point return values), and `ret`. This handles the contract that `[nil anyMethod]` returns 0/0.0/nil/NO.

The cross-platform concerns add a layer of complexity. On macOS, C symbol names have a leading underscore — `_objc_msg_lookup_imp`. On Linux (ELF), they don't — `objc_msg_lookup_imp`. The trampoline uses preprocessor macros to handle this:

```arm64
#ifdef __APPLE__
#define CSYM(name) _##name
#else
#define CSYM(name) name
#endif
```

Similarly, external symbol references use different address modes. macOS uses `@PAGE`/`@PAGEOFF` relocations. Linux ELF uses `:got:`/`:got_lo12:` for GOT-relative addressing in position-independent code. The `LOAD_GLOBAL` macro abstracts this.

The trampoline is 25 instructions in the normal path (nil check, save, call, move, restore, branch). That's the entire cost of ObjC message dispatch in our runtime: 25 instructions to save state, call Rust for the lookup, restore state, and tail-call the IMP. Apple's production trampoline is more optimized — it inlines the cache check and avoids the function call for cache hits — but our 25-instruction version is correct, portable, and comprehensible.

This is the single most critical piece of assembly in the project. Every ObjC method call flows through it. Every `[NSObject alloc]`, every `[NSString stringWithFormat:]`, every `[NSArray count]`. If one register is saved wrong, if the stack alignment is off by one, if the tail call uses `blr` instead of `br` — the system fails. Not with a clean error. With corruption. With methods receiving wrong arguments and writing to wrong addresses and returning wrong values, producing symptoms that look like bugs in completely unrelated code.

We got it right on the third attempt. The first version didn't save `x8`. The second version used `blr` instead of `br`. The third version works.

### objc_msgSendSuper

There's a companion trampoline: `objc_msgSendSuper`. When ObjC code calls `[super init]`, the compiler generates `objc_msgSendSuper(&(struct objc_super){ self, [SuperClass class] }, @selector(init))`. The first argument isn't the receiver — it's a pointer to a two-field struct containing the receiver and the superclass to start searching from.

The super trampoline reads the struct, starts the method search at the superclass instead of the receiver's ISA class, and then patches `x0` back to the receiver before tail-calling the IMP. The IMP must receive `self` — not the `objc_super` struct pointer — so the trampoline swaps the argument:

```arm64
shim_objc_msg_send_super_asm:
    ldr     x9,  [x0, #0]      // receiver = objc_super->receiver
    ldr     x10, [x0, #8]      // super_class = objc_super->super_class

    sub     sp, sp, #192
    stp     x0, x1, [sp, #0]
    // ... save all registers including x9, x10 ...

    mov     x0, x9              // arg0 = receiver
    mov     x1, x10             // arg1 = super_class
    mov     x2, x1_saved        // arg2 = selector
    bl      objc_msg_super_lookup_imp

    mov     x16, x0             // IMP → scratch
    // ... restore registers, replace x0 with receiver ...
    br      x16
```

The Apple runtime also has `objc_msgSendSuper2`, which takes the *current* class instead of the superclass in the struct, and internally reads the superclass pointer from the class's `class_t`. This variant exists for compiler optimization — the compiler doesn't need to emit code to resolve the superclass, the trampoline does it. We implement both variants.

---

## Scene 4.5: Class Registration

The dispatch mechanism is built. The trampoline bridges assembly to Rust and back. Selectors are interned. Now we need something to dispatch *to* — a class table populated with every class from every loaded image.

The starting point is the `__objc_classlist` section. Every Mach-O image that contains ObjC classes has this section. It's an array of pointers, each pointing to a `class_t` struct in `__DATA`:

```
__DATA,__objc_classlist:
  +0x00: 0x0000000100008100   → class_t for "AppDelegate"
  +0x08: 0x0000000100008140   → class_t for "ViewController"
  +0x10: 0x0000000100008180   → class_t for "CustomView"
```

The `macho-objc` crate's `parse_objc_metadata` function reads this section and follows each pointer to reconstruct the full class descriptor — name, methods, ivars, properties, protocols, superclass address, metaclass address, instance size. The result is an `ObjcMetadata` struct containing a vector of `ObjcClassRef` entries.

Registration takes these parsed entries and builds the runtime's class table:

```rust
pub fn register_image(&mut self, metadata: &ObjcMetadata, slide: i64) {
    for class_ref in &metadata.class_refs {
        let Some(name) = &class_ref.name else { continue };
        let mapped_addr = (class_ref.class_data_addr as i64 + slide) as u64;

        let is_root = (class_ref.ro_flags & 0x02) != 0;
        let is_meta = (class_ref.ro_flags & 0x01) != 0;

        let methods: Vec<MethodEntry> = class_ref.methods.iter()
            .map(|m| MethodEntry {
                selector: m.selector.clone().into(),
                imp: (m.imp_vmaddr as i64 + slide) as u64,
                type_encoding: m.type_encoding.clone().into(),
            })
            .collect();

        // ... build ClassInfo, insert into tables ...

        let name_arc: Arc<str> = Arc::from(name.as_str());
        self.classes_by_addr.insert(mapped_addr, Arc::clone(&name_arc));
        self.classes_by_name.insert(Arc::clone(&name_arc), Arc::new(info));
    }
}
```

The ASLR slide adjustment is critical. The `class_data_addr` and `imp_vmaddr` values parsed from the binary are *pre-slide* virtual addresses — where the binary expected to be loaded. The actual runtime address is `vmaddr + slide`. Every address the runtime stores must be post-slide, or the ISA pointer in a live object won't match any entry in `classes_by_addr`.

The runtime maintains two primary indexes:

- **`classes_by_name`**: `HashMap<Arc<str>, Arc<ClassInfo>>` — look up a class by its string name. Used by `objc_getClass("NSObject")` and similar introspection functions.
- **`classes_by_addr`**: `HashMap<u64, Arc<str>>` — look up a class name by its mapped address. This is the hot path: when `objc_msgSend` reads an ISA pointer from an object, it needs to find the corresponding ClassInfo. The ISA → name → ClassInfo two-step lookup is how dispatch works.

Metaclasses require special handling. For every class `Foo`, the binary contains two class_t structures: one for `Foo` (instance methods) and one for `Foo`'s metaclass (class methods). They share the same name — both are "Foo" — but they have different addresses and different method lists. The metaclass's address goes into `metaclass_addrs`:

```rust
if class_ref.metaclass_addr != 0 {
    let meta_mapped = (class_ref.metaclass_addr as i64 + slide) as u64;
    self.classes_by_addr.insert(meta_mapped, Arc::clone(&name_arc));
    self.metaclass_addrs.insert(meta_mapped);
}
```

The `+load` method adds a timing constraint. ObjC classes can define a `+load` class method that runs during image loading — before `main()`, before `__attribute__((constructor))` functions, before almost anything. The `+load` ordering is strict: superclass `+load` runs before subclass `+load`, and class `+load` runs before category `+load`. Our runtime collects `+load` IMPs during registration and topologically sorts them:

```rust
pub fn collect_load_methods(&self) -> Vec<(String, u64)> {
    // 1. Collect class +load methods (excluding category-contributed ones)
    let mut class_loads: Vec<(String, u64)> = Vec::new();
    for (name, info) in &self.classes_by_name {
        for m in &info.class_methods {
            if m.selector == "load" && !cat_imp_set.contains(&m.imp) {
                class_loads.push((name.to_string(), m.imp));
            }
        }
    }

    // 2. Topological sort: superclasses before subclasses
    // ... DFS walking superclass chain ...

    // 3. Append category +loads in registration order
    sorted.extend(self.category_load_imps.iter().cloned());
    sorted
}
```

This ordering is Apple-compatible and load-bearing. Foundation's `+load` methods initialize global state that other classes' `+load` methods depend on. Getting the order wrong causes subtle failures — not crashes, but uninitialized state that manifests as `nil` values in unexpected places.

Both the class address and the metaclass address map to the same name in `classes_by_addr`. The `metaclass_addrs` set distinguishes them — when dispatch encounters a metaclass ISA, it searches `class_methods` instead of `methods`. This is how `[NSString alloc]` works: `NSString` is itself an object whose ISA points to the `NSString` metaclass, and `alloc` lives in the metaclass's method list.

The method index deserves attention. Each `ClassInfo` maintains two `HashMap<Cow<'static, str>, usize>` — one for instance methods, one for class methods. The map keys are selector strings; the values are indices into the corresponding `Vec<MethodEntry>`. This gives O(1) method lookup while keeping the method list ordered (category merging appends to the `Vec`, and the index is rebuilt):

```rust
pub fn rebuild_indices(&mut self) {
    self.method_index = HashMap::with_capacity(self.methods.len());
    for (i, m) in self.methods.iter().enumerate() {
        self.method_index.insert(m.selector.clone(), i);
    }
    self.class_method_index = HashMap::with_capacity(self.class_methods.len());
    for (i, m) in self.class_methods.iter().enumerate() {
        self.class_method_index.insert(m.selector.clone(), i);
    }
}
```

Multi-image registration is sequential. When we load a binary and its dependencies, each image's ObjC metadata gets registered in dependency order — foundation frameworks first, then higher-level frameworks, then the main executable. This order matters because later registrations can override earlier ones (via categories), and superclass addresses must resolve to already-registered classes.

After registration completes for all images, the runtime state looks like this:

```
ObjcRuntime:
  classes_by_name:
    "NSObject"         → ClassInfo { addr: 0x1a4230100, methods: 12, ... }
    "NSString"         → ClassInfo { addr: 0x1a4230200, methods: 47, ... }
    "NSMutableString"  → ClassInfo { addr: 0x1a4230300, methods: 23, ... }
    "NSArray"          → ClassInfo { addr: 0x1a4230400, methods: 31, ... }
    ...

  classes_by_addr:
    0x1a4230100 → "NSObject"
    0x1a4230108 → "NSObject"     (metaclass)
    0x1a4230200 → "NSString"
    0x1a4230208 → "NSString"     (metaclass)
    ...

  selectors: { "init" → 0x60001000, "alloc" → 0x60001008, ... }
```

Every class from every loaded image, indexed by name and by address. Every selector interned and uniqued. The dispatch trampoline can resolve any message send in the system. The runtime is populated.

---

## Scene 4.6: Protocols and Categories

Classes and methods give us the foundation. Protocols and categories give us the dynamism that makes ObjC... ObjC.

### Protocols

The `__objc_protolist` section contains pointers to protocol descriptors. A protocol in ObjC is a contract — a set of method signatures that a class promises to implement. At the binary level, a protocol descriptor looks a lot like a class descriptor, minus the ISA chain:

```rust
pub struct ObjcProtocolRef {
    pub name: String,
    pub instance_methods: Vec<ObjcMethodInfo>,
    pub class_methods: Vec<ObjcMethodInfo>,
    pub optional_instance_methods: Vec<ObjcMethodInfo>,
    pub optional_class_methods: Vec<ObjcMethodInfo>,
    pub properties: Vec<ObjcPropertyInfo>,
    pub adopted_protocols: Vec<String>,
}
```

Notice the four method lists — required instance, required class, optional instance, optional class. ObjC protocols can declare optional methods (using `@optional`), which means a conforming class doesn't have to implement them. This is a departure from protocols/interfaces in most languages. Java interfaces require all methods. Rust traits require all non-default methods. ObjC protocols let you say "implement these if you want."

The `adopted_protocols` field lists protocols that this protocol itself conforms to. Protocol can inherit from other protocols: `NSCoding` adopts no protocols, but `NSSecureCoding` adopts `NSCoding`. Conformance checking must be recursive — a class that conforms to `NSSecureCoding` also conforms to `NSCoding`:

```rust
pub fn class_conforms_to_protocol(&self, class_name: &str, protocol_name: &str) -> bool {
    let mut current: Option<Arc<str>> = Some(Arc::from(class_name));
    let mut depth = 0;
    while let Some(name) = current {
        if depth > 64 { break; }
        if let Some(class) = self.classes_by_name.get(&*name) {
            if class.protocol_names.iter().any(|p| p == protocol_name) {
                return true;
            }
            current = class.superclass_addr
                .and_then(|addr| self.classes_by_addr.get(&addr))
                .cloned();
        } else { break; }
        depth += 1;
    }
    false
}
```

The conformance check walks the superclass chain — if `NSMutableArray` conforms to `NSCopying`, and `NSArray` is its superclass, then asking "does `NSArray` conform to `NSCopying`?" also checks `NSArray` directly. This mirrors Apple's runtime behavior where protocol conformance is inherited.

Protocol registration is straightforward — parse the protocol descriptors from `__objc_protolist`, build `ProtocolInfo` structs, insert into `protocols_by_name`. Protocols don't participate in message dispatch directly — they're metadata for runtime introspection, for `conformsToProtocol:` checks, and for the compiler's type system.

### Categories

Categories are where it gets interesting. A category adds methods to an existing class — without subclassing, without access to the original source code. The `__objc_catlist` section contains category descriptors:

```rust
pub struct ObjcCategoryRef {
    pub name: Option<String>,
    pub class_vmaddr: u64,
    pub cat_struct_vmaddr: u64,
    pub instance_methods: Vec<ObjcMethodInfo>,
    pub class_methods: Vec<ObjcMethodInfo>,
}
```

The `class_vmaddr` field identifies the target class — the class these methods are being added to. A category named `"NSString(URLEncoding)"` might add methods like `stringByAddingPercentEncoding:` to `NSString`.

During registration, category methods get merged into the target class's method list:

```rust
// Category method injection
for cat in &metadata.categories {
    let cat_class_addr = (cat.class_vmaddr as i64 + slide) as u64;
    let target_name = self.classes_by_addr.get(&cat_class_addr);

    if let Some(class_arc) = self.classes_by_name.get_mut(&*target_name) {
        let class_info = Arc::make_mut(class_arc);
        for m in &cat.instance_methods {
            if !class_info.method_index.contains_key(&*m.selector) {
                class_info.methods.push(MethodEntry {
                    selector: m.selector.clone().into(),
                    imp: (m.imp_vmaddr as i64 + slide) as u64,
                    type_encoding: m.type_encoding.clone().into(),
                });
            }
        }
        class_info.rebuild_indices();
    }
}
```

The duplicate check (`contains_key`) prevents a category from replacing a method that already exists on the class — either from the class's original definition or from a previously loaded category. In Apple's runtime, categories *do* replace existing methods (last category loaded wins), but we take the conservative approach for our shim-heavy environment: if a shim implementation already exists for a selector, we keep it. The shim is known to work; the guest category implementation might depend on Foundation internals we haven't built yet.

Cross-image categories add a complication. When a category in Image B targets a class defined in Image A, the `class_vmaddr` field is initially zero in Image B's binary — it's a bind slot that gets filled during fixup resolution. We handle this with a second pass, `merge_categories_post_fixup`, that reads the resolved class pointer from mapped memory after fixups have run:

```rust
pub fn merge_categories_post_fixup(&mut self, metadata: &ObjcMetadata, slide: i64) {
    for cat in &metadata.categories {
        let cat_mapped_addr = (cat.cat_struct_vmaddr as i64 + slide) as u64;
        let cls_ptr_addr = cat_mapped_addr + 8;  // offset of cls field
        let resolved_cls = unsafe { *(cls_ptr_addr as *const u64) };
        // Look up target class by resolved address, merge methods...
    }
}
```

This two-pass approach — merge same-image categories during registration, merge cross-image categories after fixups — handles the dependency ordering problem. Categories within the same framework can be merged immediately because the target class address is already known. Categories that cross framework boundaries must wait until the linker has resolved the bind slot.

Categories also participate in `+load` ordering. If a category defines `+load`, it runs after the target class's `+load` but in the order the categories were loaded. The runtime records category `+load` IMPs during merging:

```rust
for m in &cat.class_methods {
    if m.selector == "load" {
        let imp = (m.imp_vmaddr as i64 + slide) as u64;
        self.category_load_imps.push((target_name.to_string(), imp));
    }
}
```

These get appended to the `+load` execution list after the topologically sorted class `+load` methods. The ordering — superclasses before subclasses, classes before categories, categories in load order — matches Apple's documented behavior and is load-bearing for real frameworks.

The picture is completing. Classes provide the structure. Protocols provide the contracts. Categories provide the dynamism. Together, they give us a runtime that can represent the full ObjC type system — not just what the binary declared, but what other binaries are adding to it at load time.

---

## Scene 4.7: Dynamic Class Creation

Everything so far has been static. We parse metadata from binaries, build tables, and dispatch. But ObjC's runtime isn't static. It modifies itself.

The runtime API includes functions that create new classes from nothing:

- `objc_allocateClassPair(superclass, "MyNewClass", 0)` — allocate a new class
- `class_addMethod(cls, @selector(doThing:), imp, "v@:@")` — add a method
- `objc_registerClassPair(cls)` — finalize and register

These aren't obscure functions used by runtime hackers. They're used by mainstream frameworks every day. CoreData generates `NSManagedObject` subclasses at runtime. KVO (Key-Value Observing) creates invisible subclasses to intercept property setters. XCTest dynamically creates test case classes from test methods discovered via reflection. Swift's own `Mirror` type uses runtime introspection that depends on dynamic class metadata. If your loader can't create classes at runtime, entire frameworks stop working.

The lifecycle of a dynamically created class has three phases. First, `objc_allocateClassPair` creates a new `class_t` and its companion metaclass, both unregistered. The class inherits its superclass's instance size and method lists but starts with an empty method list of its own. Second, the caller adds methods, ivars, and protocol conformances using `class_addMethod`, `class_addIvar`, and `class_addProtocol`. These mutations are only safe during this interim phase — once the class is registered, its ivar layout is frozen (methods can still be added, but ivars cannot). Third, `objc_registerClassPair` makes the class visible in the global class table. After registration, `objc_getClass("MyNewClass")` returns it, and objects can be allocated with `[MyNewClass alloc]`.

Our implementation allocates `ClassInfo` structs in Rust and registers them in the normal class table:

```rust
pub fn add_method_to_class(&mut self, class_addr: u64, entry: MethodEntry) -> bool {
    self.intern_selector(&entry.selector);
    let is_metaclass = self.is_metaclass(class_addr);
    let Some(class) = self.get_class_by_addr_mut(class_addr) else {
        return false;
    };
    let methods = if is_metaclass {
        &mut class.class_methods
    } else {
        &mut class.methods
    };
    if methods.iter().any(|m| m.selector == entry.selector) {
        return false;  // method already exists
    }
    let idx = methods.len();
    let sel = entry.selector.clone();
    methods.push(entry);
    let method_index = if is_metaclass {
        &mut class.class_method_index
    } else {
        &mut class.method_index
    };
    method_index.insert(sel, idx);
    true
}
```

The function returns `false` if the method already exists — matching Apple's behavior where `class_addMethod` is a no-op for existing selectors (you need `class_replaceMethod` to override). The method index is updated inline to maintain O(1) lookup.

Dynamic class creation introduces an addressing challenge. Classes from binaries have natural addresses — their `class_t` struct lives at a specific virtual address in the mapped image. Dynamically created classes don't live in any mapped image. They need synthetic addresses that don't collide with any real address.

We allocate from a region above the normal virtual address space. The runtime maintains a synthetic address counter starting at a high offset (e.g., `0xFFFF_0000_0000_0000`). Each dynamically created class gets a unique address from this range. These addresses will never collide with real mapped memory on any current OS, and the ISA → class lookup works the same way as for static classes.

### KVO ISA Swizzling

The most fascinating use of dynamic class creation is KVO — Key-Value Observing. When you call `[object addObserver:self forKeyPath:@"name"]`, the runtime doesn't install a callback. It does something far more audacious: it creates a new class.

Here's what happens. Suppose `object` is an instance of `Person`. Before `addObserver:`, the object's ISA points to `Person`. After `addObserver:`, the ISA points to a secretly created subclass called `NSKVONotifying_Person`. This synthetic subclass overrides the observed property's setter (`setName:`) with a version that calls `willChangeValueForKey:`, invokes the original setter, then calls `didChangeValueForKey:`. The observer notification happens inside those will/did methods.

The object's class has been changed *without its knowledge*. If you call `[object class]`, KVO overrides that too — it returns `Person`, not `NSKVONotifying_Person`. The swizzle is invisible to the observed object. Only the runtime knows.

This is beautiful and dangerous in equal measure.

Beautiful because it's transparent. You don't change your class design to support observation. You don't adopt a protocol. You don't register callbacks. You just say "watch this property" and the runtime handles everything by rewriting the object's type at the binary level.

Dangerous because it means you can never trust an ISA pointer at face value. The class an object appears to be (`[obj class]`) may not be the class the object *actually is* (the ISA pointer's target). KVO breaks the assumption that `object_getClass(obj) == [obj class]`. Debuggers, serializers, and introspection tools that read the ISA directly will see `NSKVONotifying_Person`. Code that calls `[obj class]` will see `Person`. Two different answers to "what type is this object?"

For our runtime, the implication is clear: dynamic class creation must work correctly, because framework code depends on it. We don't implement KVO's full property observation machinery — that requires Foundation's `NSKeyValueObserving` protocol and its setter-generation logic. But we implement the primitives: `objc_allocateClassPair` creates a new class with a given superclass, `class_addMethod` adds methods to it, and `objc_registerClassPair` makes it visible to the global class table.

With these primitives, Foundation can perform ISA swizzling when it needs to. The runtime supports the operation. The policy is Foundation's business.

There's a philosophical point here. We keep encountering this pattern: the runtime provides mechanisms, not policies. `objc_msgSend` dispatches messages without knowing what messages mean. `intern_selector` uniquifies strings without knowing what they name. `objc_allocateClassPair` creates classes without knowing why. The runtime is infrastructure. It doesn't have opinions about what you build on it.

This separation of mechanism from policy is what makes the ObjC runtime so durable. It's been supporting frameworks it was never designed for — CoreData, SwiftUI's ObjC compatibility layer, Combine's KVO bridge — for decades. The mechanisms are general enough that new policies can be layered on top without changing the runtime itself.

Our runtime inherits this property. We build the mechanisms. Foundation (Chapter 5) will build the policies.

---

## Scene 4.8: Tagged Pointers

We have one more trick to handle before the runtime is complete. And it's the cleverest one.

Look at this value in the debugger:

```
x0 = 0xb000000000000032
```

That's supposed to be an ObjC object pointer — a receiver passed to `objc_msgSend`. But it can't be a real pointer. Bit 63 is set (`0xb...`), which means this address is in the kernel's virtual address space on ARM64. User-space code can't have pointers above `0x7FFF_FFFF_FFFF`. And even if it could, the value is suspiciously small and round.

This isn't a pointer at all. It's a *tagged pointer* — a value that encodes both the class and the payload directly in the 64-bit word, without any heap allocation.

Let's decode it:

```
0xb000000000000032

Bit 63:    1       → "I am a tagged pointer"
Bits 60-62: 0b101  → tag class index = 5
Bits 0-59:  0x32   → payload = 50 (decimal)
```

Tag class 5 is `NSNumber`. This tagged pointer represents `@(50)` — the NSNumber wrapping the integer 50. The number isn't stored on the heap. There's no 16-byte object with an ISA pointer and an ivar. The number *is* the pointer.

Apple introduced tagged pointers in OS X 10.10 to avoid heap-allocating tiny, immutable objects. Consider how much overhead a heap-allocated `@(50)` has:

- 16 bytes for the `malloc` header
- 8 bytes for the ISA pointer
- 8 bytes for the integer value
- Total: 32 bytes, plus `malloc` bookkeeping

A tagged pointer uses zero bytes of heap memory. The value fits in a register. No allocation, no deallocation, no reference counting, no cache misses from dereferencing a heap pointer. For applications that create millions of small NSNumber instances (every collection index, every measurement, every coordinate), the savings are enormous.

The tag class assignments on ARM64 are:

| Tag (bits 60-62) | Class |
|---|---|
| 0 | Reserved |
| 1 | NSAtom (unused) |
| 2 | NSIndexPath |
| 3 | NSManagedObjectID |
| 4 | NSDate |
| 5 | NSNumber |
| 6 | NSString (short) |
| 7 | Extended tag (bits 52-59 select from 256 additional classes) |

Let's decode a tagged `NSString` to see just how tightly Apple packs data. The string `"Hi"` can be represented as:

```
0xe200000000004948

Bit 63:    1       → tagged pointer
Bits 60-62: 0b110  → tag class index = 6 (NSString)
Bits 56-59: 0x2    → string length = 2
Bits 0-55:  0x4948 → 'H' (0x48), 'i' (0x49) — ASCII, little-endian
```

Two bytes of ASCII packed into the low bits of a pointer. The string `"Hi"` requires zero heap memory, zero reference counting, and fits entirely in a register. Compare that to a heap-allocated `NSString`:

- 16 bytes malloc header
- 8 bytes ISA pointer
- 8 bytes length field
- 8 bytes pointer to character buffer
- 3 bytes for the characters ('H', 'i', '\0') rounded up to 16 with alignment
- Total: 56+ bytes

From 56 bytes to zero bytes. For short strings that appear everywhere — dictionary keys, notification names, localization table keys — the savings are staggering. Mike Ash, who reverse-engineered much of the tagged pointer system, estimated that tagged strings alone save hundreds of megabytes of heap memory in a typical iOS application.

The reference counting interaction is equally clever. `retain` and `release` on a tagged pointer are no-ops — the runtime detects the tagged bit and returns immediately. There's nothing to retain or release because there's no heap allocation to manage. This means tagged pointers are inherently thread-safe: they can be shared across threads without atomic reference counting overhead.

`NSDate` stores a time interval from the reference date in its payload bits. `NSString` stores up to 7 ASCII characters (or fewer multibyte characters) inline. `NSIndexPath` stores short paths of small indices.

Our runtime needs to handle tagged pointers in `objc_msgSend`. When the receiver has bit 63 set, we can't dereference it to read an ISA — that would be a segfault. Instead, we extract the tag class bits and look up the corresponding class:

```rust
if obj & (1u64 << 63) != 0 {
    // Tagged pointer — look up class from tag table
    let sel_str = unsafe { CStr::from_ptr(sel as *const c_char) }
        .to_str()
        .unwrap_or("");
    if let Some(isa) = rt.tagged_class_for_pointer(obj) {
        if let Some(imp) = rt.msg_lookup(isa, sel_str) {
            return imp;
        }
    }
    return unrecognized_selector_stub as *const () as u64;
}
```

The `tagged_class_for_pointer` method extracts bits 60-62 and indexes into the runtime's `tagged_classes` array — a 7-element array mapping tag indices to class addresses. When the runtime is initialized, we populate this table with the addresses of the classes that Apple assigns to each tag slot.

The dispatch after tag-class lookup follows the same path as regular dispatch — find the class, search the method list, walk the superclass chain. A tagged `NSNumber` responds to `intValue`, `doubleValue`, `compare:`, `isEqual:`, and every other `NSNumber` method. From the caller's perspective, it's indistinguishable from a heap-allocated `NSNumber`. The tag dispatch is invisible.

There's a subtlety in the nil-check interaction. Our trampoline's first instruction is `cbz x0, .Lnil_return` — branch if zero. A tagged pointer is never zero (bit 63 is always set), so tagged pointers pass the nil check correctly. If the nil check tested bit 63 instead of testing for zero, tagged pointers would be misidentified as nil. The design of tagged pointers is deliberately compatible with the existing nil check.

Apple engineers are clever. The tagged pointer scheme:
1. Eliminates heap allocation for small immutable objects
2. Requires no changes to calling code — existing `objc_msgSend` calls work unchanged
3. Is backward-compatible with the nil check
4. Uses a single bit to distinguish the two worlds (pointer vs. inline value)
5. Reserves an extended tag for future class additions without changing the encoding

The extended tag (tag 7) uses bits 52-59 as a secondary class index, allowing up to 256 additional tagged classes. Apple has been slowly adding extended tag classes over the years — `NSIndexSet`, `NSOrderedSet`, and others have gotten tagged pointer support. The encoding is open-ended.

For our runtime, tagged pointer support means that code like this works correctly:

```objc
NSNumber *n = @(42);        // tagged pointer, no heap allocation
NSNumber *m = @(42);        // same tagged pointer value
[n isEqual:m];              // YES — same value
[n intValue];               // 42 — decoded from pointer bits
```

Without tagged pointer handling, `objc_msgSend` would try to dereference `0xb00000000000002a` as a pointer, reading ISA from kernel address space. SIGSEGV. With it, the runtime recognizes the tag, finds `NSNumber` in the class table, dispatches `intValue`, and the shim decodes the payload from the pointer bits.

---

The runtime works.

We started this chapter with a crash — `[NSObject init]` segfaulting because there was no ObjC runtime to dispatch through. Eight scenes later, we have:

- An object model parsed from binary metadata: ISA pointers, `class_t`, `class_ro_t`, method lists
- Selector interning that turns string comparison into pointer equality
- A message dispatch algorithm that walks the ISA → class → superclass chain
- An ARM64 assembly trampoline that preserves every register across the lookup
- A class table populated from every loaded image, with metaclass support
- Protocol conformance checking and category method injection
- Dynamic class creation for runtime-generated types and KVO swizzling
- Tagged pointer dispatch for inline small values

The `ObjcRuntime` struct, the `OBJC_RUNTIME` global, the assembly trampoline, the Rust lookup callback — together they form a complete, functional ObjC runtime. `objc_msgSend` dispatches correctly. `objc_getClass` finds classes by name. `sel_registerName` interns selectors. `class_addMethod` extends classes at runtime. Every piece is in place.

Let's test it:

```
$ macho-inspect --exec /opt/xcode-tools/xcode-tc/xcodebuild -version
[NSString stringWithFormat:] → 0x0000000000000000
```

No crash this time. `objc_msgSend` dispatches. The selector resolves. The method is found. The IMP is called. But the return value is zero — `nil`. `stringWithFormat:` is supposed to return an `NSString`. Instead it returns nothing.

The runtime dispatches correctly, but the method *implementations* are empty stubs. We have the mechanism — message dispatch works. We don't have the policy — no one has told the runtime what `stringWithFormat:` actually *does*. The method list says "this selector exists," but the IMP points to a placeholder that returns zero.

We need Foundation. We need real implementations of `NSString`, `NSArray`, `NSDictionary`, `NSData` — the classes that every macOS binary assumes exist and work. We need toll-free bridging between Core Foundation C types and their ObjC counterparts. We need `NSBundle` and `NSProcessInfo` and `NSFileManager` and the hundreds of other classes that form the bedrock of the platform.

The runtime is the engine. Foundation is the fuel.

That's Chapter 5.
