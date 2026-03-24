# Loader Plan

How the macOS dynamic linker (dyld) loads a Mach-O executable.

## Simplified dyld startup sequence

1. Kernel maps the executable and dyld into the process
2. dyld reads the main executable's load commands
3. For each LC_LOAD_DYLIB, dyld locates and maps the dylib
4. Recursively load transitive dependencies
5. Perform rebasing (ASLR slide adjustment)
6. Perform binding (resolve undefined symbols)
7. Run initializers in bottom-up dependency order
8. Transfer control to LC_MAIN entry point

## What we model

| Step | Crate | Status |
|---|---|---|
| Read load commands | macho-parse | Phase 1 |
| Map segments | macho-map | Phase 2 |
| Locate dylibs | macho-resolve | Phase 3 |
| Dependency graph | macho-resolve | Phase 3 |
| Init order | macho-run-plan | Phase 3 |
| Rebasing | — | Future |
| Symbol binding | — | Future |
| ObjC registration | macho-objc | Phase 5 |
| Entry transfer | macho-run-plan | Phase 2 |

## ASLR / Slide

Modern macOS uses ASLR. The kernel picks a random slide value and all vmaddrs
in the binary are offset by that amount. Our mapping plan records the file's
preferred vmaddrs; the slide is applied at map time.

## Chained fixups vs. classic dyld info

Newer binaries (arm64e, macOS 12+) use LC_DYLD_CHAINED_FIXUPS instead of
LC_DYLD_INFO_ONLY. We need to support both formats eventually.
