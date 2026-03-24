# Mach-O Notes

Working notes on the Mach-O binary format as encountered during development.

## File structure

```
┌─────────────────────┐
│   Mach Header        │  magic, cpu_type, file_type, ncmds, flags
├─────────────────────┤
│   Load Commands      │  LC_SEGMENT_64, LC_LOAD_DYLIB, LC_MAIN, ...
├─────────────────────┤
│   __TEXT segment     │  executable code, string literals, stubs
├─────────────────────┤
│   __DATA segment     │  writable globals, GOT, lazy pointers
├─────────────────────┤
│   __LINKEDIT segment │  symbol tables, string tables, code signature
└─────────────────────┘
```

## Key load commands

| Command | Purpose |
|---|---|
| LC_SEGMENT_64 | Defines a memory-mapped region |
| LC_LOAD_DYLIB | Declares a dylib dependency |
| LC_ID_DYLIB | Identifies this file as a dylib |
| LC_MAIN | Entry point (offset from __TEXT) |
| LC_RPATH | Adds an rpath search directory |
| LC_SYMTAB | Points to symbol + string tables |
| LC_DYSYMTAB | Dynamic symbol table info |
| LC_UUID | Unique build identifier |
| LC_BUILD_VERSION | Platform + SDK version |
| LC_DYLD_INFO_ONLY | Bind/rebase/export info |
| LC_DYLD_CHAINED_FIXUPS | Modern chained fixups format |

## Path variables

- `@executable_path` — directory containing the main executable
- `@loader_path` — directory containing the Mach-O that has the LC_LOAD_DYLIB
- `@rpath` — searched against LC_RPATH entries

## Version encoding

Versions in LC_LOAD_DYLIB are packed as u32: `(major << 16) | (minor << 8) | patch`

## Useful references

- Apple: "Mach-O Programming Topics"
- Apple: "dyld source code" (opensource.apple.com)
- `<mach-o/loader.h>` — canonical header definitions
