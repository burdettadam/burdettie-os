# ADR 0003: Linux foundation, Rust interfaces, upstream reuse

Status: **accepted product strategy**, 2026-11-25.

## Decision

Build and ship Burdettie OS on **Linux**. Linux is the product kernel and runtime
foundation, including on our own hardware; it is not merely a development host.
Burdettie owns the desktop experience, developer workflows, application
integration and owner-selected trust policy. Reuse maintained open-source
implementations and contribute generally useful changes to their upstreams.
Use Rust for owned interfaces, compatibility bridges, supervision and product
logic where its type system, ownership and explicit error handling help.

This decision supersedes [ADR 0001](0001-darwin-boot-substrate.md) and
[ADR 0002](0002-rust-platform-transition.md). PureDarwin/XNU is no longer the
selected shipping substrate, and an owned Rust kernel/core-userland replacement
is no longer a destination or prerequisite. The former R1-R6 replacement track
is withdrawn. Historical source/build/VM evidence remains valid only for its
recorded inputs and scope; it does not direct current product work.

The product goal remains a bootable system with our own UI and developer
experience, native Linux applications, named macOS application workflows on
supported compatibility routes, and owner-selectable permissive, audit and
enforced policy. Compatibility is qualified per application version,
architecture, backend and workflow. This decision does not claim universal
macOS compatibility or completed Linux enforcement.

## Architecture and ownership

| Area | Foundation to reuse | Burdettie responsibility |
| --- | --- | --- |
| Kernel, drivers and OS services | Linux and an existing Linux distribution/build ecosystem | Product configuration, image/session integration and target qualification |
| Darwin compatibility | Evaluate maintained upstream runtimes and selected PureDarwin components | Rust launch/admission adapters, diagnostics, ABI boundaries and measured integration gaps |
| Native desktop | Existing Wayland/session, graphics, input and audio components | Our UI, application experience and policy UX |
| Developer environment | Cargo/rustc, LLVM/Clang/LLDB, language servers, Swift toolchain where supported | Project import, toolchain discovery, build/run/debug orchestration and editor UX |
| Trust and confinement | Linux process boundaries and qualified OS controls; maintained verification libraries | Owner authority, grants, policy translation, capability reports and broker authorization |

PureDarwin is a source, integration and test partner. Reuse components that
benefit the Linux product; use an existing Linux implementation directly when
one is available. A Linux-hosted build of a Darwin library does not make that
library Linux-native. XNU-only dependencies need an adapter, a compatible
runtime or a different component. PureDarwin's Linux-on-XNU `elftrans` is the
reverse direction of our product requirement.

An optional PureDarwin VM may qualify a shared upstream fix or compare behavior.
It is not on the shipping path and does not block Linux milestones. Retain
existing macOS observations as reference evidence with their original scope.

## Rust's role

Prefer small Rust interfaces that make ownership, lifetimes, allowed states,
resource budgets and errors explicit. Examples include typed handles to foreign
resources, bounded binary/IPC parsing, launch state transitions, asynchronous
process supervision and narrow adapters to upstream APIs.

Keep unsafe and FFI obligations explicit: layout and calling convention,
pointer validity, ownership/release pairing, threading, callbacks, unwind and
panic behavior. Preserve foreign object-model boundaries. Do not expose an
incidental Rust layout as a stable external ABI or mix Objective-C runtimes
without a demonstrated object/ownership contract. Rust around a native guest or
C library does not make that foreign code memory-safe or isolate the process.

Rust is not a requirement to rewrite a working kernel, driver, compiler,
debugger, compositor, crypto library or framework. A new implementation needs a
recorded unmet requirement and an adoption/contribution experiment explaining
why the existing options are insufficient. Small product-specific adapters do
not require a separate approval ceremony.

## Preserve useful work; choose implementations by evidence

The existing Mach-O loader, Rust shims, Linux platform backend, interpreter,
framework work, CLI tools and fixtures are reusable assets. They are not all
mandatory permanent implementations. Preserve working paths while comparing
upstream alternatives on the same fixtures; replace or retire an implementation
only after the selected alternative satisfies its declared behavior and
recovery requirements.

The current bridge implements selected imported APIs, a small synthetic Mach
message/bootstrap surface and Linux host services. Raw interpreted ARM64 SVC
and general Mach IPC remain gaps. Evaluate a complete upstream compatibility
worker first; a direct `darlingserver` adapter is a separate experiment, not an
assumed drop-in solution. **Darling has not been adopted by this ADR.**

Keep native Linux ELF development and Darwin Mach-O compatibility as explicit
target profiles. Our Xcode-like experience should orchestrate existing tools;
recreating all of Xcode, its private frameworks or Apple's compiler/debugger
stack is not a product requirement.

## Security contract

Preserve [launch-policy-v1](../contracts/launch-policy-v1.md). Platform choice,
signer requirements and resource policy remain separate. The owner chooses
trust roots and grants; the supervisor authenticates those choices. Audit
reports policy decisions without applying optional policy restrictions.
Enforced launch refuses unavailable required controls before application
initialization in the worker and never silently downgrades.

Qualify native/interpreted guest execution, descendants, dynamic libraries and
every service acting on an app's behalf. A compatibility prefix, successful
sandbox stub, code-signature check or Rust implementation alone is not an OS
security boundary. The [Linux allocation](../contracts/LINUX_BACKEND_ALLOCATION.md)
identifies candidate mechanisms without promoting them to supported controls.

## Implementation and outstanding decisions

The [roadmap](../roadmap.md) replaces the former kernel-transition schedule.
The [research register](../RESEARCH_QUESTIONS.md) records current findings,
decision owners, bounded experiments and exit criteria. The
[upstream strategy](../UPSTREAM_STRATEGY.md) governs component selection and
contribution routing. The first evaluation lane is Linux x86_64; existing ARM64
tests remain useful, while additional shipping architectures need qualification.

The release distribution/image builder, desktop integration, compatibility
backend and exact application versions remain open decisions. They do not
reopen the choice of Linux or create a requirement for a private replacement
stack. Use existing development resources and preserve their cost/capacity
bounds; this strategy authorizes no new paid infrastructure.

Success is useful application/developer workflows, qualified owner control,
maintained upstream dependencies and a manageable downstream patch set. Lines
of rewritten code, a Rust-only percentage or removal of every external
dependency are not success metrics.

## Evidence

- [Original host-kernel strategy](https://github.com/burdettadam/burdettie-os/blob/d59ace0bc41f93a251c25ee849ec98fe0fda9aa0/docs/NEXT_GENERATION_ROADMAP.md)
- [Linux platform implementation](../../crates/platform-linux/src/lib.rs), [current resolver](../../crates/macho-runtime/src/resolver.rs), [SVC limitation](../../crates/arm64-translate/src/interp/exec_dp.rs)
- [PureDarwin scope](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/README.md) and [contribution policy](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/CONTRIBUTING.md)
- [Rust FFI guidance](https://doc.rust-lang.org/nomicon/ffi.html)
- [Darling system-call emulation](https://docs.darlinghq.org/internals/basics/system-call-emulation.html)
