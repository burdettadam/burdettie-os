# Burdettie OS — Linux product roadmap

Strategy revision: **linux-2026-11-25**, accepted under
[ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md).

## Product direction

Build a bootable Linux-based Burdettie OS with our UI, developer environment,
owner-selected trust policy, native Linux applications and named macOS
application workflows through qualified compatibility routes. Reuse maintained
open-source components and contribute shared fixes upstream. Prefer Rust for
interfaces, bridges, supervision and product behavior.

Linux is the shipping kernel and hardware foundation. PureDarwin supplies
selected source, SDK/tooling knowledge and shared tests; XNU is optional
reference infrastructure. The former XNU-product and owned-Rust-kernel
transition decisions and R1-R6 schedule are withdrawn. Existing runtime code is
an asset to compare and integrate, not a requirement to preserve every custom
subsystem indefinitely.

## Immediate work

1. Establish the current Linux baseline with exact code, environment, fixtures
   and native/interpreted execution routes. Separate passing, failing, skipped
   and missing-input cases. Do not inherit qualification from older receipts.
2. Compare the complete upstream Darling worker with the existing Burdettie
   bridge on a small, fixed same-ISA corpus. Keep backend adoption open until
   compatibility, maintenance and authority boundaries are understood.
3. Reproduce one shared upstream problem, beginning with the researched
   PureDarwin SDK-discovery inconsistency or a portable library package/test.
4. Integrate a native developer workflow using existing compiler, build,
   language-service and debugger interfaces.
5. Select and qualify a Linux image/session foundation and the controls required
   for owner policy. Native product progress and upstream contributions can
   proceed while broader Apple application compatibility remains incomplete.

The [research register](RESEARCH_QUESTIONS.md) defines findings, owners,
experiments and exit criteria. The [upstream strategy](UPSTREAM_STRATEGY.md)
defines contribution routing and dependency records.

## Milestones

M1-M7 are retained as product milestone identifiers, rebaselined to this strategy
revision. Older M1/PureDarwin receipts are historical; this table does not relabel
an old pass as a Linux pass or close any milestone automatically.

| Milestone | Deliverable | Completion evidence |
| --- | --- | --- |
| M1 — Linux foundation and integration choices | Pinned Linux evaluation target, existing runtime baseline, bounded upstream-worker comparison, selected image/session experiment and backend allocation | Actual source/environment identities; measured build/runtime results; explicit decisions and blockers; shared contracts reviewed |
| M2 — Shared launch and admission | Rust supervision/admission adapters for declared Linux native and Darwin compatibility routes | Authenticated owner policy, immutable input identity, correct lifecycle, truthful capability reports and refusal of unsupported enforced requests |
| M3 — Desktop and developer experience | Burdettie session/UI on reused Linux desktop services; native build/run/test/LSP/DAP workflow | Boot/session start, input/resize/clipboard, process recovery, correct compiler context and a working native debugger workflow |
| M4 — Named application workflows | Fixed macOS application/version plus a source-owned document fixture | Real edit/save/reopen and error behavior on the selected Linux compatibility route; exact dependency and CPU scope |
| M5 — Qualified owner controls | Linux restrictions and broker authorization for declared routes | Actual denied operations, descendants, inherited handles, direct syscalls and server-mediated requests; separate audit/permissive evidence |
| M6 — Persistence, updates and target hardware | Recoverable profiles/data, reproducible updates and a declared hardware matrix | Clean reboot persistence, failed-update/configuration recovery, device-specific driver/session results |
| M7 — Release and upstream maintenance | Reproducible Linux-based image, supported workflow matrix and maintained dependency/patch inventory | All declared target gates pass; source/notices retained; dependency update exercised; unsupported cases visible |

M1 and M2 can progress together. A full AppKit port, every macOS application,
full Xcode replication, an owned compiler/debugger/compositor and a custom kernel
are not prerequisites. Select small useful targets, prove their behavior and
expand from evidence.

## Reuse decisions

| Component | Current action |
| --- | --- |
| Linux kernel and drivers | Adopt upstream/distribution implementations; qualify exact hardware/configuration |
| Darwin execution | Evaluate upstream worker first, then a narrow direct adapter only if justified; Darling is not yet selected |
| Existing loader/shims/interpreter | Retain as tested baseline, diagnostic tools and adapters; retire duplication only after replacement gates |
| Frameworks and Objective-C runtime | Audit a coherent dependency/object model; prefer reuse and shared fixes over blanket Rust reimplementation |
| Developer tools | Delegate to Cargo/rustc, LLVM/Clang/LLDB, language servers and supported Swift tools |
| Desktop | Reuse an established Linux session/compositor; own our UX without requiring our own display server |
| Trust and enforcement | Owner policy in Rust; verified Linux mechanisms and brokers; reuse maintained verification libraries |
| PureDarwin build/VM tools | Optional upstream qualification and reference work with explicit scope, not shipping dependencies |

See [component decisions](BURDETTIE_REPLACEMENT_TARGETS.md) and
[developer-tool plan](XCODE_PUBLIC_COMPATIBILITY_PLAN.md).

## Evidence, cost and completion rules

Pin exact dependencies and test inputs. A build is not an application pass;
a compatible function name is not semantic parity; a sandbox/signature success
is not resource confinement. Keep native ELF, native Mach-O and interpreted
Mach-O results separate. Required Linux policy capabilities must be discovered
and established on the actual kernel, not inferred from a version label.

Use existing development resources and preserve their bounds. Preflight disk,
memory, CPU/time and artifact provenance before large upstream builds. This
roadmap adds no paid infrastructure or broad unbounded build campaign. A
resource-blocked experiment remains blocked; it is not a reason to rewrite the
subsystem privately.

Success measures are working workflows, reliable owner control, upstream
contributions and a manageable patch queue. Track submissions and review
outcomes honestly; upstream acceptance is not guaranteed. Do not erase existing
regression fixes while changing strategic direction.

## Historical references

- [Former roadmap at the prior source](https://github.com/burdettadam/burdettie-os/blob/a55b71930e5aee279214daa934e084dba8863cfa/docs/roadmap.md)
- [Archived runtime roadmap](archive/NEXT_GENERATION_ROADMAP.md)
- [Superseded boot decision](adr/0001-darwin-boot-substrate.md)
- [Withdrawn Rust-platform transition](adr/0002-rust-platform-transition.md)
- [Historical PureDarwin source/build evidence](milestones/M1_PUREDARWIN_SOURCE_REFRESH_20261123.md)

Historical completion percentages, future dates and implementation inventories
are not current product commitments. ADR 0003 and this roadmap govern new work.
