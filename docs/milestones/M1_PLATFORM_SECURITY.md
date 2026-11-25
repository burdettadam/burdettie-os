# M1 — Linux foundation and security contracts

Strategy revision: **linux-2026-11-25**. Status: **in progress; rebaselined**.
[ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) selects Linux as
Burdettie's shipping kernel and runtime foundation. Rust supplies owned
interfaces, bridges and supervision; upstream implementations supply shared
foundations. PureDarwin is a source/test partner and optional reference VM.

This replaces the former PureDarwin/XNU boot target and Rust-kernel destination.
It does not mark the Linux product, app compatibility or enforced mode complete.
Historical M1 receipts retain their original backend, revision and scope.

## Product and target decisions

| Field | Current decision |
| --- | --- |
| Product | Bootable Linux-based Burdettie OS, own UI/developer workflows and owner trust; native Linux apps plus named macOS compatibility workflows |
| First evaluation lane | Linux x86_64 with matching x86_64 Mach-O fixtures; existing ARM64/interpreter regressions remain separate evidence |
| Kernel | Linux; no owned kernel replacement milestone |
| Runtime backend | Current Burdettie baseline compared with a complete upstream Darling worker; adoption unselected |
| Release image/session | Existing Linux distribution/image and desktop components; exact versions/choice require the bounded M1 experiment |
| New active target plan | [linux-product-targets.json](linux-product-targets.json); planning data, not an executable capability manifest |
| Reference work | PureDarwin/macOS comparisons only for a specified component or workflow; not shipping prerequisites |
| Application candidate | Exact Stickies version/slice still requires inventory; source-owned CLI/Foundation and document fixtures provide independent reproducible work |
| Security modes | Permissive, audit and enforced under the existing launch-policy contract |
| Resources | Existing development resources and their limits; no paid expansion authorized by this strategy |

The former [m1-targets.json](m1-targets.json), PureDarwin backend JSON and source
receipts remain historical inputs to the retained reference tools. They are not
the current product selection and must not be consumed as Linux qualification.

## Reuse the existing runtime and contract work

Current source baseline for this strategy review is
`a55b71930e5aee279214daa934e084dba8863cfa`. It contains the Linux platform backend,
loader/shims, native and interpreted routes, framework work, public fixture
sources and launch/admission work. Preserve these assets while comparing
upstream alternatives; no new compatibility backend is claimed here.

The [launch-policy-v1 contract](../contracts/launch-policy-v1.md) still defines
mode semantics, owner authority, grants, immutable inputs, reporting and worker
lifecycle. [Linux backend allocation](../contracts/LINUX_BACKEND_ALLOCATION.md)
replaces the former Darwin allocation for product planning. All candidate
mechanisms need evidence on their actual Linux execution route. A process,
Rust wrapper, successful sandbox shim or valid JSON report does not establish
confinement.

Enforced launch refuses unavailable required controls before initialization.
Audit reports would-deny outcomes without applying optional policy restrictions.
The supervisor owns authenticated policy and worker lifecycle; app workers and
compatibility services cannot manufacture grants or alter supervisor state.

## M1 work and exit gates

| Work item | Owner | Required evidence |
| --- | --- | --- |
| M1.1 Linux baseline | Platform/runtime | Pin actual distro/kernel/toolchain/source; run existing applicable fixtures and classify missing inputs, skips and failures honestly |
| M1.2 Upstream compatibility comparison | Compatibility | Upstream control run plus Burdettie-supervised worker on a small fixed same-ISA corpus; compare behavior, maintenance and resource cost |
| M1.3 Image/session feasibility | Platform/UI | Select an existing Linux base; boot the session, execute a declared worker, preserve exact data across clean reboot and demonstrate recovery |
| M1.4 Launch/policy integration boundary | Runtime/security | Preserve the contract; define worker/server authority, input binding, native/interpreted scope and required refusal behavior |
| M1.5 Linux capability allocation | Platform/security | Candidate mechanism and negative test for each control family; no supported flag without observed enforcement |
| M1.6 Target freeze | Product/compatibility | Record chosen backend/image, fixed fixture corpus, application candidate/version status and remaining blockers |

See [RQ-01 through RQ-09](../RESEARCH_QUESTIONS.md) for bounded experiments,
source findings and stop conditions. A direct darlingserver adapter follows a
working complete-runtime baseline only if it reduces maintenance. Direct Darwin
traps and cross-ISA execution remain separately scoped.

## Application and document workflows

Stickies remains an Apple-authored GUI candidate, not a declared supported app.
Before testing, inventory the exact bundle/version, source OS, selected slice,
resource/executable hashes, dependency closure and services. The workflow is to
create/edit a note with representative Unicode text, use selection/clipboard,
quit, cleanly reboot and reopen the exact content. Repeat on three documented
cold boots. If dependencies block it, record the blocker and make an explicit
application/version decision; do not silently substitute a native Rust demo.

`BurdettieDocument` 0.1.0 remains a separate public document-fixture specification:
source-owned C/Objective-C, initially a programmatic interface, open a known
UTF-8 file, edit/save exact bytes, quit/reboot/reopen, and report missing or
unwritable files. Build it through the selected toolchain and run it on the
Linux compatibility route. It is not declared implemented or passing here.
Later test a denied file access, explanation, owner grant and successful
relaunch under the same launch contract.

When an application bundle is available locally, the existing collector can
inspect it without launching or uploading it:

```sh
python3 scripts/collect_m1_inventory.py --no-hardware --app /path/to/Stickies.app > burdettie-m1-app-inventory.json
```

The collector does not prove authenticity, transitive dependencies or
compatibility. Keep application bundles and private observations outside public
source. Missing Apple inputs do not block the Linux baseline, native developer
workflow or shared library/package contributions.

## Integrated alpha acceptance

- The declared Linux image boots independently on its reference VM, starts the
  Burdettie session and preserves exact user/app data across clean restart.
- Native Linux developer workflows use existing tools; Darwin fixtures and the
  named application pass on their declared compatibility route.
- Supported CLI/GUI/native/interpreted paths use the authenticated launch
  contract or clearly refuse unsupported combinations.
- Each advertised control is demonstrated through actual operations,
  descendants, inherited handles and relevant brokers/servers. Missing controls
  never silently weaken an enforced launch.
- Crashes and denials leave the session usable; policy and updates are durable
  and recoverable without losing user data.
- Release/hardware claims identify the exact kernel, configuration, drivers,
  CPU route, component revisions and notices. A Docker demonstration does not
  qualify boot or physical hardware.

M1 closes after its bounded decisions and feasibility results are recorded;
M2-M7 deliver the integrated alpha under the [roadmap](../roadmap.md). Older
PureDarwin build/boot results cannot close these Linux gates. The previous M1
plan remains in [Git history](https://github.com/burdettadam/burdettie-os/blob/a55b71930e5aee279214daa934e084dba8863cfa/docs/milestones/M1_PLATFORM_SECURITY.md).
