# Burdettie-OS — Feature Reference

Burdettie-OS is an independently implemented Mach-O loader and experimental
Darwin userspace runtime written in Rust. Its components cover binary parsing,
loading, symbol resolution, Objective-C dispatch, framework shims, ARM64
interpretation, platform integration, and a software-rendered GUI.

Compatibility depends on the binary, host, execution path, and implemented APIs.
Individual methods may be partial or stubbed. The presence of a class, symbol,
or shim is not evidence of complete framework behavior.

## Where to look

| Document | Contents |
| --- | --- |
| [CAPABILITIES.md](CAPABILITIES.md) | Subsystem descriptions and implementation details. |
| [CRATE_MAP.md](CRATE_MAP.md) | Workspace organization by layer. Check [Cargo.toml](../Cargo.toml) for the current member list. |
| [METRICS.md](METRICS.md) | Historical compatibility and coverage measurements. |
| [SECURITY.md](SECURITY.md) | Trust-policy models, platform assumptions, and known limitations. |
| [APPLE_STACK_COMPARISON.md](APPLE_STACK_COMPARISON.md) | Layer-by-layer comparison and coverage notes. |
| [roadmap.md](roadmap.md) | Planned work and target-platform direction. |
| [SEMANTIC_MEMORY_BUILD_BRIEF.md](SEMANTIC_MEMORY_BUILD_BRIEF.md) | Design of the experimental semantic-memory subsystem. |

## Interpreting results

The detailed references were written at different development stages. Counts,
pass rates, class inventories, and performance results in those documents are
historical snapshots unless accompanied by a matching revision and reproducible
command. Use the current source and the results of your own checkout when
assessing a capability.

For a first run, follow the [README walkthrough](../README.md#quick-start).
Record the commit, host architecture, toolchain, fixture sources, and command
when publishing new results. Locally imported Apple software is a separate
validation input and is not part of the public source or default test contract.
