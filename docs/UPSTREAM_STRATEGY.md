# Upstream reuse and contribution strategy

Current strategy: [ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md),
accepted 2026-11-25. Linux is our shipping foundation. Rust is preferred for
owned interfaces and bridges. The goal is to improve shared open-source
foundations while retaining Burdettie's UI, developer experience and owner policy.

## Choose reuse before a replacement

For a material new subsystem or replacement, record the needed behavior and
the closest maintained upstream implementation. Prefer, in order:

1. Use the upstream package/API unchanged.
2. Add a small Rust adapter or product configuration.
3. Fix the shared implementation and contribute that fix with a regression.
4. Carry a narrow, attributed patch while review is pending.
5. Implement a missing component only after documenting why the options above
   do not meet the named requirement.

This is an engineering decision record, not a new permission gate. Do not
discard useful code before the replacement meets its behavior contract. Do not
make a component Rust-only merely to increase the proportion of Rust in the OS.

## Responsibility map

| Need | Reuse/evaluate | Where general improvements belong |
| --- | --- | --- |
| Kernel, devices and platform services | Linux, existing distro packages | Linux or the responsible driver/library/distribution |
| Image assembly and updates | Existing Linux image/build tools; distro choice is RQ-04 | Selected distribution/image-builder upstream |
| Darwin execution on Linux | Existing Burdettie bridge compared with a complete Darling worker | Actual runtime owner; Rust product adapter remains Burdettie |
| Portable Darwin library logic and SDK tooling | Selected PureDarwin components | PureDarwin for shared behavior/build fixes; original library when appropriate |
| Dispatch and portable Swift libraries | Existing Linux ports from Swift upstream | The corresponding Swift component |
| Compilers, language services and debugging | Cargo/rustc, rust-analyzer, LLVM/clangd/LLDB, SwiftPM/SourceKit-LSP | The tool that owns the defect |
| Windows, input and rendering | Current winit/softbuffer boundary; existing Wayland desktop | winit, softbuffer, desktop/toolkit or graphics upstream |
| Our UI, project workflows and owner policy | Burdettie code using those components | Burdettie unless a generally useful API can be extracted |

Do not route a Linux-only shim into PureDarwin simply because the code originated
there. Do not route an original-library bug through a permanent private fork
when the library's own maintainers can accept the fix.

## First contribution candidates

These are researched candidates, not submitted or accepted changes.

### 1. SDK discovery consistency

At PureDarwin `0c1990c803c13b68cad4df3f6dc34ba4306fc789`, the source SDK recipe
declares version 26.5, while `toolchain.nix` defaults to a MacOSX11.3.sdk path
and its `xcrun --show-sdk-version` wrapper prints 11.3. Determine which wrapper
paths are used by external projects and whether callers override the defaults.
Then test a focused host developer package with explicit SDK discovery.

Pass: reported SDK path/version and selected tools agree with the actual input;
a source-owned fixture produces the intended Mach-O metadata. Keep bootstrap
dependencies acyclic. Compile/link evidence is separate from Linux execution.

Sources: [SDK assembly](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/apple-sdk-pinned.nix),
[toolchain wrapper](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/toolchain.nix).

### 2. Native libDER package and shared regression inputs

Package the existing C sources for Linux hosts and exercise valid encodings,
truncation, malformed lengths and overflow cases. Retain a narrow Rust wrapper
for ownership and error handling in Burdettie unless upstream wants to maintain
it. Do not add a new DER implementation. A parser does not supply CMS/chain
verification or a trust-policy decision.

Source: [libDER build definition](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libDER/CMakeLists.txt).

### 3. SBPL host tooling and truthful capability diagnostics

Reuse the portable compiler/parser for diagnostics and shared tests only where
useful. Its serialized output targets PureDarwin's kernel sandbox; it is not a
Linux enforcement format. Some recognized operation families currently produce
no enforcing operation bits. A successful compile must not appear as supported
confinement. Start with packaging/tests; discuss changes to parser semantics or
strict-mode behavior before submitting them upstream.

Sources: [compiler](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libsandbox/sbpl.c),
[Darwin application layer](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/src/Libraries/libsandbox/sandbox.c).

### 4. One shared framework behavior fixture

Extract a real failing workflow into a source-owned C/Objective-C fixture with
precise output, error and lifetime expectations. Run it separately on the
selected Linux compatibility route and PureDarwin when the contribution affects
that target. PureDarwin already has a
[framework test package](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/apple/frameworks-ff-tests.nix).
Passing a startup banner or symbol lookup does not establish semantic parity.

## Contribution process

Read the actual component's current contribution rules before work intended for
submission. Record the upstream revision, relevant history/blame, minimal
reproducer, behavior before/after, target tests and license provenance. Keep
project branding and policy choices out of generic patches.

PureDarwin's inspected [policy](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/CONTRIBUTING.md)
directs newcomers toward Nix packages and asks them to coordinate before changing
existing kernel/system-library behavior. Additive libSystem integration is
exempt when it does not change the underlying implementation. Follow its build
and target-test requirements; a Linux host test alone does not qualify a change
to Darwin behavior. Maintainers decide acceptance.

AI-assisted code is allowed there with contributor responsibility, line-by-line
review and provenance. Ordinary issues, PR descriptions and review replies must
be human-authored; the policy describes a separate disclosed-automation exception.
Prepare technical evidence for the contributor and respect each project's
communication policy. Strategy research does not itself send messages or
authorize posting on an upstream maintainer's behalf.

## Dependency and patch records

For each adopted component, retain upstream URL, exact revision/version,
per-file licenses/notices, build recipe, enabled features, target/ABI,
modifications, maintainer route and qualification evidence. A process boundary
does not automatically settle license obligations; review the actual combined
distribution and preserve the relevant source/notice obligations.

PureDarwin's [NOTICE](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/NOTICE)
makes component/file notices authoritative. Do not label its entire tree as a
single license. The same applies to Darling and its submodules.

Every carried patch should name its upstream issue/PR when one exists, the reason
it is carried, the tested revision and a removal condition. Rebase or retire it
when upstream resolves the problem; record rejected/out-of-scope patches honestly.
Evaluate adoption by supported workflows and maintenance cost, not by whether
every patch has already merged. Contributions can also be tests, packaging,
documentation and reproducible bug reports.

## Progress measures

Track working application/developer workflows, reproducible regressions,
upstream submissions and outcomes, downstream patches and their age, duplicated
subsystems retired after qualification, and the cost of upstream updates.
Do not use total rewritten lines or blanket framework-coverage percentages as
substitutes for those outcomes.
