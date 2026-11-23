# PureDarwin source refresh — 2026-11-23

Status: **new current source selected; current-source preflight and image build not yet run**.

## Decision

Use the current upstream development head of `PureDarwin/PureDarwin`, pinned to:

- Branch: `main`
- Commit: `0c1990c803c13b68cad4df3f6dc34ba4306fc789`
- Commit date: 2026-11-22 06:56:03 UTC
- Commit subject: `kc: rge in every collection, and em, igc and rge back in the default one now the FreeBSD layer is one shared kext`
- Repository README at this commit identifies `main` as the development branch.
- Build host and initial guest remain `x86_64-linux` and x86_64 Darwin in QEMU.

This replaces the previous candidate `0163b2db225b85e3bf96fc25646ea290105ac298`. GitHub's compare endpoint reports the current `main` is **155 commits ahead** of that previous candidate. The old `upstream-xnu-12377` branch is **65 commits behind current main**. Do not select that stale branch or the older release image as the current source baseline.

## Why not the 20.5 release image

PureDarwin's official `20.5` release was published on 2026-10-17. Its minimal image is the image used by the November 14 bootstrap experiment. That experiment remains valid historical evidence for shell automation and exact-byte persistence on that specific 20.5 image. It is not a boot result for the refreshed current source.

For current development, build the minimal image from the pinned current source using the upstream Nix flake. The current README documents:

```sh
nix build .#image-minimal
nix run .#vm
```

The README says the build uses PureDarwin's own open-source SDK and does not require an Apple SDK. Nix is the supported build system; do not replace it with a direct CMake build.

## Exact source inputs inspected

| Input | Current value |
| --- | --- |
| PureDarwin source | `0c1990c803c13b68cad4df3f6dc34ba4306fc789` |
| Branch | `main` |
| Flake lock Git blob | `1795642f943e71c7abf6a080e011068a19861bbe` |
| Flake lock SHA-256 | `00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0` |
| Root Nixpkgs revision | `fd1462031fdee08f65fd0b4c6b64e22239a77870` |
| iig-tools | `28010c688caab7eb5400b9c98faa5b1e538e0108` |
| kc-tools | `05e5ed6a42533cfc4330559484e74b6dedeae51e` |
| xnu-loader | `2e0ccce2bd88eec1194e78eaeb46a91601478555` |
| corecrypto submodule | `fa94ec601f8d6b3a2a39b974f29c5acaf8a9fa2d` |

The flake lock is byte-identical to the prior candidate's lock. The corecrypto submodule is not: the prior candidate used `f577aafe4b9a89727968666237d20cc51d2f770d`. The current main source tree also contains 155 commits of changes since the prior candidate. An unchanged lock file does not make old derivations, SDK outputs, linker outputs or image evidence current-source evidence.

The current flake declares `x86_64-linux` as a supported build system, and its README exposes `image-minimal` and the QEMU VM runner. Its Nix inputs retain the XNU 12377 toolchain family through the `kc-tools` and `xnu-loader` inputs. PureDarwin's current project documentation identifies Darwin 26.5 / XNU 12377 and x86_64 as its release target.

## Documentation discrepancy

The PureDarwin website handbook still describes `upstream-xnu-12377` as the development branch and `main` as frozen. The current repository's main-branch README instead says development happens on `main`; the live GitHub branch state confirms that main has newer commits. For this source refresh, the exact repository branch and commit metadata plus the README at the pinned commit are authoritative. Re-check the upstream branch model before any future update rather than assuming the website and repository docs remain synchronized.

## Evidence that remains historical

The following evidence is retained, not deleted:

- The 20.5 minimal-image bootstrap and its exact-byte persistence result.
- The no-build plan, SDK realization, object compilation, libtapi staging and native-linker build evidence tied to `0163b2db225b85e3bf96fc25646ea290105ac298`.
- The previous source's exact flake lock and Nixpkgs input identity.

These results must not be represented as current-main preflight, build, boot, ABI or runtime qualification. The prior outputs may be reused only if a fresh evaluation proves the exact derivation and output identities are unchanged and the current host still satisfies all capacity guards. Otherwise rebuild the necessary stages. Do not overwrite or delete retained failed trees, disks, logs, or old evidence.

## Next bounded work

1. Inspect the existing AWS host in `us-east-2` and verify its current instance, disk, service and cost safeguards before running commands. Do not expand infrastructure or use a paid upgrade.
2. Confirm the retained checkout is clean, fetch the exact `0c1990c803c13b68cad4df3f6dc34ba4306fc789` commit and exact `external/corecrypto` submodule, and verify the `flake.lock` hash. Do not follow a moving branch during the experiment.
3. Run `nix flake show`, metadata/evaluation and a no-build plan for `image-minimal` and the QEMU runner. Record derivations, fetched paths, download/unpacked estimates, current free space and the 8-GiB reserve before realizing anything.
4. Reassess the existing two-vCPU / 2048-MiB / single-thread-TCG profile against the current `vm` runner. It remains provisional until the current image boots under the bounded host controls.
5. Build the current `image-minimal` only if the measured plan fits the existing host limits. Record its SHA-256, Nix store identity, QEMU/OVMF inputs and full boot logs.
6. Independently verify fresh nonce-based shell access, exact persistence across separate QEMU processes, graceful shutdown, recovery, then link and execute the x86_64 fixtures. Qualify Burdettie runtime and policy enforcement separately.

No current-main checkout, current-main image build, boot, fixture link, guest execution or Burdettie enforcement is claimed by this source-selection update.

## Upstream references

- [Current PureDarwin repository](https://github.com/PureDarwin/PureDarwin)
- [Pinned current README](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/README.md)
- [Pinned current flake](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/flake.nix)
- [Pinned current lock](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/flake.lock)
- [Pinned current corecrypto submodule](https://github.com/PureDarwin/PureDarwin/tree/0c1990c803c13b68cad4df3f6dc34ba4306fc789/external/corecrypto)
- [Previous 20.5 release](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5)
