# Component reuse and integration decisions

Current strategy: [ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md),
2026-11-25. This file's historical name is retained for links. It is no longer a
checklist for replacing macOS/PureDarwin services or a kernel with our own Rust
implementations.

## Disposition of existing work

| Area | Keep and use now | Reuse-first next decision |
| --- | --- | --- |
| Mach-O parsing, mapping and diagnostics | Existing Rust tools and source-owned fixtures | Compare upstream parsing/loading where it reduces maintenance; preserve diagnostic/behavior contracts |
| Runtime, libSystem shims and interpreter | Current supported baseline and regression knowledge | Evaluate a complete upstream runtime; add narrow adapters or shared fixes before expanding our own syscall/Mach implementation |
| Objective-C and Foundation/AppKit subsets | Existing working fixtures and UI/bridge behavior | Select a coherent upstream object/runtime model; migrate only after ownership, ABI and workflow qualification |
| Linux platform services | Existing PAL backend and host integration | Use Linux libraries/APIs directly through narrow Rust boundaries; no kernel replacement |
| PureDarwin libraries/toolchain | Portable components and existing build/test knowledge | Small native packages, shared regression tests and SDK fixes; port only when the dependency boundary is understood |
| Dispatch, crypto, text and media | Maintained existing implementations and regression fixes | Prefer original upstream Linux libraries rather than duplicating their algorithms |
| Developer tooling | Existing project parsers and narrow CLI frontends | Delegate compile/build/index/debug to upstream tools; own the workflow/UX |
| Desktop | Existing GUI bridge and demos | Integrate with an established Linux session/compositor/toolkit; own the product experience |
| Trust and supervision | Existing policy/admission contract and useful code | Rust owner-policy adapters plus qualified Linux controls and maintained verification libraries |

A component's presence or historical completion label does not prove semantic
coverage or confinement. Each adopted replacement must pass the declared
positive/negative behavior, lifetime, architecture, security and recovery
checks. Keep old and new behavior distinguishable during evaluation; remove
obsolete duplication after qualification.

The [research register](RESEARCH_QUESTIONS.md) records the unresolved choices.
The [upstream strategy](UPSTREAM_STRATEGY.md) identifies contribution routes and
first candidates. No percentage of Rust implementation or independence from
all external libraries is a release metric.

The previous implementation inventory and its historical counts are available
in [Git history](https://github.com/burdettadam/burdettie-os/blob/a55b71930e5aee279214daa934e084dba8863cfa/docs/BURDETTIE_REPLACEMENT_TARGETS.md).
