# burdetty-os Demo Guide

Everything you need to build, package, and run the full demo — from CLI
scorecard to GUI applications.

## Prerequisites

- **macOS 13+** (Ventura or later) on Apple Silicon
- **Rust stable** (1.82+) via `rustup`
- **Xcode Command Line Tools** (`xcode-select --install`)

## Quick Start (Development)

```bash
# Build everything
cargo build --workspace

# Run the interactive shell
cargo run -p runtime-shell

# Inside the shell:
darwin> help          # list commands
darwin> list          # list demo fixtures
darwin> run hello     # run a CLI demo
darwin> score         # full compatibility scorecard
darwin> gui           # list GUI apps
darwin> gui hello-darwin       # launch GUI app
darwin> gui control-gallery    # launch widget gallery
darwin> gui foundation-explorer  # browse Foundation classes
darwin> exit
```

## Building the Demo Image

### Option A: DMG Disk Image

```bash
# Full pipeline: format → clippy → build → test → fixtures → bundle
./packaging/build.sh --release

# Bundle into dist/
./packaging/bundle-runtime.sh --release

# Create .dmg
./packaging/make-vm-image.sh
```

### Option B: Docker Image (CLI only)

```bash
# Build on macOS first
./packaging/build.sh --release
./packaging/bundle-runtime.sh --release

# Build Docker image from the bundle
docker build -f docker/Dockerfile.demo -t burdetty-os-demo dist/

# Run interactively
docker run -it --rm burdetty-os-demo
```

### Option C: VM with Virtualization.framework

```bash
# One command — installs macOS guest if needed, then boots
./vm-image/launch.sh

# Or step by step:
./vm-image/launch.sh install   # first-time setup (~13 GB download)
./vm-image/launch.sh           # boot existing VM
./vm-image/launch.sh status    # check VM state
./vm-image/launch.sh clean     # remove VM bundle
```

## Demo Layout

When packaged, the runtime is laid out as:

```
darwin-runtime/
├── bin/
│   ├── runtime-shell           # interactive CLI
│   ├── macho-inspect           # Mach-O inspection tool
│   ├── hello-darwin            # GUI: lifecycle demo
│   ├── control-gallery         # GUI: widget exercise
│   └── foundation-explorer     # GUI: class browser
├── fixtures/                   # 62 test binaries
│   ├── simple_cli/app
│   ├── return42/app
│   └── ...
├── config/
│   ├── banner.txt              # boot banner
│   ├── demos.json              # demo descriptors
│   ├── runtime-env.sh          # environment setup
│   └── auto-start.sh           # VM auto-launch script
├── docs/
│   └── ...
└── logs/
    └── trace.log               # runtime trace output
```

## Boot Sequence (VM)

1. VM boots → auto-login → desktop appears
2. `runtime-shell` launches in a terminal window
3. Boot banner displays runtime stats:
   ```
   ╔═══════════════════════════════════════════════════╗
   ║  burdetty-os Runtime Environment v0.1       ║
   ║  Compatibility: 62/62 (100%)                      ║
   ║  Foundation classes: 40 | Shim functions: 59      ║
   ║  Type 'help' for commands, 'gui' for GUI demos    ║
   ╚═══════════════════════════════════════════════════╝
   ```
4. User can run CLI demos, view scorecard, or launch GUI apps

## Shell Commands

| Command | Description |
|---|---|
| `list` | List all demos grouped by category |
| `run <name>` | Run a demo fixture by name |
| `score` | Full compatibility scorecard table |
| `gui [app]` | List or launch GUI demo apps |
| `trace on\|off` | Toggle runtime event tracing |
| `policy perm\|secure` | Toggle security policy mode |
| `info` | Show session settings |
| `banner` | Reprint boot banner |
| `help` | Show command help |
| `exit` | Quit |

## GUI Applications

### hello-darwin
Minimal lifecycle demo. Click "Run Scorecard" to execute all fixtures
through the runtime and see pass/total results. A "Quit" button exits
cleanly.

### control-gallery
Widget exercise with a text field, echo button, toggle background, and
clear button. Demonstrates the full text input → button action → label
update pipeline.

### foundation-explorer
Introspection browser for the 40 registered Foundation classes. Scrollable
class list with 12 visible rows, "Show" buttons to display method names,
and Prev/Next pagination.

## Compatibility Score

The runtime currently passes **62/62** fixture binaries through all stages:

| Stage | Description |
|---|---|
| Parse | Mach-O header + load commands parsed correctly |
| Load | All dependencies located and mapped |
| Fixups | Rebase/bind/lazy-bind fixups applied |
| Enter | Main entry point reached |
| Complete | Execution completed without crash |
| Exit | Exit code matches expected value |

## Troubleshooting

**"Fixture binary not found"** — Run `./scripts/build_fixtures.sh` to
compile the test binaries.

**GUI app doesn't launch** — Ensure you've built with `cargo build
--workspace`. GUI apps require macOS with a display server.

**VM won't boot** — Requires macOS 13+ on Apple Silicon. Check
`./vm-image/launch.sh status` for diagnostics.
