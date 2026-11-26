# Upstream reuse and contribution strategy

Current strategy: [ADR 0003](adr/0003-linux-foundation-and-upstream-reuse.md)
and [ADR 0004](adr/0004-linux-integration-boundaries.md), accepted 2026-11-25.
Linux is our shipping foundation. Rust is preferred for
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
| Image assembly and updates | Debian 13/mkosi prototype; NixOS remains a release contender | Selected distribution/image-builder upstream |
| Darwin execution on Linux | Complete upstream core/system Darling control first; existing Burdettie bridge as comparison | Actual runtime owner; Rust product adapter remains Burdettie |
| Portable Darwin library logic and SDK tooling | Selected PureDarwin components | PureDarwin for shared behavior/build fixes; original library when appropriate |
| Dispatch and portable Swift libraries | Existing Linux ports from Swift upstream | The corresponding Swift component |
| Compilers, language services and debugging | Cargo/rustc, rust-analyzer, LLVM/clangd/LLDB, SwiftPM/SourceKit-LSP | The tool that owns the defect |
| Developer workbench | Evaluate Theia and existing extension interfaces | Theia or the actual extension/tool owner; Burdettie project/target UX stays here |
| Windows, input and rendering | Consolidate host-backed winit application path; evaluate separate SCTK shell client on unmodified labwc | winit, SCTK, softbuffer or the responsible desktop/toolkit; observe each contribution policy |
| Our UI, project workflows and owner policy | Burdettie code using those components | Burdettie unless a generally useful API can be extracted |

Do not route a Linux-only shim into PureDarwin simply because the code originated
there. Do not route an original-library bug through a permanent private fork
when the library's own maintainers can accept the fix.

## First contribution candidates

These are researched candidates, not submitted or accepted changes. The
[PureDarwin probe record](research/2026-11-25-puredarwin-reuse.md) now contains
executed evidence: a rendered SDK wrapper mismatch and a Linux libDER build
with two failed semantic expectations. Those results sharpen the contribution
scope; they do not qualify either component for production trust decisions.

### 1. SDK discovery consistency

At PureDarwin `0c1990c803c13b68cad4df3f6dc34ba4306fc789`, the source SDK recipe
declares version 26.5, while `toolchain.nix` defaults to a MacOSX11.3.sdk path
and its `xcrun --show-sdk-version` wrapper prints 11.3. Determine which wrapper
paths are used by external projects and whether callers override the defaults.
Then test a focused host developer package with explicit SDK discovery.

The bounded shell probe rendered the upstream discovery branches without Nix
or an SDK. Setting `DARWIN_SDK_ROOT=/fixture/MacOSX26.5.sdk` changed the reported
path, while the reported version remained 11.3. This reproduces the reporting
inconsistency at the wrapper boundary; it does not prove a full SDK build or
compiler invocation. Preserve caller tracing, actual SDK metadata and bootstrap
dependency checks in the proposed fix.

Pass: reported SDK path/version and selected tools agree with the actual input;
a source-owned fixture produces the intended Mach-O metadata. Keep bootstrap
dependencies acyclic. Compile/link evidence is separate from Linux execution.

Sources: [SDK assembly](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/apple-sdk-pinned.nix),
[toolchain wrapper](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/nix/pkgs/toolchain/toolchain.nix).

### 2. Resolve libDER semantics before trust-parser adoption

The existing C sources compiled and executed on Linux, including sanitizer
runs. The 30-case probe passed 28 semantic expectations and failed two: DER
BIT STRING unused-bit canonicality and constructed-SEQUENCE tag representation.
Hold adoption in signature/entitlement decisions. The first contribution should
be a minimal regression and API/encoding-contract clarification, followed by a
focused upstream fix and native packaging. A portable build and a clean
sanitizer observation on this corpus are insufficient semantic qualification.

Retain a narrow Rust wrapper for borrowed-buffer lifetimes, ownership and error
handling only after the chosen upstream contract is suitable. Whole-input
validation is also a caller obligation for prefix-decoding APIs. Do not add a
new DER implementation. A parser does not supply CMS/chain verification or a
trust-policy decision. [Probe and results](research/2026-11-25-puredarwin-reuse.md).

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

### Other upstreams influence architecture choices

- **Darling:** test the existing rootless/non-root and unprivileged installation
  paths before proposing a new launcher. Establish the complete core/system
  control route before a Rust supervisor adapter or optional framework build.
  Any activation/barrier or lifecycle fix should improve the upstream route,
  not create a second private runtime. Host-only launcher tests are not an
  application pass. Selected prerequisites succeeded on the existing AWS host,
  but the disk reserve prevented a build or dependency installation; no runtime
  adoption follows. [Runtime research](research/2026-11-25-darling-runtime.md),
  [host preflight](research/2026-11-25-aws-host.md).
- **SCTK:** its inspected guide asks for generally useful, modular client
  improvements and focused PRs. It is a relevant destination for shared Rust
  Wayland-client work. [Contribution guide](https://github.com/Smithay/client-toolkit/blob/d557485b490836962748878c8ed81c49c63ae397/CONTRIBUTING.md).
- **labwc:** the current policy rejects AI-generated contribution content,
  including code, issues and PR descriptions. Research/debugging use whose
  generated output is not contributed is distinguished in the policy. Reuse
  unmodified for the prototype; do not promise a generated upstream patch.
  A requirement beyond its conservative compositor scope can change the
  selection. [Policy](https://github.com/labwc/labwc/blob/4d5f9cb843c8f32d4b0b338d88fed6ba3b2a5a75/CONTRIBUTING.md).
- **Theia:** prefer composition/extensions over a deep workbench fork. Its
  contribution guide requires an Eclipse Contributor Agreement and discussion
  before substantial work. No agreement is accepted by this research.
  [Guide](https://github.com/eclipse-theia/theia/blob/4f8c4094892dabc1530d23ea94eb7b525acfa28a/CONTRIBUTING.md).

An upstream policy or scope mismatch does not prevent reuse of an unchanged
licensed component, but it changes the expected maintenance/contribution path.
Record that tradeoff before selecting a dependency that will need modifications.

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
