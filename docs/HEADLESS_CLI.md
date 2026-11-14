# Headless Darwin CLI feature selection

The `host-dialogs` feature makes the native macOS file-dialog backend optional.
It is enabled by default in [cf-lite](../crates/cf-lite/Cargo.toml),
[macho-runtime](../crates/macho-runtime/Cargo.toml) and
[macho-inspect](../crates/macho-inspect/Cargo.toml), so
ordinary builds retain their existing open/save panels. The CLI's explicit
`gui` feature also enables it.

For a console-only dependency graph, select the CLI package and disable its
default features:

```sh
cargo build --locked -p macho-inspect --no-default-features --target x86_64-apple-darwin
```

This selects the intended dependencies; it is not a complete cross-build recipe.
A suitable Rust standard library, SDK, linker and target libraries are still
required. Their compatibility with a bare Darwin guest is not established by
Cargo resolution or a Linux build.

| CLI selection | Native macOS file dialogs | Window backend |
| --- | --- | --- |
| Default features | Enabled | Disabled |
| `--no-default-features` | Disabled | Disabled |
| `--no-default-features --features gui` | Enabled | Enabled |

Without the native dialog backend, [NSSavePanel and NSOpenPanel](../crates/cf-lite/src/nsopenpanel.rs) `runModal`
return `NSModalResponseCancel`. They do not open a host dialog or report a new
selection. This also preserves the previous Linux cancellation behavior.

Cargo features are [additive](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification).
Building another workspace package alongside the CLI can enable `host-dialogs`
again. Use a package-specific command for the console port. The runtime and CLI
use explicit local dependency paths to disable dependency defaults under the
repository's Rust 1.93 toolchain; the workspace dependencies retain their
existing defaults for other consumers. The redundant runtime development
dependency on `cf-lite` is removed so tests cannot reactivate those defaults.

## Checks and limits

Run the locked dependency-graph regression and the focused panel tests:

```sh
python3 scripts/check_headless_cli_dependencies.py
cargo test --locked -p cf-lite --no-default-features --lib nsopenpanel::tests
cargo check --locked -p macho-inspect --no-default-features
```

The graph script checks `x86_64-apple-darwin` by default and accepts
`--target aarch64-apple-darwin`. It verifies that the headless graph excludes
`rfd`, `objc2-app-kit` and the window backend, while default and explicit GUI
graphs retain the intended dependencies. It needs Cargo, but no Apple SDK or
target standard library. Cargo may acquire locked registry packages; add
`--offline` to require cached inputs.

With Rust 1.93.0 on the Linux development host, all three x86_64 Darwin graph
checks pass, all seven focused panel tests pass, and the headless CLI passes
`cargo check`. These checks do not establish Darwin linking, native macOS dialog
behavior, guest execution or security enforcement. Dynamic framework lookups,
Rust standard-library assumptions and the runtime's other ABI requirements
still need separate qualification.
