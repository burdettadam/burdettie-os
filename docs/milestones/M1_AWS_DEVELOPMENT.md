# M1 development on the retained AWS host

Status on 2026-11-13 UTC: **development environment prepared; the first bootstrap experiment was started, but its final result has not been retrieved**. The last observation showed an active QEMU process and Darwin kernel/device output. The AWS console session then required sign-in. A verified shell, persistence result, Burdettie runtime result and security-enforcement result are not established by this checkpoint.

## Host and experiment scope

The existing x86_64 Ubuntu 24.04 development instance is being reused under the active AWS Free plan. It has two vCPUs, 8 GiB RAM and an existing encrypted 40-GiB root disk. The initial census found about 18 GiB of free disk and 4.8 GiB of available memory. No instance or volume expansion was made. Existing workloads and their stored data remain separate from the Burdettie workspace.

This is sufficient for the bounded prebuilt-image experiment. It is not a measured capacity result for building the selected OS image from source. The host cannot reserve upstream's recommended 60-GB Nix-store budget for a full image build on its current root disk; the minimal-image peak footprint still needs measurement. Nix, the pinned source checkout and the Linux fixture build recipe remain outstanding.

| Component | Observed or selected value |
| --- | --- |
| Bootstrap image | Official PureDarwin 20.5 minimal image, 274726912 bytes |
| Image SHA-256 | `ef65a816395680ef9210f401cebe3783c131badca1a942cb80e56bec97e717ac` |
| Release source | `1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290` |
| Harness source | Burdettie commit `5800f5ec505c427bc335215d028e2ec9618ebfee` |
| Harness SHA-256 | `5b2653a72861b8d28c40fd8c21e692fb3ef57ab7c4f94ae672bb35daebaf0519` |
| QEMU package | Ubuntu `1:8.2.2+ds-0ubuntu1.18` |
| OVMF package | Ubuntu `2024.02-2ubuntu0.10`; matching 4-MiB CODE/VARS pair |
| Guest configuration | q35, IvyBridge/GenuineIntel, TCG single-thread, two vCPUs, 2048 MiB RAM |
| Host process limits | Non-login `burdettie-dev` account, 3-GiB cgroup memory ceiling, no swap, 75% of one CPU, 64 tasks, 1250-second service limit |
| Host access boundary | Private network namespace, read-only system view, run-directory write exception, private temporary files, no new privileges |
| Guest connectivity | No NIC; no monitor/display listeners |
| Per-boot timeout | 600 seconds; two distinct QEMU processes required for a persistence pass |

The downloaded image passed its checksum check. The host checkout and harness also matched the exact revisions above before launch. Those checks establish input identity. Guest behavior needs the separate runtime evidence. The Linux host controls protect this experiment; they do not qualify Burdettie's application security profiles.

## Workspace and recovery

The isolated workspace is `/var/lib/burdettie-dev`:

- `assets/puredarwin-minimal-20.5.img` is the verified, read-only input image.
- `src/burdettie-os` contains the pinned public source checkout.
- `runs/bootstrap-20261113-01` contains private writable guest/firmware copies and diagnostic outputs.
- `runs/bootstrap-20261113-01/result.json` is the machine-readable experiment result when available.

The bounded service is `burdettie-bootstrap-20261113.service`. A separate host stop timer was armed and verified. Exact cloud resource identifiers, account/credit observations, access details and command receipts belong in the private execution handoff, not this public repository.

After restoring console access, read the existing result before considering another run:

```sh
sudo systemctl show burdettie-bootstrap-20261113.service \
  -p ActiveState -p SubState -p Result -p ExecMainStatus \
  -p MemoryPeak -p CPUUsageNSec
sudo cat /var/lib/burdettie-dev/runs/bootstrap-20261113-01/result.json
sudo tail -n 80 /var/lib/burdettie-dev/runs/bootstrap-20261113-01/write-serial.log
```

`ActiveState=active` and a service's interim `Result=success` do not establish that the experiment passed. Use the final JSON, exit status, nonce response and second-process readback together. Preserve the original run and logs; subsequent experiments use a new output directory and unit name.

## What the first observation established

The serial log showed the kernel enumerating the ATA disk, EFI partition, ext4 root partition, USB keyboard and mouse. The last captured report still had `status: running` and no completed phases. Device discovery does not establish a mounted root filesystem or working userland shell.

The matching release explicitly allows a long wall-clock gap under TCG between media probing and devfs readiness. A `devfs not ready` retry message alone is therefore insufficient to diagnose a missing disk. Let the bounded run produce its actual result and retain that evidence. [Matching storage-driver source](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/src/Kernel/Extensions/IOStorageFamily/IOMediaBSDClient.cpp).

## Next decisions

1. Retrieve the existing result and logs after sign-in. Record `pass`, `fail` or `blocked` from actual evidence; do not substitute the current checkpoint for a final report.
2. If boot times out, remove the toolchain-version difference as a controlled follow-up. The release's pinned Nixpkgs input supplies QEMU 10.2.4 and edk2/OVMF 202602. The Ubuntu packages used here are an explicitly recorded variant, not a demonstrated cause of failure. Evaluate the download/closure sizes of just those pinned tools before installing or building them on the shared disk.
3. Record any successful older-release boot as bootstrap evidence only. The selected M1 candidate remains `0163b2db225b85e3bf96fc25646ea290105ac298` and needs its own image, runtime and platform results.
4. Continue the Linux/PureDarwin x86_64 fixture build recipe and runtime ABI probe, then the shared supervisor/policy work. Physical Mac hardware remains unnecessary for this work.

Sources: [release](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5), [release lock](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/flake.lock), [pinned QEMU](https://github.com/NixOS/nixpkgs/blob/fd1462031fdee08f65fd0b4c6b64e22239a77870/pkgs/by-name/qe/qemu/package.nix), [pinned edk2](https://github.com/NixOS/nixpkgs/blob/fd1462031fdee08f65fd0b4c6b64e22239a77870/pkgs/by-name/ed/edk2/package.nix), [upstream build budget](https://puredarwin.org/handbook/getting-started.html).
