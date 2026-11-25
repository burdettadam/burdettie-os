# ADR 0002: Rust platform transition — superseded

Status: **superseded; replacement requirement withdrawn**, 2026-11-25.

[ADR 0003: Linux foundation, Rust interfaces, upstream reuse](0003-linux-foundation-and-upstream-reuse.md)
is the current decision. Burdettie builds and ships on Linux. Rust is preferred
for owned interfaces, bridges, supervision and product behavior where it adds
value. An owned Rust kernel and wholesale replacement of core userland are no
longer goals, milestones or release prerequisites. The former R1-R6 transition
track is withdrawn.

Prefer existing open-source implementations, small adapters and upstream fixes.
Preserve useful existing code until a qualified alternative replaces it. The
product still owns its UI, developer experience and trust policy, and still
requires evidence for named application workflows and security controls.

The [previous strategy](https://github.com/burdettadam/burdettie-os/blob/a55b71930e5aee279214daa934e084dba8863cfa/docs/adr/0002-rust-platform-transition.md)
is retained in Git history, not as an active instruction. No new kernel
prototype or platform-replacement work is scheduled by the current strategy.

Current work: [roadmap](../roadmap.md), [upstream strategy](../UPSTREAM_STRATEGY.md),
[research register](../RESEARCH_QUESTIONS.md).
