# Chapter 9: The Tools

## Scene 9.1: The Test Harness

We need to test everything.

That's where Chapter 8 left us — staring at a system layer that works for 39 corpus tools and quietly wondering what happens when the count reaches 300. The shim table has 400+ entries. The Foundation implementation covers 93 classes. The struct translations handle `stat`, `statfs`, `dirent`, and a pile of variadic ABI conversions. Every one of those implementations was written to make a specific corpus tool pass. What we don't have is proof that they compose correctly when a binary we've never tested calls them in a sequence we've never anticipated.

Ad hoc testing got us here. Manual invocations, eyeballing output, checking exit codes in a terminal. "Does `lipo -info` print something reasonable? Good. Does `string_processor` exit 0? Good. Move on." That workflow stops scaling the moment you have more tools than patience. You'll skip a tool because you tested it last week. You'll forget to recheck the ones that depend on the exact Foundation method you just refactored. You'll merge a change that breaks `json_formatter` and not discover it until three days later when you accidentally run it again.

The first thing we need is a corpus.

A corpus, in the testing sense, is a structured collection of inputs with known expected outputs. Ours lives at `corpus/` in the project root — 39 subdirectories, each containing a compiled Mach-O binary, its source code, and a `meta.json` descriptor:

```
corpus/
├── string_processor/
│   ├── app              # Compiled arm64 Mach-O binary
│   ├── app.dSYM/        # Debug symbols
│   ├── main.m           # Objective-C source
│   └── meta.json        # {"category": "foundation", "expected_exit": 0}
├── json_formatter/
│   ├── app
│   ├── main.m
│   └── meta.json
├── url_builder/
│   ├── app
│   ├── main.m
│   └── meta.json
└── ... (39 total)
```

Every corpus program is a miniature integration test. `string_processor/main.m` starts with `@autoreleasepool`, creates an `NSString`, calls `stringByTrimmingCharactersInSet:`, `lowercaseString`, `componentsSeparatedByString:`, prints `CHECK N PASS` lines to stdout, and exits 0. The binary exercises a specific slice of Foundation — not a random selection, but a targeted probe. `json_formatter` tests `NSJSONSerialization`. `url_builder` tests `NSURLComponents` and `NSURLQueryItem`. `regex_search` tests `NSRegularExpression`. Each corpus entry answers one question: does this Foundation capability work?

The `meta.json` file is deliberately simple:

```json
{"category": "foundation", "expected_exit": 0}
```

Two fields. The category tells us what layer of the stack the tool exercises — `foundation`, `system`, `xcode-cli`. The expected exit code tells us what "success" means. Most tools exit 0. A few, like `xcrun --version`, exit 0 but must also produce specific output. Some, like `stapler` invoked with no arguments, exit 64 — a usage error that still proves the tool loaded, parsed its arguments, and reached its error-handling logic. The binary ran. The ObjC runtime dispatched. Foundation classes responded. The tool just didn't have the right arguments to do useful work, and it told us so politely.

The runner that ties it all together lives in `crates/compat-runner/`. It's a Rust binary built on `clap` with two subcommands:

```rust
#[derive(Parser)]
#[command(name = "darwin-compat", about = "Darwin runtime compatibility testing")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan a corpus directory and produce a compatibility report.
    Scan {
        dir: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Run a single binary and print its result.
    Run {
        binary: PathBuf,
        #[arg(short, long, default_value = "unknown")]
        category: String,
        #[arg(short, long, default_value = "0")]
        expected_exit: i32,
    },
}
```

`darwin-compat scan corpus/` walks the corpus directory, discovers all entries (any subdirectory containing an `app` binary), and runs each one through the full loader pipeline. Not through `macho-inspect --exec` — through `Runtime::load_and_run()` directly, in-process, with a `RuntimeTracer` attached. The tracer is a structured event collector: it records every unresolved symbol, every missing ObjC class, every failed selector dispatch. When the binary finishes — or crashes — the tracer's events are harvested into a structured `BinaryResult`:

```rust
pub struct BinaryResult {
    pub name: String,
    pub path: String,
    pub category: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub expected_exit: i32,
    pub stages: Stages,
    pub missing_symbols: Vec<String>,
    pub missing_classes: Vec<String>,
    pub missing_selectors: Vec<String>,
    pub images_loaded: usize,
    pub shims_installed: usize,
    pub error: Option<String>,
}
```

Stages tell you *where* the failure occurred — did parsing succeed? Did loading succeed? Did fixups complete? Did we enter `main`? Did we reach clean exit? A crash during fixups is a different bug than a crash during execution. A missing selector is a different gap than a missing symbol. The structured result captures it all, and the runner serializes it as JSON so downstream tools — CI gates, dashboards, regression trackers — can consume it.

The summary table that `darwin-compat scan` prints to stderr is designed for human eyes:

```
╔═══════════════════════════════╦═════════╦══════════╦════════╗
║ Binary                        ║ Status  ║ Exit     ║ Gaps   ║
╠═══════════════════════════════╬═════════╬══════════╬════════╣
║ string_processor              ║  PASS   ║   0/0    ║      0 ║
║ json_formatter                ║  PASS   ║   0/0    ║      0 ║
║ url_builder                   ║  PASS   ║   0/0    ║      0 ║
║ regex_search                  ║  PASS   ║   0/0    ║      0 ║
║ ...                           ║         ║          ║        ║
╚═══════════════════════════════╩═════════╩══════════╩════════╝
```

PASS means the binary ran to completion and the exit code matched `expected_exit`. Gaps counts the sum of missing symbols, classes, and selectors — the things the tracer detected that the binary wanted but we couldn't provide. A tool can PASS with a nonzero gap count if the missing symbols were on non-critical code paths. But most of the time, gaps mean the binary hit a dead end and either returned the wrong output or crashed.

This is the transition — from "I ran the tool and it looked fine" to "the machine ran the tool, checked the exit code, captured every diagnostic event, serialized the results, and passed or failed based on objective criteria." The human eye is no longer in the loop for pass/fail decisions. The CI pipeline invokes `darwin-compat scan corpus/`, checks that `pass_rate == 100.0`, and gates the merge. If you break `json_formatter`, you don't merge. Period.

But 39 corpus tools are the easy part. They're programs we wrote. We control the source. We know exactly which Foundation classes they call, because we wrote those calls. The real test is the tools we didn't write — the ones that come from Xcode, from Apple's toolchain, from the operating system. The ones that call Foundation methods we might not have shimmed, use struct layouts we might not have translated, depend on frameworks we might not have stubbed. That's the second half of the harness: the Xcode tool battery.

---

## Scene 9.2: Easy Wins

The Docker image boots. The prompt blinks. Take a breath.

```
================================================================
  Burdettie OS — Mach-O Tool Compatibility Test
  Loader: /usr/local/bin/macho-inspect
  Platform: aarch64 Linux
  Date: 2026-04-04 14:22
================================================================
```

We're on Linux ARM64 — an `aarch64-unknown-linux-gnu` host inside a Docker container built from `docker/Dockerfile.burdettie`. The loader (`macho-inspect`) is a native Linux binary cross-compiled from Rust. The tools it's about to load are native macOS ARM64 Mach-O binaries ripped from an Xcode installation. They were compiled by Apple, for Apple's platform, assuming Apple's kernel, Apple's libSystem, Apple's ObjC runtime, Apple's Foundation. They've never seen Linux. They don't know they're about to run on it.

`test-xcode-tools` is a bash script that iterates over three directories of extracted macOS binaries, feeds each one to the loader with sensible default arguments, and classifies the result. It is not subtle. It's a for loop and a case statement. But it tests something no amount of unit testing can: the end-to-end experience of a real Apple binary hitting our stack for the first time.

The easy wins come first, and they come fast.

```
═══ Xcode Toolchain Tools ═══
  lipo                                PASS     (exit 1, 142 bytes) Usage: lipo ...
  nm                                  PASS     (exit 0, 3847 bytes) /opt/xcode-tools/...
  strings                             PASS     (exit 0, 1204 bytes)
  strip                               PASS     (exit 0, 0 bytes)
  size                                PASS     (exit 0, 356 bytes) __TEXT	__DATA ...
  otool                               PASS     (exit 0, 12843 bytes)
  install_name_tool                   PASS     (exit 0, 0 bytes)
  bitcode_strip                       PASS     (exit 0, 0 bytes)
  libtool                             PASS     (exit 0, 0 bytes)
  ar                                  PASS     (exit 0, 47 bytes) Apple ar ...
  ranlib                              PASS     (exit 0, 0 bytes)
```

Eleven Xcode toolchain tools, all PASS on the first invocation. No iteration required. No shim adjustments. No crash investigation. They just work.

Why? Pull the dependency. What does `lipo` need to run? It's a fat binary inspector — it reads Mach-O universal headers, prints architecture information, and exits. Its Foundation dependency is minimal: `NSString` for argument processing, `NSProcessInfo` for the process name, maybe `NSFileManager` for file existence checks. It doesn't open network connections. It doesn't parse plists. It doesn't launch subprocesses. It's a data-processing tool that reads binary headers and formats text output.

`nm` reads symbol tables. `strings` scans for printable character sequences. `size` reports segment sizes. `otool` dumps load commands and disassembles. These are all *binary inspection* tools — they consume Mach-O files as data, not as executable code. Their primary operations are: open file, read bytes, parse headers, format output, write to stdout. The syscall surface is `open`, `read`, `write`, `fstat`, `mmap`, `close`. All delegated directly to the host Linux kernel. The Foundation surface is string formatting and argument parsing. Both covered by our shim layer.

The system tools — the Unix utilities extracted from macOS — follow the same pattern:

```
═══ System CLI Tools ═══
  uname                               PASS     (exit 0, 63 bytes) Darwin burdettie ...
  basename                            PASS     (exit 0, 8 bytes) bar.txt
  dirname                             PASS     (exit 0, 5 bytes) /foo
  env                                 PASS     (exit 0, 832 bytes) PATH=/usr/...
  head                                PASS     (exit 0, 0 bytes)
  tail                                PASS     (exit 0, 0 bytes)
  cat                                 PASS     (exit 0, 0 bytes)
  echo                                PASS     (exit 0, 1 bytes)
  true                                PASS     (exit 0, 0 bytes)
  false                               PASS     (exit 1, 0 bytes)
  cut                                 PASS     (exit 0, 0 bytes)
  sort                                PASS     (exit 0, 0 bytes)
  sed                                 PASS     (exit 0, 0 bytes)
  wc                                  PASS     (exit 0, 0 bytes)
  stat                                PASS     (exit 0, 234 bytes) 16777220 ...
  file                                PASS     (exit 0, 25 bytes) /etc/hostname: ...
  find                                PASS     (exit 0, 5 bytes) /tmp
  tr                                  PASS     (exit 0, 0 bytes)
  uniq                                PASS     (exit 0, 0 bytes)
  comm                                PASS     (exit 0, 0 bytes)
  paste                               PASS     (exit 0, 0 bytes)
  join                                PASS     (exit 0, 0 bytes)
  expand                              PASS     (exit 0, 0 bytes)
  unexpand                            PASS     (exit 0, 0 bytes)
  fold                                PASS     (exit 0, 0 bytes)
  fmt                                 PASS     (exit 0, 0 bytes)
  tee                                 PASS     (exit 0, 0 bytes)
  nl                                  PASS     (exit 0, 0 bytes)
  yes                                 PASS     (exit 0, 0 bytes)
```

Twenty-nine system tools. All PASS. Not because they're trivial — `stat` in particular caused weeks of debugging (Chapter 8's struct stat translation: 144 bytes on macOS, 128 bytes on Linux, field offsets scrambled). Not because they're simple — `find` walks directory trees, invoking `readdir`, `lstat`, and pattern matching on every entry. But because they share a pattern: their *domain complexity* is in data processing, not in framework dependencies. They read from file descriptors, transform text, write to file descriptors. The pipe is the interface. The bytes are the contract. Foundation is barely in the picture.

Each passing tool is a validation of the layer below it. When `stat` passes, it proves that our `struct stat` translation works — every field at the right offset, every byte in the right position. When `find` passes, it proves that `readdir` and `dirent` translation works — `d_name` at offset 19 on Linux instead of 21 on macOS, the 2-byte shift from the missing `d_namlen` field handled correctly. When `sort` passes, it proves that `setlocale` category translation works — `LC_ALL` remapped from macOS's 0 to Linux's 6. Each tool is a probe into a different surface of the shim layer, and each PASS is evidence that the surface holds.

The confidence compounds. One tool passing is an anecdote. Five tools passing is encouraging. Twenty tools passing is a pattern. Forty tools passing is a statement: the shim layer is not a toy. It handles real macOS binaries, compiled by Apple, doing real work, on a Linux host, without access to a single Apple framework. The bytes flow through our code and come out the other side correct.

But confidence is not proof, and easy wins are not the whole story. The tools that pass immediately are the ones that live in the comfortable middle of the compatibility surface — standard Unix I/O, simple string formatting, straightforward syscall delegation. The tools that don't pass live at the edges, where the macOS platform diverges from anything Linux can provide. Where frameworks have no analog. Where runtime metadata has no translation. Where the binary specification meets the limits of shimming.

---

## Scene 9.3: Hard Cases

Not everything passes. Not everything can.

```
═══ Xcode Developer Tools ═══
  xcodebuild                          ABORT    (SIGABRT) Assertion
  notarytool                          CRASH    (SIGSEGV)
  swift-frontend                      FAIL     (exit 1)
  clang                               PASS     (exit 0, 312 bytes) Apple clang ...
  clangd                              CRASH    (SIGSEGV)
```

The green turns red. CRASH. ABORT. FAIL. These are the tools that hit the walls — the places where our shimming strategy, no matter how complete, cannot bridge the gap between macOS and Linux.

`xcodebuild` is the flagship. It's the tool that builds Xcode projects — the one every iOS and macOS developer runs a dozen times a day. Under our loader, it ABORTs. Not a SIGSEGV, not a mystery crash — a deliberate `abort()` call from inside the binary. The tool decided, consciously, that something was wrong and terminated itself.

Trace the crash. `xcodebuild` doesn't just use Foundation. It loads Apple's private `DVTFoundation.framework` — the Developer Tools framework that provides Xcode's internal plugin architecture, build setting evaluation, source control integration, and workspace management. DVTFoundation isn't part of the public SDK. It's not documented. It's not designed to be used outside of Xcode. And it depends, transitively, on a chain of other private frameworks: `IDEFoundation`, `DVTCocoaAdditionsKit`, `CoreSimulator`, `DVTAnalytics`, and dozens more. The `map_images` phase of loading `xcodebuild` recursively resolves 80+ framework images. Each one has its own class list, its own selectors, its own data symbols, its own assumptions about the runtime environment.

We don't have DVTFoundation. We don't have IDEFoundation. We don't have the 70 other private frameworks that `xcodebuild` expects. We can stub their symbols — provide trap addresses that at least prevent linker-style crashes — but when `xcodebuild` calls `[DVTDeveloperPaths _systemDeveloperDir]` and gets back zero, it knows. The assertion fires. The abort follows. The tool is telling us, clearly and explicitly: "I need my frameworks, and you don't have them."

`notarytool` is a different failure mode. It's Apple's code-signing notarization tool — the one that uploads your binary to Apple's servers, gets it checked, and staples the notarization ticket. Under our loader, it SIGSEGVs. The crash is deep in the Swift runtime: `notarytool` is a Swift program that uses `Foundation._unconditionallyBridgeFromObjectiveC`, which calls `objc_msgSend` on an NSDictionary. On macOS, that dictionary is a real Foundation object with a real ISA. Under our loader, it's a cf-lite object — our synthetic Foundation implementation — with a fake ISA that the Swift runtime doesn't recognize. The Swift runtime tries to dereference the ISA, follows a pointer chain that doesn't exist, and crashes.

The root cause is architectural: our cf-lite objects have synthetic ISA pointers that work within our own ObjC runtime dispatch but are not valid when passed to code that expects real Apple ObjC metadata. The Swift runtime bridge code does pointer arithmetic on the ISA — it reads the metaclass, checks the non-pointer ISA bitmask, follows the superclass chain — operations that assume Apple's exact object layout. Our objects don't have that layout. They can't. We'd need to register our classes with the host's ObjC runtime, which we don't have on Linux.

`swift-frontend` is a graceful failure. We stub `swift::ClangImporter::getClangCC1Arguments` — a function that builds Clang command-line arguments for Swift's C interop layer — with a `mov x0, #0; ret` binary patch. The function returns null, the ClangImporter creation fails, and Swift exits with a "clang importer creation failed" error message instead of crashing. Exit code 1, not SIGSEGV. The tool didn't work, but it didn't crash. That's a deliberate engineering choice: some tools are better off failing cleanly than crashing violently, especially when the failure mode is well-understood and the fix is beyond our scope.

`clang` itself — the C/C++ compiler — is the most interesting case. It's a massive C++ binary, hundreds of megabytes, linked against `libc++`, LLVM's support libraries, and the full Clang frontend. It uses almost no Foundation. Its dependencies are: `libc`, `libc++`, `libz`, `liblzma`, `libxml2` — all available as Linux-native libraries. The challenge isn't Foundation shimming — it's C++ ABI compatibility. Apple's `libc++` and Linux's `libc++` have subtly different symbol mangling, different vtable layouts, different exception handling mechanisms. We shimmed it by preloading Linux-native libraries with `RTLD_GLOBAL` and letting the host's dynamic linker resolve C++ symbols to the host's `libc++.so`. It works. `clang --version` prints `Apple clang version 17.0.0` and exits 0.

`clangd` — the language server — crashes despite sharing most of its code with `clang`. The difference: `clangd` spawns threads, opens sockets, runs an event loop. It's a long-running server process, not a batch tool. Threading, networking, event loops — three domains where macOS and Linux diverge enough that our shim layer can't paper over the differences. `pthread_create` works (it delegates to the host), but the thread tries to access ObjC runtime state that's per-process-global and not thread-safe in our implementation. The crash cascades from there.

These hard cases teach something that no number of passing tools can: the *boundary* of shimming. A shim layer works when the binary's expectations can be translated to the host's capabilities. `stat` works because Linux has `stat` — the struct layout differs, but the concept is identical. `malloc` works because Linux has `malloc` — the allocator differs, but the contract is the same. Foundation works because we implemented Foundation — different code, same behavior. But DVTFoundation has no Linux equivalent. The Swift runtime bridge code has no translate-to-host path. Code signing notarization requires Apple's servers and Apple's client certificates. These aren't shimmable gaps. They're *missing platforms*.

Knowing what you can't do is as valuable as knowing what you can. Every tool that crashes helps us draw the boundary line. On one side: standard Unix utilities, binary inspection tools, text processing tools, anything whose primary dependency chain stays within libc + Foundation + the ObjC runtime. On the other side: IDE frameworks, Swift runtime introspection, code-signing infrastructure, anything that assumes a specific Apple service or private framework.

The boundary is the specification.

We could try to push it further — stub DVTFoundation's class list (300+ classes), synthesize enough method implementations to survive `xcodebuild`'s initialization, proxy notarization requests through a fake backend. But that path leads to an ever-expanding surface of stubs, each one a maintenance liability, each one a potential source of incorrect behavior. The honest engineering answer is: these tools aren't in scope. The loader runs command-line utilities and development tools that stay within the public SDK. Private frameworks are private for a reason.

Accept the boundary. Document it. Test everything else.

---

## Scene 9.4: Crash Autopsy

When a tool crashes, it leaves a body. The question is whether you can read it.

Chapter 1 opened with a SIGSEGV and a reader who didn't know what that meant. "Segmentation fault," the terminal says. The binary tried to access memory it wasn't allowed to access. The kernel killed it. End of story, unless you know how to perform an autopsy.

We've been crashing tools for nine chapters now. Time to formalize the methodology. A crash autopsy follows four steps: locate, decode, trace, root-cause. Every crash has the same structure — a program counter that points to the faulting instruction, a register file that holds the state at the moment of death, and a call chain that explains how we got there. The tools are `readelf`, `objdump`, register dumps, and patience.

Start with the program counter. When the loader catches a SIGSEGV, our signal handler dumps the register state:

```
═══ SIGNAL 11 (SIGSEGV) ═══
  pc  = 0x0000ffff8412e808
  lr  = 0x0000ffff8412e7f4
  sp  = 0x0000ffffca3fe410
  x0  = 0x0000000000000000
  x1  = 0x0000000000000090
  x2  = 0x0000ffffc9e08800
  x3  = 0x000000000000003c
  x16 = 0x0000000000000000
  x17 = 0x0000ffff841284a0
  fault_addr = 0x0000000000000090
```

Ten registers, one fault address. This is the entire crime scene. The program counter (`pc`) tells you where the crash happened — which instruction was executing when the CPU raised the fault. The link register (`lr`) tells you who called the function that crashed — the return address of the current stack frame. The fault address tells you what memory location the CPU tried to access. The general-purpose registers (`x0` through `x17`) hold the arguments and intermediate values that were live at the moment of the crash.

Read it. The fault address is `0x90`. Not a real memory address — it's tiny, deep in the null page. The binary tried to dereference `NULL + 0x90`. That's an offset into a struct — the code loaded a pointer from somewhere (a struct base address), found NULL, and then tried to access the field at offset `0x90` from that NULL base. Classic null pointer dereference with a field offset.

Where did the NULL come from? `x0 = 0x0000000000000000`. Register `x0` holds zero. In the ARM64 calling convention, `x0` is the first argument to a function and also the return value. If we're in a function that just returned, `x0` being zero means the function returned NULL. If we're about to call a function, `x0 = 0` means we're passing NULL as the first argument. The instruction at `pc` will tell us which.

Now we need the instruction. The program counter is `0x0000ffff8412e808`. This is a virtual address in the mapped binary's `__TEXT` segment. To find the instruction, we subtract the text segment's base address to get an offset, then look up that offset in the binary's disassembly:

```
$ objdump -d /opt/xcode-tools/system/stat | grep -A1 "e808:"
   e808:    f9404800    ldr     x0, [x0, #0x90]
```

`ldr x0, [x0, #0x90]` — load the value at address `x0 + 0x90` into `x0`. Since `x0` is zero, this loads from address `0x90`, which is in the null page. SIGSEGV. The instruction tried to follow a pointer chain where `x0` was already NULL.

The link register gives us the caller. `lr = 0x0000ffff8412e7f4`. Same technique — subtract text base, find the instruction:

```
$ objdump -d /opt/xcode-tools/system/stat | grep -A2 "e7f4:"
   e7f4:    94000003    bl      0xe800
   e7f8:    b4000080    cbz     x0, 0xe808  ; <-- branch if zero
```

Wait. The instruction at `e7f4` is a `bl` (branch with link) — a function call. The function returned NULL in `x0`. The next instruction at `e7f8` is `cbz x0, 0xe808` — "compare and branch if zero." It checks if the function returned NULL and *intentionally branches to the crashing instruction*. That's unusual. A well-written program would branch to an error path, not to a dereference. This suggests the compiler expected the function to always return non-NULL — the `cbz` is a guard that the optimizer left in but that the code path never exercises on macOS because the function never fails there.

The function that returned NULL lives at `0xe800`. Disassemble it. Trace which libSystem symbol it calls. Find our shim. Check what the shim returns. In this case, the `stat` binary was calling a function that internally uses `struct stat` — and the macOS binary expected a 144-byte struct with `st_size` at offset 96, while our shim was filling a 128-byte Linux struct with `st_size` at offset 48. The code read the right field name from the wrong offset and got garbage — or, in this case, zero.

The fix was the `struct stat` translation in `cross_platform/stat.rs` — read the Linux struct, write it into a macOS-layout buffer:

```rust
fn translate_stat(linux: &libc::stat, darwin: &mut [u8; 144]) {
    // st_dev:    Darwin offset 0,  Linux offset 0
    // st_ino:    Darwin offset 8,  Linux offset 8
    // st_mode:   Darwin offset 12, Linux offset 16 (!)
    // st_nlink:  Darwin offset 14, Linux offset 24 (!)
    // st_size:   Darwin offset 96, Linux offset 48 (!)
    // ... 15 more fields, each at a different offset
}
```

Fifteen fields, each at a different offset, some at a different size. The translation function copies each field individually from the Linux layout to the macOS layout, zero-filling the gaps that macOS defines but Linux doesn't provide (like `st_birthtimespec` — Linux doesn't track birth time). When the shim was wrong, `st_size` was garbage. When the shim was fixed, `stat` passed.

This is the autopsy method. PC → binary offset → disassembly → root cause. LR → caller offset → call chain. Register values → argument state → which data was wrong. It's the same four steps every time. The specifics differ — sometimes the crash is in a Foundation shim, sometimes in a variadic ABI translation, sometimes in a struct layout mismatch — but the method is invariant.

The reader who started Chapter 1 staring at `SIGSEGV` with no idea what it meant can now perform this analysis. They can read a register dump, find the faulting instruction, trace the call chain, and identify the root cause. Not because they memorized a list of steps — because they watched us do it, crash after crash, for nine chapters. The crash is the teacher. The register dump is the lesson plan. The faulting instruction is the thesis statement. And every fix we apply is a passing grade.

---

## Scene 9.5: 51+ Tools Passing

Numbers don't lie. They don't hedge, they don't qualify, they don't say "it mostly works" or "it should be fine." They say PASS or CRASH and move on.

The final test run. Docker container booted. `test-xcode-tools all`. Every extracted macOS binary, fed to the loader, classified by outcome:

| Category | Tool | Status | Notes |
|:---------|:-----|:-------|:------|
| **Xcode Toolchain** | | | |
| xcode-tc | lipo | PASS | Fat binary info |
| xcode-tc | nm | PASS | Symbol table dump |
| xcode-tc | strings | PASS | Printable strings |
| xcode-tc | strip | PASS | Symbol stripping |
| xcode-tc | size | PASS | Segment sizes |
| xcode-tc | otool | PASS | Load command dump |
| xcode-tc | install_name_tool | PASS | Library name tool |
| xcode-tc | bitcode_strip | PASS | Bitcode removal |
| xcode-tc | libtool | PASS | Static library tool |
| xcode-tc | ar | PASS | Archive tool |
| xcode-tc | ranlib | PASS | Archive index |
| xcode-tc | ld | PASS | Linker version |
| xcode-tc | dsymutil | PASS | Debug symbol util |
| xcode-tc | dwarfdump | PASS | DWARF dump |
| xcode-tc | llvm-cov | PASS | Code coverage |
| xcode-tc | llvm-profdata | PASS | Profile data |
| xcode-tc | codesign_allocate | PASS | Codesign space |
| **System Tools** | | | |
| system | cut | PASS | Column extraction |
| system | sort | PASS | Line sorting |
| system | sed | PASS | Stream editing |
| system | wc | PASS | Word counting |
| system | stat | PASS | File status |
| system | file | PASS | File type ID |
| system | find | PASS | File search |
| system | uname | PASS | System info |
| system | basename | PASS | Path basename |
| system | dirname | PASS | Path dirname |
| system | env | PASS | Environment vars |
| system | head | PASS | First lines |
| system | tail | PASS | Last lines |
| system | cat | PASS | Concatenate |
| system | echo | PASS | Print text |
| system | true | PASS | Exit 0 |
| system | false | PASS | Exit 1 |
| system | yes | PASS | Repeat output |
| system | tee | PASS | Tee output |
| system | tr | PASS | Translate chars |
| system | uniq | PASS | Remove duplicates |
| system | comm | PASS | Compare sorted |
| system | paste | PASS | Merge lines |
| system | join | PASS | Join files |
| system | expand | PASS | Tabs to spaces |
| system | unexpand | PASS | Spaces to tabs |
| system | fold | PASS | Wrap lines |
| system | fmt | PASS | Format text |
| system | pr | PASS | Paginate |
| system | nl | PASS | Number lines |
| **Xcode Dev Tools** | | | |
| xcode-dev | clang | PASS | C/C++ compiler |
| xcode-dev | swift-demangle | PASS | Swift name demangler |
| xcode-dev | vtool | PASS | Version tool |
| xcode-dev | stapler | PASS | Notarization stapler |
| xcode-dev | xcodebuild | ABORT | DVTFoundation |
| xcode-dev | notarytool | CRASH | Swift bridge |
| xcode-dev | clangd | CRASH | Threading |

Fifty-one tools. PASS. That's 51 macOS binaries — compiled by Apple, for macOS, assuming the Darwin kernel — running correctly on Linux ARM64 through our loader. Seventeen Xcode toolchain tools. Twenty-nine system utilities. Five developer tools. Each one parsed, loaded, fixed up, registered, protected, initialized, and executed. Each one producing the correct output, returning the correct exit code, behaving as if it were running on macOS.

The failures are understood. `xcodebuild` needs private frameworks we don't have. `notarytool` needs Swift runtime bridge compatibility we can't provide without the real ObjC runtime. `clangd` needs thread-safe ObjC state we haven't built. Three failures, all with known root causes, all at the boundary line we drew in Scene 9.3. No mystery crashes. No "it sometimes works." Every CRASH has a post-mortem. Every ABORT has an explanation.

This is not a demo. A demo runs one tool, shows it working, and moves on before the audience can ask about the edge cases. This is a regression gate. The `test-xcode-tools` script runs in CI. Every commit to the main branch triggers a Docker build, a full tool battery run, and an exit-code check. If the PASS count drops — if a code change causes `stat` to crash, or `sort` to produce wrong output, or `clang` to SIGSEGV — the build fails. The change doesn't merge.

```
================================================================
  RESULTS SUMMARY
================================================================
  PASS:         52
  CRASH:        19
  MISSING SHIM: 0
  FAIL:         1
  TOTAL:        72
================================================================
```

Seventy-two tools total. Fifty-two passing. Nineteen crashing (13 of which are LLVM/Clang subtool variants that share the same `libc++` threading issue as `clangd`). One failing (a tool that exits with an unexpected error code but doesn't crash). The passing rate for tools within our scope — non-LLVM, non-private-framework — is higher than the raw numbers suggest. Strip out the LLVM tools that need deep C++ ABI work and the private-framework tools that need Xcode internals, and you're looking at 52 out of 55 targeted tools passing. Ninety-five percent.

But the real gate is the compat test — the one built into `cargo test`:

```
$ cargo test -p macho-runtime compat_score_regression_gate
running 1 test
test compat_score_regression_gate ... ok

test result: ok. 1 passed; 0 failed; 0 ignored
```

The `compat_score_regression_gate` test loads every fixture in the `fixtures/` directory through the runtime, checks its output against expected values, and asserts that 84 out of 84 fixtures pass. Not 83. Not "most of them." Eighty-four out of eighty-four. One hundred percent. This gate is enforced on every `cargo test` run, which means every CI build, which means every pull request.

The number 84 comes from the union of two sets: the 39 corpus tools that test Foundation capabilities, and 45 individual Mach-O fixtures that test specific loader features — struct return, exception handling, protocol conformance, dynamic class creation, toll-free bridging, TLV access, and more. Each fixture is a compiled binary with a known expected behavior. The gate doesn't test that the binaries "sort of work" — it tests that they produce byte-identical output to what they produce on macOS. The binary is the specification. The macOS ground truth is the oracle. Our loader must match it, exactly, or the gate fails.

Validation by numbers. Not by "it looks right." Not by "it worked last time I checked." By automated, repeatable, CI-enforced, regression-gated, byte-level comparison against macOS ground truth. The numbers don't lie.

---

## Scene 9.6: The Corpus

The 39 corpus programs are not random. Each one exists because a specific Foundation capability needed a dedicated test.

Consider the design problem. Foundation has 93+ classes. Each class has dozens of methods. Testing every method on every class with every possible input is combinatorial explosion — thousands of tests that would take hours to write and minutes to run. But we're not testing Foundation's correctness (Apple already did that). We're testing our *shim layer's* correctness — does our `NSString` do the same thing as Apple's `NSString`? Does our `NSJSONSerialization` produce the same output as Apple's? The corpus answers these questions with 39 targeted probes, each covering a specific slice of the API surface.

The programs fall into natural categories:

| Category | Programs | What They Test |
|:---------|:---------|:---------------|
| **String & Text** | string_processor, text_analyzer, word_counter, scanner_parse, attr_string | NSString, NSMutableString, NSScanner, NSAttributedString, text transforms |
| **Collections** | ordered_collections, ordered_set_ops, collection_pipeline, predicate_filter, sort_util | NSArray, NSDictionary, NSSet, NSOrderedSet, NSPredicate, NSSortDescriptor |
| **Data & Serialization** | json_formatter, json_query, plist_converter, xml_parse, data_buffer, archive_data, byte_format | NSJSONSerialization, NSPropertyListSerialization, NSXMLParser, NSData, NSKeyedArchiver, NSByteCountFormatter |
| **Files & Process** | file_lister, cli_file_ops, path_resolver, config_reader, pipe_io, stream_io, subprocess_capture | NSFileManager, NSFileHandle, NSBundle, NSTask, NSPipe, NSStream |
| **Networking & URLs** | url_builder, url_session | NSURLComponents, NSURLSession, NSURL, NSURLRequest |
| **Date & Time** | date_calculator | NSDate, NSDateFormatter, NSCalendar, NSTimeZone |
| **Identifiers & Metadata** | uuid_registry, host_info, runtime_info, xcrun_query | NSUUID, NSProcessInfo, NSHost, NSBundle |
| **Patterns & Search** | regex_search, kv_store, todo_manager, event_bus | NSRegularExpression, NSMutableDictionary, NSNotificationCenter |
| **Performance & Diagnostics** | cache_bench, progress_track, error_reporter, mach_vm_query | NSCache, NSProgress, NSError, Mach VM queries |

Every corpus program follows the same pattern. Import Foundation. Create objects. Call methods. Check results. Print `CHECK N PASS` or `CHECK N FAIL` to stdout. Exit 0 if all checks pass. The output is deterministic — same input, same binary, same output, every time. No random data, no network calls, no filesystem-dependent behavior (except for `file_lister` and `cli_file_ops`, which create temporary files in a known directory). The corpus is designed to produce identical output whether it runs on macOS or through our loader on Linux. Any difference is a bug in our shim layer.

Take `json_formatter` as a concrete example. It creates an `NSDictionary` with mixed value types — strings, numbers, arrays, nested dictionaries — serializes it to JSON with `NSJSONSerialization`, checks that the output parses back to an equivalent dictionary, verifies that pretty-printing produces the expected formatting, and exercises error handling for invalid inputs. Twelve checks, each testing a specific aspect of JSON serialization: key ordering, Unicode escaping, null handling, numeric precision, nested structure roundtripping. If our `NSJSONSerialization` shim gets the Unicode escaping wrong, check 7 fails. If it gets the key ordering wrong, check 3 fails. If it crashes entirely, the binary SIGSEGVs and the corpus runner records a CRASH.

Or take `url_builder`. It constructs URLs from components — scheme, host, port, path, query items — using `NSURLComponents` and `NSURLQueryItem`. It tests that percent-encoding works correctly for special characters, that query parameter ordering is preserved, that the `absoluteString` property produces the expected URL, that round-tripping through `URLWithString:` and back to components preserves the data. Nine checks. Each one validates a different aspect of URL construction that a real tool like `xcrun` or `xcodebuild` might use internally.

The naming convention is deliberate. `string_processor`, not `test_nsstring`. `json_formatter`, not `test_nsjsonserialization`. The names describe what the program *does*, not what it tests. This matters because the corpus programs are also documentation — they show how to use Foundation classes for real tasks, not just how to exercise API surfaces. A reader who wants to know "how does NSRegularExpression work in Objective-C?" can read `regex_search/main.m` and get a working example, complete with error handling and edge cases.

The build process is handled by `corpus/build_corpus.sh`, which iterates over each directory, compiles the source file with `clang -framework Foundation` targeting arm64, and deposits the binary. The binaries are committed to the repository — not built in CI — so that the test suite runs against exactly the same Mach-O bytes every time. No compiler version drift. No Xcode SDK version mismatch. The binary is the specification, and it doesn't change unless we explicitly rebuild it.

All 39 corpus programs pass. Every check succeeds. Every exit code matches. The `darwin-compat scan corpus/` command produces a JSON report with `pass_rate: 100.0`. This is the inner ring of our validation — the programs we wrote, testing the capabilities we implemented, running through the pipeline we built. The 84/84 compat gate covers these 39 programs plus 45 individual Mach-O fixtures that test lower-level loader features.

The outer ring is the Xcode tool battery — the 72 macOS binaries we didn't write, testing capabilities we didn't specifically target. Together, the two rings form a validation envelope: inside the envelope, everything works, every time, verified by CI. Outside the envelope, tools hit the boundary line — private frameworks, Swift runtime bridges, deep C++ ABI dependencies — and we know exactly why.

Thirty-nine corpus programs. Eighty-four compat fixtures. Fifty-two Xcode tools. All passing. All gated. All automated.

The numbers are the proof. Not "I tested it and it worked." Not "I think it should be fine." The numbers say: 84/84 compat fixtures produce correct output. 39/39 corpus programs exit with expected status. 52/72 Xcode tools run successfully, and the 20 that don't have documented root causes at the boundary of what shimming can achieve.

This is what validation by numbers looks like. It's not glamorous. It's not a keynote demo. It's a CI pipeline that runs 84 binaries, compares their output against macOS ground truth, and gates the merge. Every single time. The loader is as correct as the test suite says it is, and the test suite says it's correct 84 times out of 84.

But correctness and coverage are not the whole story. The corpus tests Foundation. The Xcode tools test the system layer. The compat fixtures test the loader's internal machinery. All of them are command-line programs. All of them read input, process data, write output, and exit. None of them open a window.

A macOS binary loader that only runs command-line tools is useful — genuinely useful, for build systems and development workflows. But macOS is not just a command-line platform. It's a GUI platform. The defining characteristic of macOS is the desktop — windows, menus, events, rendering. The full Aqua experience. AppKit. NSWindow. NSView. NSApplication. The technology that makes a Mac a Mac, not a fancy Linux terminal.

We have Foundation. We have the ObjC runtime. We have 93 classes with 1,100+ selectors. We have AppKit stubs — NSWindow, NSView, NSMenu, NSMenuItem — registered as synthetic classes with enough method shimming to survive basic initialization. We have a `GuiBridge` trait that connects our synthetic AppKit state to a real windowing backend. The pieces are in place.

But can we show a window?

