# Burdettie OS — Linux product roadmap

Strategy revision: **linux-2026-11-25**, accepted under
[ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md), with integration
boundaries refined by [ADR 0004](adr/0004-linux-integration-boundaries.md).

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

1. Establish the complete upstream Darling core/system route on a suitable
   Linux x86_64 host, using thin source-owned fixtures. Use its existing
   rootless/unprivileged paths first and defer optional frameworks. Compare
   with our current bridge before adding a Rust supervisor adapter. Host-level
   launcher probes are not runtime qualification.
2. Qualify one application/profile as a complete runtime domain, including its
   server, prefix, launchd, descendants and helpers. Keep owner policy outside.
   Compare a rootless activation barrier with an existing outer launcher and
   non-root copy mode. Preserve enforced refusal where controls are missing.
3. Turn the reproduced PureDarwin SDK wrapper inconsistency and libDER failed
   semantic expectations into focused regression/contract work. Hold libDER
   trust-parser adoption; a Linux build is insufficient. Follow actual upstream
   contribution rules and route fixes to the component owner.
4. Consolidate the working host-backed GUI event-loop path, then prototype our
   panel/launcher as a separate SCTK layer-shell client on unmodified labwc.
   Evaluate Debian 13/mkosi for the image. Boot/session and release selection
   remain open; NixOS remains a release contender.
5. Evaluate Theia's existing workbench with a small Burdettie extension/Rust
   tool interface. Complete native Rust/C build, tests, LSP and DAP acceptance
   in a trusted development route. Preserve native progress while Apple
   application compatibility remains incomplete.

The [research register](RESEARCH_QUESTIONS.md) defines findings, owners,
experiments and exit criteria. The [upstream strategy](UPSTREAM_STRATEGY.md)
defines contribution routing and dependency records.

## Research results shaping this sequence

| Evidence | Result and consequence |
| --- | --- |
| [Darling launcher/source probes](research/2026-11-25-darling-runtime.md) | Rootless, non-root copy and relocatable unprivileged paths exist; eight local test programs compiled, six passed and two failed under observed environment restrictions. Establish a complete core/system control before custom launch infrastructure. |
| [Existing AWS host preflight](research/2026-11-25-aws-host.md) | Compiler-free socket, memory/ptracer, namespace and a minimal Landlock check succeeded in the recorded context. Free disk was below the 8 GiB reserve, so no runtime/framework build or dependency installation was performed. |
| [Linux mechanism probes](research/2026-11-25-linux-enforcement.md) | Selected inherited seccomp denial worked; several runtime/enforcement primitives were blocked. Writable/executable memfd aliases defeated a small two-check W^X filter. Full enforcement remains open. |
| [PureDarwin reuse probes](research/2026-11-25-puredarwin-reuse.md) | SDK reporting mismatch reproduced in a rendered wrapper; libDER built on Linux but passed only 28 of 30 semantic expectations. Regressions/contract fixes precede trust-parser adoption. |
| [GUI source review](research/2026-11-25-linux-desktop-devtools.md) | Active demo apps use host-backed `window-backend-darwin::GuiApp`; the separately named Linux polling backend lacks event dispatch. Application windows and desktop shell clients need separate interfaces. |
| [Native development/image-tool probes](research/2026-11-25-linux-desktop-devtools.md) | Native C build/edit/run and structured diagnostics passed; mkosi accepted a candidate configuration. No Rust/LSP/DAP/IDE, image boot or session qualification was performed. |

These results refine implementation choices without closing M1-M7. Preserve
the distinction between a source finding, a narrow mechanism test and a
qualified application route. Do not inherit an old pass or a research-host
kernel capability as a product claim.

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
| Darwin execution | Evaluate complete core/system rootless upstream route first; optional frameworks and direct adapters follow evidence; Darling is not yet adopted |
| Existing loader/shims/interpreter | Retain as tested baseline, diagnostic tools and adapters; retire duplication only after replacement gates |
| Frameworks and Objective-C runtime | Audit a coherent dependency/object model; prefer reuse and shared fixes over blanket Rust reimplementation |
| Developer tools | Evaluate Theia workbench/extensions and delegate to Cargo/rustc, LLVM/Clang/LLDB, language servers and supported Swift tools |
| Desktop | Prototype separate SCTK shell clients over unmodified labwc; consolidate the active winit application path; no owned compositor requirement |
| Image | Prototype Debian 13/mkosi; compare NixOS when runtime packaging and recovery evidence exist; no release image qualified yet |
| Trust and enforcement | Owner policy in Rust outside per-application whole-runtime domains; qualify Linux controls and brokers; hold libDER trust use until semantic gates pass |
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

Prototype selection does not guarantee upstream modification fit. Labwc's
current policy rejects AI-generated contribution content and its feature scope
is conservative. Prefer unmodified reuse there and generally useful client
work in SCTK; reassess the compositor if a named workflow requires changes
outside its scope. Theia's workbench APIs and extension compatibility require
their own measured gate. These constraints belong in dependency selection,
not in a hidden commitment to maintain a private fork.

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
are not current product commitments. ADRs 0003/0004 and this roadmap govern
new work.
