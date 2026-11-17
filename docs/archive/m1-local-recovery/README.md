# Recovered M1 local-overlay guides

These three guides are recovered byte-for-byte from the retained local overlay
at `d3aca41ea49c59d4ba4fe4f046b461f1af1504ed`. They were absent from the public
branch at `4073ffcf59852c1ed92d94df53d917f92d227933`.

They are **historical records, not current qualification reports**. References
to "this overlay", "this continuation", locally missing maps or a failed local
Swift linker describe the earlier environment. They do not supersede later
published source, successful reference CI, or the remaining selected-guest gates.
The source-owned inspectors and producers described here already exist on the
public branch; do not reapply the old overlay patches.

| Recovered guide | Preserved scope |
| --- | --- |
| [Compiler-rt qualification](M1_COMPILER_RT_QUALIFICATION.md) | Selected x86_64 source-derived qualification plan, not build evidence |
| [Compiler-rt inspector](M1_COMPILER_RT_INSPECTOR.md) | Archive/link checks and their historical limitations |
| [Local evidence capture](M1_LOCAL_EVIDENCE_CAPTURE.md) | Capture contract and the retained failed local linker attempt |

For active work, use the [recovery checkpoint](../../milestones/M1_RECOVERY_CHECKPOINT.md),
[current evidence integration](../../milestones/M1_EVIDENCE_INTEGRATION.md),
[observed-link capture](../../milestones/M1_OBSERVED_LINK_CAPTURE.md), and
[compiler-runtime ABI guide](../../milestones/M1_COMPILER_RT_ABI_PROBE.md).
The newer public ABI guide is deliberately retained rather than overwritten by
its older local copy. Raw receipts, generated binaries, original Git bundles,
and machine-specific recovery material remain outside this source archive.
