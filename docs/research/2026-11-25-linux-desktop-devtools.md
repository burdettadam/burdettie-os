# Linux desktop, image and developer-tool research

Research date: 2026-11-25 (America/Denver). Burdettie source inspected:
`d264e02f379b8fd4c4205bc416ea060064ad59fd`. This report advances RQ-04, RQ-05
and the GUI part of RQ-08 under [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md).

## Decisions this evidence supports

1. Separate **application windows**, **desktop shell clients**, and the
   **compositor** in the architecture. Reuse an existing compositor. Our current
   winit code is an application-window integration, not a compositor or complete
   desktop shell.
2. Consolidate the working host-backed GUI/event-loop path before expanding
   custom GUI infrastructure. Keep the event loop on its owning thread and use
   commands/proxies across threads instead of a blanket unsafe `Send + Sync`
   assertion over the event loop.
3. Use a conventional Debian-family x86_64 environment for the first integrated
   evaluation, with **Debian 13 + mkosi as the image experiment** and **unmodified
   labwc + a Rust layer-shell client as the desktop experiment**. These are
   prototype selections, not a qualified release image or compositor commitment.
4. Evaluate **Eclipse Theia as the reusable workbench** for our own developer
   product, with a narrow Rust process/tool interface. Keep project-import logic
   usable through ordinary CLI and VS Code-compatible extension interfaces. Do
   not start another editor, terminal, language service or debugger implementation.
5. Keep the trusted developer environment separate from the enforced application
   execution route. IDE workspace trust, extension activation and debugger access
   need their own integration; they do not implement Burdettie's OS authority
   contract.

These are research recommendations for the next prototype. Source inspection
and the two executed probes below do not qualify a booted image, GUI session,
IDE, language server, debugger or Apple application.

## 1. The actual GUI paths differ from the crate names

The inspected lockfile contains winit **0.30.13**, softbuffer **0.4.8**, and
smithay-client-toolkit **0.19.2**. Repository-wide source searches found:

| Path | Observed implementation | Architectural implication |
| --- | --- | --- |
| [`window-backend-linux`](../../crates/window-backend-linux/src/lib.rs) | Creates winit windows and softbuffer surfaces. `pending_events` is initialized empty; `poll_events()` only drains it. There is no `ApplicationHandler`, event pump or producer for that queue. | Do not describe this path as an input/resize-qualified Linux backend. No app entry point was found constructing `WinitBackend`. |
| [`app-kit-lite::NSApplication`](../../crates/app-kit-lite/src/app.rs) | Its loop invokes the backend's `poll_events()`, then timers/blocks. | It does not supply the missing winit dispatch for the polling backend. |
| [`window-backend-darwin::GuiApp`](../../crates/window-backend-darwin/src/lib.rs) | Uses `EventLoop::run_app` and `ApplicationHandler` with input/redraw handling. Native demo applications import this type. | The crate name does not mean it is an XNU-only path. This is the existing host-backed GUI path to preserve and qualify. |
| [`window-backend-darwin::WinitGuiBridge`](../../crates/window-backend-darwin/src/guest_bridge.rs) | The guest bridge also uses `run_app`, an application handler and an event-loop proxy. | Preserve its integration work while deciding the future compatibility worker boundary. |

The search covered all Rust files, not only the Linux backend. It does **not**
show that every GUI path is broken. It shows that there are two different
implementations with misleading platform names and different completeness.

The [`WindowBackend` trait](../../crates/window-backend/src/lib.rs) requires
`Send + Sync`. `WinitBackend` satisfies it using unsafe implementations with a
comment that callers keep operations on the main thread; that promise is not
represented in its public method types. Winit's own
[event-loop source](https://github.com/rust-windowing/winit/blob/v0.30.13/src/event_loop.rs)
marks its event loop neither `Send` nor `Sync` and provides `EventLoopProxy`
for cross-thread wakeups. This supports a concrete Rust interface improvement:
keep a thread-affine GUI driver and expose a sendable command/proxy handle.
Mutex exclusion alone does not establish thread affinity. No claim of an
observed crash or exploit is made here.

**Next gate:** choose one application event-loop owner, then prove real resize,
text/IME input, pointer input, clipboard and shutdown under a Wayland session.
Keep headless semantic tests, but do not use them as evidence of event delivery
by the compositor.

## 2. Owning a desktop does not require owning its compositor

Winit 0.30.13 exposes ordinary Wayland `xdg_toplevel` windows and application
IDs. Its [Wayland extension surface](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform/wayland.rs)
does not provide the layer-shell role used by desktop panels and backgrounds.
Softbuffer presents pixels; it does not provide desktop window-management
authority. A full-screen ordinary application is not, by itself, a desktop
shell.

Smithay's **client toolkit**, distinct from the Smithay compositor-building
library, already provides a layer-shell client API. Its
[upstream example](https://github.com/Smithay/client-toolkit/blob/d557485b490836962748878c8ed81c49c63ae397/examples/simple_layer.rs)
binds the protocol, anchors a surface, sets keyboard interactivity, receives
configure/input events and presents shared-memory buffers. The example also
checks that the compositor exposes the protocol. This is a useful existing
Rust interface for a Burdettie panel/launcher, not a reason to build another
Wayland compositor. The example inspected at current upstream is **not** proof
that the older 0.19.2 already in our lockfile can be used unchanged.

| Candidate | Fit for the first experiment | Limit or open question |
| --- | --- | --- |
| **labwc** | A stacking compositor that intentionally delegates panels, wallpaper and related desktop pieces to clients; avoids selecting a whole desktop toolkit | Minimal compositor policy, no compositor animations, and limited appetite for new features; use unmodified for the experiment |
| **Weston** | Useful reference and nested/headless integration target | Its kiosk shell deliberately makes top-level windows full-screen; kiosk success does not qualify a multiwindow desktop |
| Existing full desktop | Lowest immediate assembly work for application testing | A Burdettie replacement panel/window policy needs that desktop's extension model; app-window success does not prove shell replacement |
| New Smithay compositor | Maximum implementation control | Introduces another substantial subsystem to maintain; no researched requirement currently justifies this |

Sources: [labwc scope and design](https://github.com/labwc/labwc/blob/4d5f9cb843c8f32d4b0b338d88fed6ba3b2a5a75/README.md),
[labwc configuration and client startup](https://labwc.github.io/labwc-config.5.html),
[Weston overview](https://wayland.pages.freedesktop.org/weston/index.html),
[Weston kiosk shell](https://wayland.pages.freedesktop.org/weston/toc/kiosk-shell.html).

**Prototype selection:** an unmodified labwc session, one separate Rust
layer-shell panel/launcher and existing ordinary application windows. Require
panel anchoring/exclusive space, launch/focus behavior, multiple windows,
clipboard/IME, output changes and clean session shutdown before promoting this
to a product foundation. Preserve another known compositor as a reference so
our client does not silently depend on private extensions.

Protocol access is also authority. A desktop shell may need to enumerate or
activate other applications, while an untrusted application must not receive
equivalent privileges. The inspected
[labwc server](https://github.com/labwc/labwc/blob/4d5f9cb843c8f32d4b0b338d88fed6ba3b2a5a75/src/server.c)
contains security-context-aware protocol filtering. Its presence is source
evidence only: actual restricted sockets and protocol grants belong in the
Linux enforcement qualification, not in an assumption that Wayland alone
isolates every client.

### Upstream contribution fit changes the choice

Labwc's current
[contribution policy](https://github.com/labwc/labwc/blob/4d5f9cb843c8f32d4b0b338d88fed6ba3b2a5a75/CONTRIBUTING.md)
rejects contributions containing AI-generated content, including code, issues
and PR text. It allows research/debugging uses whose generated output is not
included in contributions. Therefore we can evaluate and reuse labwc unchanged,
but this workflow must not promise an AI-generated upstream patch there. A
requirement for compositor changes is a reason to reassess fit rather than
assume a permanent private fork. No upstream communication was sent.

SCTK's inspected
[contribution guide](https://github.com/Smithay/client-toolkit/blob/d557485b490836962748878c8ed81c49c63ae397/CONTRIBUTING.md)
welcomes general-purpose, modular client improvements and focused PRs. That is
a better match for reusable Rust client-side work, subject to the current rules
and maintainer review when a concrete change exists.

## 3. Linux image choice: evaluate the smallest change first

The repository already uses Debian **bookworm** containers for its runtime
builds. The current research host is Debian **13/trixie**. Debian 13 is a new
evaluation baseline here, not a retroactive qualification of old Debian 12
receipts. See the current [runtime Dockerfile](../../docker/Dockerfile.burdettie).

| Option | Reuse advantage | Work still required |
| --- | --- | --- |
| **Debian 13 + mkosi** | Conventional package/filesystem environment; keeps the existing Debian integration family; mkosi provides image assembly and VM-oriented commands | Package/snapshot pinning, complete session services, installer/update/recovery design and boot/hardware qualification |
| **NixOS** | Declarative system modules, image/VM workflows and boot generations/rollback | Package the actual compatibility runtime and desktop; measure closure/build cost; qualify update/state migration and recovery |

NixOS's [manual](https://nixos.org/manual/nixos/stable/) documents its image and
generation workflows. Its [official FAQ](https://nix.dev/guides/faq) explains
the non-FHS layout and available packaging approaches. **Inference:** adding
that packaging variable before establishing a working compatibility runtime
may increase diagnosis cost. This is not evidence that Darling cannot work on
NixOS, and NixOS remains a release contender. PureDarwin's use of Nix build
recipes does not itself require Burdettie's shipping distribution to be NixOS.

Mkosi is an existing package-manager-based image builder, not a complete update
policy. Its [pinned manual](https://github.com/systemd/mkosi/blob/9193786913aa79e3486dc044fe5cf68129aa03ef/mkosi/resources/man/mkosi.1.md)
documents distro snapshots, tools trees and namespace requirements. Do not
write a new image builder or update service merely to fill an untested gap.

### Executed mkosi configuration probe

Upstream source: `systemd/mkosi` at
`9193786913aa79e3486dc044fe5cf68129aa03ef`; reported version `28~devel`.
Reproducer: [linux_desktop_mkosi_probe.py](../../scripts/research/linux_desktop_mkosi_probe.py).

Executed:

```sh
python3 scripts/research/linux_desktop_mkosi_probe.py --mkosi-source /path/to/pinned/mkosi
```

The script invokes upstream `--version`, `dependencies` and `--json summary`
against a temporary configuration. It does not run `build`, `boot`, `vm`,
`burn`, package installation, key enrollment or host policy changes.

| Observation | Result |
| --- | --- |
| CLI and dependency-list query | Exit 0 |
| Configuration parse | Exit 0; no stderr |
| Main image | `Distribution=debian`, `Release=trixie`, `Architecture=x86-64` |
| Output | `Format=disk`, raw image, `Bootable=enabled`, `Bootloader=systemd-boot` |
| Parsed package requests | `labwc`, `linux-image-amd64`, `systemd`, `systemd-boot-efi`, `systemd-sysv` |
| Repository key checks | Enabled |
| Snapshot/tools tree | Unselected; this is not a frozen package closure |
| Host prerequisites | No QEMU, `systemd-repart`, `systemd-ukify` or labwc executable found |
| Image build, boot, GUI, update/recovery, Secure Boot | **Not attempted or qualified** |

This proves that the existing tool accepts a small candidate image definition,
not that requested packages resolve or the image boots. The next image
experiment must pin package snapshots and a tools tree, add the actual
session/login/audio/portal services, then demonstrate boot, document persistence
and recovery. Kernel capabilities must be tested in that image; this host's
kernel is not the future product kernel.

## 4. Reuse the workbench as well as the compilers

Reusing LSP/DAP leaves substantial work if we still build an editor, search,
terminal, project tree, source-control UI and extension host ourselves.
Theia's [composition model](https://theia-ide.org/docs/composing_applications/)
and [extension model](https://theia-ide.org/docs/extensions/) allow a custom
product to reuse those workbench pieces and add its own commands/views. A
VS Code-compatible extension offers a narrower portable integration; Theia
extensions offer deeper product UI customization. The documentation recommends
these two routes over its separate Theia-plugin mechanism.

**Recommended next frontend experiment:** a small Theia product or existing
Theia IDE with one Burdettie extension exposing project import, explicit target
selection and a build/run result through a Rust CLI/service. Keep compiler,
language-server and debugger processes upstream-owned. Prove the necessary
extension APIs before choosing a shipping workbench; Theia documents versioned
VS Code API coverage and some stubbed APIs, so installability is not a feature
pass. [Extension compatibility](https://theia-ide.org/docs/user_install_vscode_extensions/)

Inspected Theia source is `4f8c4094892dabc1530d23ea94eb7b525acfa28a`.
Its [package configuration](https://github.com/eclipse-theia/theia/blob/4f8c4094892dabc1530d23ea94eb7b525acfa28a/package.json)
requires Node >=24 at that revision. Its
[notices](https://github.com/eclipse-theia/theia/blob/4f8c4094892dabc1530d23ea94eb7b525acfa28a/NOTICE.md)
declare EPL-2.0 with the stated secondary-license conditions and separately
list third-party content. Extensions have their own provenance/terms. Its
[contribution guide](https://github.com/eclipse-theia/theia/blob/4f8c4094892dabc1530d23ea94eb7b525acfa28a/CONTRIBUTING.md)
requires an Eclipse Contributor Agreement and coordination before substantial
work. No agreement was accepted or upstream submission made in this research.

Rust remains useful for a stable, explicit tool/target/policy boundary. Small
TypeScript UI extensions to a maintained workbench are compatible with the
strategy; a Rust-only editor requirement would recreate the unnecessary rewrite
goal. Do not deeply fork Theia for branding or force compiler internals through
a new Burdettie protocol when their existing interfaces suffice.

### Developer authority is a separate integration decision

Theia's [workspace-trust documentation](https://theia-ide.org/docs/workspace_trust/)
describes feature gating for tasks, debugging and extensions, and records
current limitations. It does not establish Burdettie's OS confinement. A
workbench extension can spawn processes outside a narrow custom build button
unless the whole process boundary is accounted for.

Use an explicit trusted developer route for the first native toolchain/IDE
experiment. The launch-policy v1 application contract keeps inspection and
runtime-generated executable memory deny-only. Do not silently reinterpret
those controls to accommodate a debugger or an Electron/JavaScript engine.
Future confined developer workspaces need a separately reviewed capability
contract and coverage for extension hosts, terminals, debug adapters and child
processes.

## 5. Executed native compiler probe and remaining gates

Host: Debian GNU/Linux 13, Linux 6.18.57, x86_64, glibc 2.41.
Compiler: `cc (Debian 14.2.0-19) 14.2.0`.
Reproducer: [devtools_native_probe.py](../../scripts/research/devtools_native_probe.py).

```sh
python3 scripts/research/devtools_native_probe.py
```

The probe creates only temporary source/build files. It compiles a C program
with `-std=c11 -Wall -Wextra -Werror -g -O0`, executes it, edits its constant,
rebuilds and executes it again. It then compiles an invalid source with GCC's
structured diagnostics option.

| Check | Observed result |
| --- | --- |
| Initial build/run | Exit 0; `native value=41` |
| Edit/rebuild/run | Exit 0; `native value=42`; binary hash changed |
| Object format | ELF64, machine 62 (x86_64) |
| Invalid source | Compiler exit nonzero |
| Structured error | JSON error for `missing_symbol`, `invalid.c`, line 1, column 25 |
| Rust, clangd, LLDB-DAP, Swift tools | Not found in the inspected host paths |
| LSP completion/navigation, breakpoint/step/variables, IDE GUI | **Not run or qualified** |

This is evidence for the native C compiler/diagnostic interface, not for an
incremental build system or the complete developer milestone. The missing
tools require an explicit, pinned development environment; a passing optional
test that skips absent tools is insufficient.

Next native acceptance should run one Rust and one C project through the reused
tools and selected frontend, including source diagnostics, tests, a breakpoint,
step/variable inspection and process cleanup. Add a portable SwiftPM project
as its own route. Keep `.pbxproj` import and Mach-O execution separate so a
Darwin runtime blocker does not prevent a useful native developer product.

## Remaining evidence that could change these prototype choices

- A named desktop workflow requiring compositor behavior outside labwc's scope.
- Shell protocol availability, privileged-interface isolation, IME/accessibility
  or hardware failures on the actual evaluation session.
- Excessive packaging/upgrade work for the compatibility worker, favoring a
  different conventional distribution or NixOS.
- Update/recovery evidence that favors one image ecosystem decisively.
- Theia extension API, startup/memory, accessibility or native-debugger failures
  that make another existing workbench a better fit.
- A measured requirement for a custom component that upstream configuration,
  an adapter or a maintainable contribution cannot satisfy.

Until those observations exist, keep these changes focused on integration and
shared upstream improvements, and keep release qualification explicitly open.
