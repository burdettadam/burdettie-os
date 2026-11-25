# ADR 0001: Darwin boot substrate — superseded

Status: **superseded and withdrawn as product direction**, 2026-11-25.

[ADR 0003: Linux foundation, Rust interfaces, upstream reuse](0003-linux-foundation-and-upstream-reuse.md)
is the current decision. Linux is the shipping kernel and runtime foundation.
PureDarwin/XNU is no longer the selected product substrate. Its VM may be used
only as an optional reference or to qualify a specific upstream contribution.

The former source-image build, guest port and XNU boot work are not prerequisites
for Linux product milestones. Continue using the shared launch/policy contract,
but qualify its implementation on the selected Linux execution route.

The [previous decision and exact experimental recipes](https://github.com/burdettadam/burdettie-os/blob/a55b71930e5aee279214daa934e084dba8863cfa/docs/adr/0001-darwin-boot-substrate.md)
remain available in Git history. Existing evidence files are unchanged and apply
only to their recorded source/image/environment. This redirect does not convert
old PureDarwin results into Linux qualification or erase their historical scope.

Current work: [roadmap](../roadmap.md), [M1 Linux platform plan](../milestones/M1_PLATFORM_SECURITY.md),
[research register](../RESEARCH_QUESTIONS.md).
