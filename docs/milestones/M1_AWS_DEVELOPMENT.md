# M1 development on the retained AWS host

Status on 2026-11-14 UTC: **the original experiment has been retrieved and diagnosed; a corrected, bounded PureDarwin 20.5 bootstrap passes shell execution and exact-file persistence across two QEMU processes. M1 remains in progress.** The selected source-derived SDK and two Mach-O object probes also built successfully. Selected-image construction, executable linking, Burdettie runtime execution, Apple applications, display/session and enforcement remain separate gates.

The [sanitized bootstrap receipt](evidence/bootstrap-2026-11-14.json) records all three outcomes, input identities, timings and log hashes. Raw logs, writable disks, private recovery records, cloud identifiers and command receipts are retained outside the public repository.

## Host and experiment scope

The existing x86_64 Ubuntu 24.04 instance is reused under its active AWS Free plan: two host vCPUs, 8 GiB RAM and the existing encrypted 40-GiB root disk. No instance, volume, paid upgrade or infrastructure expansion was added. Existing services and data are retained. Capacity remains a measured constraint on this shared host.

| Component | Observed value for the completed bootstrap |
| --- | --- |
| Image | Official PureDarwin 20.5 minimal image, 274726912 bytes |
| Image SHA-256 | `ef65a816395680ef9210f401cebe3783c131badca1a942cb80e56bec97e717ac` |
| Release source | `1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290` |
| Corrected harness revision | `b0752ab4278e31be34253d9adacccad35b8fb262` |
| Corrected harness SHA-256 | `5076328a7c5fad280a7573b4f3aea21576db8dad721439d373d60696db30a3e8` |
| QEMU | Ubuntu `1:8.2.2+ds-0ubuntu1.18` |
| Firmware | Ubuntu OVMF matching 4-MiB CODE/VARS pair; both hashes in the receipt |
| Guest | q35, IvyBridge/GenuineIntel, TCG single-thread, two vCPUs, 2048 MiB RAM |
| Host limits | Non-root account, 3-GiB cgroup memory ceiling, no swap, 75% of one CPU, 64 tasks, 1250-second service limit |
| Access boundary | Private network namespace, read-only system view, run-directory write exception, private temporary files, no new privileges |
| Connectivity | No guest NIC or public monitor/display listeners |
| Timeout | 600 seconds per boot |

These Linux host controls bound the experiment. They do not qualify Burdettie's application security profiles. Each attempt uses a fresh output directory and writable image/firmware copies. The original checkout, disks and logs are preserved.

## Retrieved outcomes

| Experiment | Actual outcome | Evidence and interpretation |
| --- | --- | --- |
| `bootstrap-20261113-01` | **Fail** | Write phase timed out after 600.166 seconds with `timeout_waiting_for_prompt`; no verified challenge and no second boot. |
| `bootstrap-20261114-01` | **Interrupted** | The host stopped/restarted the service while scheduled package maintenance was active. Final report says `fail` / `interrupted`, with no completed phase. A restarted invocation refused the existing output directory. This is not a guest boot verdict. |
| `bootstrap-20261114-02` | **Pass, bootstrap scope** | Write boot 235.948 seconds; read boot 233.682 seconds. Both verified a fresh Darwin/x86_64 shell nonce and the required data response. Both QEMU processes exited before the next phase or final result. |

The original serial log shows a mounted root, devfs readiness, launchd and a decorated zsh prompt. The readiness matcher did not strip the observed terminal character-set escape, and it expected a prompt at the end of the entire buffer. Asynchronous service messages after the prompt defeated that check. No challenge was sent; the original run remains a failure.

The corrected harness strips the relevant terminal escapes and recognizes a rendered prompt line even when later console output follows. A prompt is only a readiness hint. Shell verification still requires a complete, fresh nonce response; echoes, partial lines and prefixes/suffixes do not qualify. Regression tests cover split terminal output, echoed challenges and exact-byte persistence across separate fake processes.

The successful follow-up changed the harness while preserving the image, QEMU, firmware, guest resources and CPU/memory limits. Independent review found exactly one standalone shell response and one data response per boot. Echoed commands contain separated nonce halves, never a complete expected response. The second boot creates an expected file and uses `cmp -s` against the retained file before printing its persistence marker. All four completed-run log hashes and byte counts were verified.

**Shutdown limitation:** the harness issues guest `sync` and then stops QEMU from the host. The second boot replays a dirty ext4 journal before the successful readback. This demonstrates persistence of the tested exact bytes through that termination/restart; it is not graceful guest shutdown, general filesystem durability or a warning-free boot. TCG feature warnings and guest service diagnostics remain in the logs.

## Selected-source preparation and capacity

The selected M1 candidate remains `0163b2db225b85e3bf96fc25646ea290105ac298`. Its complete shallow checkout and exact corecrypto submodule were obtained; `flake.lock` remains unchanged at SHA-256 `00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0`.

Nix 2.28.3 was installed in a non-root profile without a system daemon or shell-profile modification. Its official release tarball SHA-256 is `85d1847d06d5d56167796d3f61cd992908de84584db3e700da031a782b59ea22`. The first evaluation refused a shallow Git input without an explicit shallow attribute. The fixture recipe now supplies `submodules=1&shallow=1` with the exact revision, and has a regression test for that URL.

A subsequent **no-build** evaluation of `apple-sdk`, `native-ld`, `darwin-cross-toolchain` and `libsystem` passed in 232.21 seconds. It used no lock update, no import-from-derivation and zero build jobs. The combined plan lists **32 local derivations and 264 fetched paths: 916.73 MiB download, 4811.47 MiB unpacked**. At that checkpoint the host had 14387019776 free bytes, the Nix store occupied 2160553984 filesystem bytes and the source checkout occupied 683802624 bytes. Those numbers are observed preparation/planning data, not a measured build peak.

The shared-host reserve is 8 GiB. The combined unpacked download estimate leaves little room above that reserve for local outputs and temporary trees. Measure individual stages and enforce disk, memory, CPU and time limits before realization. The full-image 60-GB upstream guideline does not by itself reject the narrower fixture lane; the actual plan also does not prove that all stages fit. No selected-candidate image build or boot is established by this dry run.

The individual SDK plan was 3 local derivations and 72 fetched paths (412.25 MiB download, 1908.90 MiB unpacked). A bounded SDK-only build then passed in **75.11 seconds** using the unchanged selected source. The final SDK's stdio, pthread and Mach headers and libSystem link stub were checked and hashed. Free space afterward was **12243050496 bytes**, above the 8-GiB reserve; the 3-GiB growth guard was not reached. SDK warnings remain in its log. This is a source-derived SDK result, not an image, linked fixture or runtime result. Larger linker/libSystem stages were not launched: their raw Ninja job counts and additional LLVM/Debug build footprint still need explicit bounds.

The subsequent **offline object-only probe passed** using that already-realized SDK and its LLVM 21.1.8 compiler: 12 seconds of input preflight followed by 11 seconds for compilation and inspection. `simple_cli` produced a 768-byte x86_64 `MH_OBJECT`; `m1_abi_probe` produced a 5136-byte x86_64 `MH_OBJECT`. Both report minimum OS 26.5.0 and SDK command field 0.0.0; the selected SDK is bound independently by its input and header hashes. The run used no new dependency download and finished with 12219154432 disk bytes free, a stage decrease of 24576 bytes. Symbol inspection was not run because LLVM nm was absent. The [object receipt](evidence/objects-2026-11-14.json) preserves those exact boundaries: no executable link, guest execution, Burdettie runtime execution or enforcement was attempted.

See [the fixture and ABI gate](M1_FIXTURE_ABI.md) for the pinned SDK/linker/libSystem recipe and source-owned C probe. It refuses unavailable outputs and never launches a dependency build implicitly.

## Runtime and security work

[PR #7](https://github.com/burdettadam/burdettie-os/pull/7) publishes separately reviewed runtime prerequisites: optional native dialogs for the headless Darwin CLI, correct immediate-exit behavior and interception of actual interpreter exit shims. Host tests and dependency-graph checks pass; Darwin linking and execution inside the selected guest remain unqualified. Normal callback ownership and drain-before-unmap still need implementation.

[PR #5](https://github.com/burdettadam/burdettie-os/pull/5) already contains admission work for native, interpreter and typestate routes. It is a separate, unmerged change; it does not establish complete mode, provider or lifecycle enforcement. Remaining integration is tracked in [issue #6](https://github.com/burdettadam/burdettie-os/issues/6).

The [backend allocation](../contracts/BACKEND_ALLOCATION.md) assigns 17 control families to concrete candidate mechanisms and qualification tests. All start unavailable/unsupported with no qualification evidence. The contract validator now binds evidence to the specific capability, evidence kind and scopes as well as platform context. Neither that consistency check nor this bootstrap promotes a security capability to enforced.

## Remaining gates

1. Bound and realize the remaining linker/libSystem inputs within the existing host budget, then link and inspect the public fixture executables. The SDK and object-compilation stages have passed.
2. Build and independently boot the selected image, including its own persistence, recovery and graceful-shutdown assessment.
3. Qualify Rust standard-library/libSystem linkage and execute the x86_64 fixture through Burdettie inside that guest.
4. Resolve the concrete worker-lifetime, display/input and backend gaps before advertising the corresponding behaviors.
5. Respect all repository checks. The existing `cargo-deny` failure for unmaintained `ttf-parser` is tracked in [issue #2](https://github.com/burdettadam/burdettie-os/issues/2); it is not suppressed by this work.

Physical Mac ownership and a real Apple application are not M1 prerequisites. The independent OS, real-application and selectable-security goals remain unchanged, and retain their own later acceptance evidence.

Sources: [20.5 release](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5), [matching release runner](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/nix/image-contents.nix), [selected source lock](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.lock), and [upstream build budget](https://puredarwin.org/handbook/getting-started.html).
