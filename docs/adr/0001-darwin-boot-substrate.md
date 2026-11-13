# M1 platform and boot decision

Status: selected for a bounded feasibility spike. The production substrate is conditional on the evidence below. The x86_64 QEMU VM is the initial reference platform. M1 remains in progress until its build/boot/runtime experiment and contract decisions are recorded; physical Mac and real-application qualification proceed later.

## Decision

Use an x86_64 Darwin guest built from the PureDarwin XNU distribution as the first boot candidate. Pin PureDarwin at `0163b2db225b85e3bf96fc25646ea290105ac298`, observed on `upstream-xnu-12377`. Preserve its `flake.lock` and exact submodules. Begin with its minimal image, open-source `xnu-loader`, UEFI firmware and QEMU q35. Use x86_64 Linux as the reproducible build host only. Burdettie supplies its runtime, application session, launch supervisor, policy interface and selected application compatibility above the Darwin substrate.

This decision establishes where to invest the first platform experiment. It does not establish that the Burdettie runtime already runs on PureDarwin, that an arbitrary Intel Mac boots it, or that the substrate enforces Burdettie policies. Those are separate gates.

Use Linux for the complete M1 build/VM path. Existing native macOS tests can remain reference evidence, and Linux ARM64 interpreter validation remains useful; neither is a prerequisite for the x86_64 OS experiment. Prefer x86_64 application slices on that lane; additional architectures require their own qualification.

## Why this candidate

Apple's XNU source describes the Darwin kernel and x86_64/ARM64 support, but a kernel tree alone does not supply a usable boot image, drivers, libSystem, dyld, init, filesystem or display stack. PureDarwin provides an existing integration candidate for those pieces. Its current handbook describes q35/OVMF boot, an ext4 root, launchd and an x86_64 shell. This materially reduces the initial platform integration question compared with designing a new kernel. These are upstream claims; Burdettie must reproduce the narrow checks it relies on.

The current Burdettie repository is a user-space runtime. `platform-darwin-host` wraps host services; `window-backend-darwin` depends on `winit`/`softbuffer`. The macOS target selects these backends, which does not establish that they work on bare PureDarwin. Rust standard-library/libSystem linkage and the display/input backend are explicit porting gates. Its VM demo downloads and installs a full macOS restore image through Apple's Virtualization framework on Apple Silicon. That VM remains a useful reference environment, but does not prove an independently assembled Burdettie system.

| Alternative | M1 disposition | Reason |
| --- | --- | --- |
| PureDarwin XNU image plus Burdettie runtime and session | Selected for feasibility | Closest existing boot integration to the Darwin product goal; requires runtime ABI, display and confinement qualification |
| Build directly from current Apple XNU source | Reserve | Adds bootstrap, drivers, userland and image assembly before the application/runtime question can be tested |
| New Rust kernel | Deferred | Rust components remain welcome; replacing kernel mechanisms adds a separate substantial research program before useful app workflows |
| Linux guest as shipping OS | Not selected by this decision | Existing portability/testing backend remains useful; changing the final substrate requires an explicit product decision |
| Full macOS VM with Burdettie session | Reference development environment | Does not satisfy independent OS boot acceptance |

## Version and source rules

Use the exact PureDarwin commit above rather than following the development branch during a run. Record the actual Nix version, lock hash, QEMU/OVMF derivations, generated image SHA-256, kernel version and any local patch hash. Pinning the branch alone is insufficient.

There is an upstream documentation mismatch. The pinned README still tells builders to supply Apple's `MacOSX11.3.sdk.tar.xz` and describes a different development branch. The pinned `nix/pkgs/toolchain/apple-sdk-pinned.nix` assembles a source-derived SDK; the current SDK handbook states that the proprietary SDK requirement has been removed. Follow the selected revision's build definitions and record any actual evaluation/build disagreement. Do not copy an Apple SDK into the public repository or treat a dependency's label as a distribution-rights review. Review the actual component closure and retain its separate notices before publishing an image.

## Initial target matrix

| Field | Selected VM experiment | Physical target |
| --- | --- | --- |
| Identifier | `puredarwin-x86_64-q35-tcg` | Unbound until hardware inventory |
| Guest CPU | x86_64; QEMU `IvyBridge,vendor=GenuineIntel` | Exact Intel CPU and model identifier required |
| Build host | x86_64 Linux with Nix flakes | Build host can be separate from target Mac |
| Firmware | Pinned OVMF X64 via upstream lock; `xnu-loader` | Actual firmware width and boot support must be recorded |
| Machine and resources | q35; 4 vCPUs; 4096 MiB RAM | Actual RAM and controller/device inventory |
| Root and storage | Minimal ext4 image; GPT/ESP; AHCI raw disk | Select external test media first; actual SATA/NVMe/USB controller required |
| Display and input | Standard VGA/firmware framebuffer; QEMU xHCI keyboard and mouse; serial console | Actual GPU/framebuffer and input controllers required |
| Network | Offline for initial proof; no NIC and no SSH forwarding | Record actual Ethernet/Wi-Fi device; do not inherit upstream generic driver support |
| Execution target | x86_64 Mach-O fixture, then x86_64 application slice | Same guest/application architecture unless separately justified |
| Security claim | None from boot success; capability report starts unknown/unsupported | Actual enforcement evidence required on this backend |

The q35 machine alias is pinned indirectly by the QEMU derivation; save the expanded command and its resolved machine version with the evidence. KVM with host passthrough may be a developer acceleration lane after the TCG baseline; it is a separate machine/CPU configuration and needs its own result.

The physical-target column is a deferred hardware-support track, not an M1 closure dependency. When physical support is added, collect model identifier, CPU model/architecture/features, RAM, firmware width/version, storage/controller, GPU/display path, keyboard/trackpad/USB controllers, and network device IDs. VM evidence does not establish support for that machine.

## Bounded spike and stop conditions

Run the stages in order. A stage may consume at most two focused engineer days before producing a written pass/fail/blocker result and a decision about the next attempt. This is an investigation budget, not a delivery-date estimate. Do not continue adding frameworks while a platform gate is unresolved.

1. **Source and toolchain preflight.** Obtain the pinned tree, initialize its exact submodules, keep the lock unchanged, and evaluate the minimal image recipe. Record missing inputs, actual build host and dependency closure. A clean minimal image build is the first pass condition. An unavailable environment is `blocked`, not a failed candidate and not a pass.
2. **Independent boot and persistence.** Boot a writable copy of the minimal image under the selected VM. Capture the loader, XNU, root mount, launchd and shell evidence. Write a fixed test string under `/var/tmp/burdettie-m1`, flush, stop QEMU, restart the same writable image and confirm the exact bytes. Boot the untouched base image separately as recovery. Save serial logs and image hashes. A shell from the old macOS IPSW VM does not satisfy this stage.
3. **Runtime ABI bridge.** First run a source-built x86_64 Mach-O console probe on the guest that exercises allocation, file I/O, thread creation and process exit. Then build/package the narrow Burdettie CLI runtime against the selected SDK/libSystem and execute a source-built x86_64 guest fixture through it. Record all unresolved symbols, minimum-version commands, framework dependencies and actual native/guest provenance. An Apple macOS-target Rust binary is not assumed to link or run merely because both systems are Darwin.
4. **Session and enforcement boundary.** Determine the concrete display/input path for Burdettie's software renderer without assuming Apple's host AppKit. Produce the exact process-launch and resource-confinement capability map consumed by the M1 security contract. Unknown mechanisms stay unsupported; do not map XNU presence or a successful sandbox shim to enforcement. PureDarwin's minimal console is a root shell for bringup. The Burdettie supervisor may need privileged mechanisms, but its app workers must run without supervisor authority. Permissive policy does not require running all guest apps as root. M3 completes the session boot integration; M5 proves confinement.
5. **Later physical boot probe.** This is outside M1 closure. If physical Mac support is selected later, use separate external test media and record boot selection, console, root mount, input and persistent writes. Device/installer/recovery qualification belongs to that hardware-support target; it does not block the VM preview or virtual-machine alpha.

If stages 2 or 3 cannot pass within the investigation budget, produce a gap list with estimated implementation work and decide whether to continue this candidate, use a known earlier pinned PureDarwin revision, or reassess the Darwin integration. A failed candidate does not silently authorize changing the shipping product to Linux.

## Reproducible developer commands

The following is a recipe for an equipped development host, not a record of an executed boot. Start in a new scratch checkout. These commands do not install to a physical disk.

```sh
git clone --branch upstream-xnu-12377 https://github.com/PureDarwin/PureDarwin.git puredarwin-m1
cd puredarwin-m1
git checkout --detach 0163b2db225b85e3bf96fc25646ea290105ac298
git submodule update --init --recursive
git status --short
nix build --no-update-lock-file --out-link result-minimal .#image-minimal
nix build --no-update-lock-file --out-link result-qemu \
  github:NixOS/nixpkgs/fd1462031fdee08f65fd0b4c6b64e22239a77870#legacyPackages.x86_64-linux.qemu
nix build --no-update-lock-file --out-link result-ovmf \
  github:NixOS/nixpkgs/fd1462031fdee08f65fd0b4c6b64e22239a77870#legacyPackages.x86_64-linux.OVMF.fd
mkdir -p m1-state
cp --reflink=auto result-minimal/puredarwin-minimal.img m1-state/disk.img
chmod u+w m1-state/disk.img
cp result-ovmf/FV/OVMF_VARS.fd m1-state/OVMF_VARS.fd
chmod u+w m1-state/OVMF_VARS.fd
sha256sum result-minimal/puredarwin-minimal.img
result-qemu/bin/qemu-system-x86_64 \
  -machine q35,accel=tcg -cpu IvyBridge,vendor=GenuineIntel \
  -smp 4 -m 4096 -vga std -display none \
  -fw_cfg name=opt/ovmf/X-PciMmio64Mb,string=2048 \
  -drive if=pflash,format=raw,unit=0,readonly=on,file=result-ovmf/FV/OVMF_CODE.fd \
  -drive if=pflash,format=raw,unit=1,file=m1-state/OVMF_VARS.fd \
  -device ich9-ahci,id=sata \
  -drive if=none,id=system,file=m1-state/disk.img,format=raw,cache=writeback \
  -device ide-hd,bus=sata.0,drive=system \
  -device qemu-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 -device usb-mouse,bus=xhci.0 \
  -nic none -serial mon:stdio
```

The QEMU/OVMF Nixpkgs revision above is the root `nixpkgs` input in the pinned PureDarwin lock. The command follows the pinned runner's boot, CPU, disk, firmware and input configuration, with network/audio omitted and the display directed to the serial terminal. It is a derived, unexecuted Burdettie experiment configuration, not a claim that this exact variant has already booted. Save its expanded arguments and package derivations with the result.

Use the final QEMU command for later boots; do not repeat the image or firmware copy steps over existing test state. The writable copy is essential: the upstream runner adds `snapshot=on` for a read-only Nix-store image, discarding changes at QEMU exit. The supplied upstream runner also expects `puredarwin.img` by default and includes network/SSH forwarding. The explicit command here selects the actual minimal image and an offline configuration instead.

Within the guest, use the same fixed string on both boots and compare exact output:

```sh
mkdir -p /var/tmp/burdettie-m1
printf '%s\n' burdettie-m1-persistence-v1 > /var/tmp/burdettie-m1/probe.txt
sync
cat /var/tmp/burdettie-m1/probe.txt
```

Exit QEMU completely, rerun the identical host command against the same image, then read the file again. Save an evidence record with separate `build`, `boot`, `persistence`, `runtime`, `physical_hardware` and `confinement` statuses. A pass in one field does not promote the others.

## Observed feasibility result in this session

The available execution host is Linux x86_64. Read-only checks found Git and a C compiler, but no Nix, QEMU, Rust/Cargo or Clang on PATH, and no `/dev/kvm`. No pinned PureDarwin checkout or image was present. A follow-up capacity check found about 17 GiB free. Upstream recommends budgeting 60 GB or more of Nix store for a full image build; the minimal image's exact peak build footprint has not been measured here. The independent boot experiment is blocked by the execution environment, not by absence of Mac hardware. No image was built or VM booted. KVM is optional because the baseline uses TCG. The preflight reports tool/source readiness only; disk capacity and the actual build remain separate checks. See [Mac-free execution options](../milestones/M1_WITHOUT_MAC.md).

## Sources

- [Burdettie README at the inspected source](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/README.md)
- [Burdettie VM guide](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/docs/VM_DEMO.md) and [actual VM launcher](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/vm-image/Sources/main.swift)
- [Burdettie Darwin host platform](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/platform-darwin-host/src/lib.rs) and [display backend dependencies](https://github.com/burdettadam/burdettie-os/blob/7bf375562ca9edf513c353da7ec3309c0337f457/crates/window-backend-darwin/Cargo.toml)
- [Current Apple XNU](https://github.com/apple-oss-distributions/xnu/blob/main/README.md)
- [Pinned PureDarwin image and runner source](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/image-contents.nix)
- [Pinned PureDarwin lock](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.lock) and [SDK recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/apple-sdk-pinned.nix)
- [PureDarwin getting started](https://puredarwin.org/handbook/getting-started.html), [SDK](https://puredarwin.org/handbook/building/apple-sdk.html), [boot chain](https://puredarwin.org/handbook/images/boot-chain.html), and [hardware evidence](https://puredarwin.org/handbook/images/hardware.html)
