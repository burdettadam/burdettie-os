# Docker-First Development Workflow

## Goal

Use Docker as the primary development and verification lane now, without
waiting for a future tool that exposes a live Mac environment.

The immediate rule is simple:

- Linux or Compose-backed Docker runs are the authoritative scored lane.
- Mac-host access is optional and diagnostic.
- Imported Apple artifacts are allowed as read-only validation inputs, but they
  do not define the runtime architecture.

## Current Constraint

The repo does not currently have a first-class tool for talking to a Mac host
from inside the developer workflow. That means:

- artifact extraction from macOS is still manual or externally orchestrated
- the Compose file used in practice is outside this workspace
- development must proceed against the Docker images and scripts that already
  exist in the repo

This is workable. The repo already has three distinct Docker lanes.

## Docker Lanes

### 1. Cross-platform compile and lint lane

Use `docker/Dockerfile.dev` when the goal is to validate the cross-platform
crate subset on Linux.

Use this lane for:

- parser and planner crates
- non-runtime utilities
- formatting, clippy, and basic unit coverage on Linux

Do not use it as proof for runtime, ObjC, Foundation, or app-compat behavior.

### 2. Full Linux runtime build and test lane

Use `docker/Dockerfile.linux-e2e` when the goal is to exercise the runtime
stack on Linux ARM64.

Use this lane for:

- runtime crate buildability on Linux
- interpreted fixture execution
- regressions in runtime, Foundation, or ObjC behavior that do not require
  imported Apple artifacts

This is the best base lane for Docker-first runtime development.

### 3. Imported-artifact compatibility lane

Use `docker/Dockerfile.burdettie` plus the existing `scripts/linux-test.sh`
loop when the goal is to execute imported Apple binaries inside the runtime.

Use this lane for:

- Xcode CLI tool compatibility
- GUI application verification through the Linux display lane
- Docker-backed tool validation against staged imports
- external Compose-based app scoring once the manifest overlay is wired there

For the future external Compose lane, imported artifacts should be consumed
through a declared read-only overlay described by a manifest. The current
in-repo exception is the staged `/opt/xcode-tools` tool path used by the
existing tool-compat loop.

Current state:

- the in-repo scripted path is strongest for Xcode CLI tools staged into
  `docker/xcode-tools` and exposed at `/opt/xcode-tools`
- the same image now has a virtual-display path for GUI validation via
  `burdettie-gui` inside the container or `scripts/docker-gui.sh` from the repo
- the manifest-based imported-app overlay is the target contract for the
  external Compose lane, not yet the in-repo default automation path

## Imported Artifact Contract

The imported-artifact contract is defined by:

- `docker/imported-artifacts.schema.json`
- `docker/imported-artifacts.example.json`

Recommended layout for the external Compose lane:

- manifest mounted at `/opt/imported-artifacts/manifest.json`
- imported app bundles under `/opt/imported-artifacts/apps/`
- imported frameworks under `/opt/imported-artifacts/frameworks/`
- imported SDK overlays under `/opt/imported-artifacts/sdks/`
- imported Xcode tools may also be exposed at `/opt/xcode-tools/` to preserve
  compatibility with existing scripts

The repo-side Compose wiring example now lives at:

- `docker/compose.burdettie-gui.example.yaml`

That example bakes in the verified GUI viewer wiring for the Docker lane:

- host port `5900` mapped to container VNC `5900`
- host port `6080` mapped to container noVNC `6080`
- `BURDETTIE_GUI_MODE=home` by default so the viewer opens on the persistent
  Burdettie home screen instead of an empty desktop
- switch to `BURDETTIE_GUI_MODE=guest` to run smoke fixtures or imported guest
  apps through `macho-inspect --exec --gui`
- `BURDETTIE_GUEST_APP` selects the guest executable in `guest` mode,
  defaulting to `/workspace/fixtures/cocoa_draw/app`
- the first imported Apple target in `guest` mode should use
  `/opt/imported-artifacts/apps/Stickies.app/Contents/MacOS/Stickies`
- an open-source terminal example can use
  `/opt/imported-artifacts/apps/WezTerm.app/Contents/MacOS/wezterm-gui`
- the Launchpad home screen auto-discovers imported GUI apps described by the
  mounted manifest, and local dev can stage the same bundles under `imports/apps`

The overlay must be mounted read-only.

Until the external Compose lane consumes the manifest directly, the in-repo
workaround remains the existing `/opt/xcode-tools` staging path for imported
tool binaries.

## Scoring Rules

An imported-artifact run only counts toward progress if all of the following are
true:

1. The run occurs in Linux Docker or the Compose lane.
2. The imported artifact set is described by a manifest conforming to the schema.
3. Imported artifacts are mounted read-only.
4. Host-assisted Apple framework resolution is not part of the scored path.
5. The run satisfies the target tier rules from
   `docs/APPLE_APPLICATION_TARGETING_PLAN.md`.

Mac-host runs are still useful for diagnosis, dependency inspection, and
behavior comparison, but they do not count as scored compatibility progress.

## Practical Loops

### Fast runtime loop

Use the existing Linux compatibility loop when iterating on runtime changes:

- build or refresh the runtime image once
- use `scripts/linux-test.sh` for repeated Linux ARM64 runs
- drop into `--shell` mode when you need to inspect the container state

### App-compat loop

When validating imported Apple applications:

1. stage the app bundle into the imported overlay
2. record its metadata in the manifest
3. classify it as Tier A, B, or C before scoring it
4. run it only through Linux Docker or Compose if the result will be claimed as
   progress

The same overlay can also carry open-source macOS bundles used as validation
targets, such as `WezTerm.app`. Those should be treated as validation-only
until their rendering and PTY behavior are proven in the Linux display lane.

This loop is a target workflow for the external Compose lane. It is not yet a
fully scripted in-repo path on the same level as the current tool-compat loop.

### GUI app loop

When validating Docker-backed GUI behavior:

1. build or refresh `docker/Dockerfile.burdettie`
2. start the viewer lane with `scripts/docker-gui.sh`
3. run guest apps through `burdettie-gui macho-inspect --exec --gui ...`
4. count the result only if the app behavior is observed in the Linux display lane

### Tool-compat loop

When validating imported Apple tools:

1. extract or transfer the tool set outside the container
2. expose it through the imported overlay or `/opt/xcode-tools/`
3. run the existing Docker compatibility scripts
4. treat host-only runs as oracle output, not as scored delivery

Imported Xcode tool runs are validation work. They do not count as app-compat
progress unless a later scoring policy explicitly says otherwise.

## Immediate Development Priority

Until the future Mac-environment tool exists, prioritize work that improves one
of these Docker-backed outcomes:

1. Linux runtime correctness for public Cocoa and Foundation surfaces.
2. Docker-backed execution of imported Apple applications on public surfaces.
3. Rust-native build-driver work that produces app bundles runnable in the same
   Docker scoring lane.

Do not spend primary effort on workflow branches that require a live Mac host to
be present during normal execution.

## Related Files

- `docs/APPLE_APPLICATION_TARGETING_PLAN.md`
- `docs/XCODE_PUBLIC_COMPATIBILITY_PLAN.md`
- `docker/imported-artifacts.schema.json`
- `docker/imported-artifacts.example.json`
- `docker/Dockerfile.dev`
- `docker/Dockerfile.linux-e2e`
- `docker/Dockerfile.burdettie`
- `scripts/linux-test.sh`