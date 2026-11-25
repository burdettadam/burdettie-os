# Develop and run Burdettie on Linux

Current strategy: [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md).
Linux is both the primary development environment and the shipping OS
foundation. Mac hardware is not required for the native Linux product,
source-owned fixtures, Rust adapters or upstream packaging/test contributions.

## Start with the existing Linux workflows

Use the [build guide](../BUILD.md) and existing Docker/Linux scripts with the
pinned Rust toolchain. Record the actual host distribution/kernel, CPU,
compiler/tool versions, source revision and fixture inputs. Keep native ELF,
native Mach-O and interpreted Mach-O results separate. A Linux C compiler's
normal ELF output is not a Mach-O runtime fixture.

The current runtime has Linux platform services and partial Darwin behavior.
First reproduce its supported baseline, then compare a complete upstream
compatibility worker under the same declared fixture expectations. The
[research register](../RESEARCH_QUESTIONS.md) defines the comparison and the
narrow adapter experiment. Darling is a candidate, not an adopted dependency.

For our developer experience, use existing compilers, build systems, language
servers and debuggers. Native Linux ELF build/run/test/index/debug is the first
workflow. Mach-O production and execution are a separate target profile with
explicit SDK, architecture and compatibility backend.

## Bootable product and owner control

A Docker demo is useful development evidence but does not provide a bootable
Linux image. M1 selects an existing Linux image/session foundation and records
its boot, runtime, persistence and recovery results. The
[active target plan](linux-product-targets.json) keeps unresolved image and
backend fields explicit.

Use the [shared launch contract](../contracts/launch-policy-v1.md) and
[Linux control allocation](../contracts/LINUX_BACKEND_ALLOCATION.md). Policy
parsing, code-signature presence and sandbox-success stubs are not enforcement.
Measure controls on the real kernel and execution path, including descendants
and compatibility servers. Unavailable required controls refuse enforced launch.

## Optional reference environments

PureDarwin source, host tools and portable library components can be reused
without making XNU the product kernel. A PureDarwin VM is appropriate when a
specific upstream contribution needs target testing or a behavior comparison.
Its retained recipes, [source refresh](M1_PUREDARWIN_SOURCE_REFRESH_20261123.md),
[AWS checkpoint](M1_AWS_DEVELOPMENT.md) and `m1-targets.json` describe historical
reference work. Do not resume their image-build sequence merely because it was
previously the next task. Recheck source, capacity and task relevance first.

Native macOS can remain an optional observation environment for exact Apple
application behavior. It does not gate native Linux progress. Application
compatibility claims still require legally available exact inputs and observed
workflows; missing app inputs are recorded as missing rather than simulated.

## Resource discipline

Use existing resources, reuse verified artifacts only when their identities
match, and preflight disk/memory/time before large upstream builds. Preserve
the retained host's existing limits and shared services. No paid infrastructure
or resource expansion is introduced by this strategy update.

The former Linux-build-host/XNU-product guide is preserved in
[Git history](https://github.com/burdettadam/burdettie-os/blob/a55b71930e5aee279214daa934e084dba8863cfa/docs/milestones/M1_WITHOUT_MAC.md).
Current acceptance is [M1 Linux foundation](M1_PLATFORM_SECURITY.md).
