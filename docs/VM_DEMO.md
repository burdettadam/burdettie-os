# VM Demo Guide

## Overview

The VM demo boots a lightweight macOS guest using Apple's Virtualization framework.
The guest shares a directory with the host containing the runtime binaries and all
test fixtures, allowing you to run and inspect Mach-O binaries inside an isolated
environment.

## Requirements

- **macOS 13+** (Ventura or later)
- **Apple Silicon** (M1/M2/M3/M4)
- **~20GB free disk** (13GB IPSW download + 64GB sparse disk image)
- **Xcode** or Command Line Tools (for Swift compilation)

## Quick Start

```bash
# 1. Build the project and bundle runtime
./packaging/build.sh --release
./packaging/prepare-vm-rootfs.sh --release

# 2. Launch the VM (auto-installs on first run)
./vm-image/launch.sh
```

On first run, this will:
1. Build the Swift VM launcher
2. Download the macOS restore image (~13GB IPSW)
3. Create a 64GB sparse disk image
4. Install macOS into the VM
5. Boot the VM with the shared directory mounted

Subsequent runs skip steps 2-4 and boot directly.

## Commands

```bash
./vm-image/launch.sh              # Auto-detect: install or boot
./vm-image/launch.sh install      # Force fresh macOS installation
./vm-image/launch.sh boot         # Boot existing VM
./vm-image/launch.sh status       # Show VM bundle info
./vm-image/launch.sh clean        # Remove VM bundle (disk image, etc.)
./vm-image/launch.sh help         # Show help
```

## Inside the Guest

Once macOS boots in the VM, the shared directory is available as a VirtioFS volume.
Mount it and source the environment:

```bash
# The shared directory should auto-mount at /Volumes/darwin-runtime/
source /Volumes/darwin-runtime/config/runtime-env.sh

# Inspect a fixture
macho-inspect fixtures/simple_cli/app

# List all fixtures
ls /Volumes/darwin-runtime/fixtures/
```

### Auto-Launch (Optional)

To have Terminal open automatically on login with the runtime environment:

```bash
cp /Volumes/darwin-runtime/config/auto-launch.plist \
   ~/Library/LaunchAgents/com.burdettie-os.runtime.plist
launchctl load ~/Library/LaunchAgents/com.burdettie-os.runtime.plist
```

## Configuration

All settings are controlled via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `VM_IPSW` | `~/.burdettie-os/RestoreImage.ipsw` | Path to macOS restore image |
| `VM_SHARED_DIR` | `./dist` | Directory shared with guest |
| `VM_CPUS` | `2` | Number of virtual CPUs |
| `VM_MEMORY_GB` | `4` | Memory allocation in GB |
| `VM_BUNDLE` | `~/.burdettie-os/vm-bundle` | VM persistent state directory |
| `BURDETTIE_OS_ROOT` | Current directory | Project root |

Example with custom settings:

```bash
VM_CPUS=4 VM_MEMORY_GB=8 ./vm-image/launch.sh
```

## Shared Directory Layout

The `dist/` directory (created by `prepare-vm-rootfs.sh`) is shared read-only with the guest:

```
dist/
├── bin/
│   └── macho-inspect          # CLI inspection tool
├── fixtures/
│   ├── simple_cli/app         # Basic executable
│   ├── return42/app           # Exit code test
│   ├── dylib_chain/app        # Dynamic library chain
│   ├── objc_classes/app       # ObjC class usage
│   └── ... (58+ fixtures)
├── config/
│   ├── runtime-env.sh         # Environment setup script
│   └── auto-launch.plist      # LaunchAgent for auto-start
└── docs/
    ├── README.md
    ├── architecture.md
    └── ...
```

## Architecture

```
┌─────────────────────────────┐
│  Host (macOS ARM64)         │
│                             │
│  vm-image/launch.sh         │
│    └─ darwin-vm (Swift)     │
│        └─ VZVirtualMachine  │
│            │                │
│            │ VirtioFS       │
│            │ (dist/ → guest)│
│            ▼                │
│  ┌──────────────────────┐   │
│  │ Guest (macOS ARM64)  │   │
│  │                      │   │
│  │ /Volumes/            │   │
│  │   darwin-runtime/    │   │
│  │     bin/             │   │
│  │     fixtures/        │   │
│  │     config/          │   │
│  └──────────────────────┘   │
└─────────────────────────────┘
```

## Troubleshooting

**"VM not installed"**: Run `./vm-image/launch.sh install` first.

**IPSW download fails**: Download manually from [Apple's developer site](https://developer.apple.com/download/) and set `VM_IPSW=/path/to/file.ipsw`.

**"Unsupported hardware"**: Virtualization.framework requires Apple Silicon. Intel Macs are not supported.

**Shared directory not visible in guest**: The VirtioFS share should auto-mount. If not, check System Settings → General → Sharing in the guest.

**Insufficient disk space**: The IPSW is ~13GB and the sparse disk image starts small but can grow to 64GB. Ensure adequate free space.

**Clean start**: Run `./vm-image/launch.sh clean` to remove all VM state, then `./vm-image/launch.sh install` for a fresh install.
