# M1 development on the retained AWS host

Status on 2026-11-14 UTC: **the original experiment has been retrieved and diagnosed; a corrected, bounded PureDarwin 20.5 bootstrap passes shell execution and exact-file persistence across two QEMU processes. M1 remains in progress.** The selected source-derived SDK and two Mach-O object probes also built successfully. The later native-linker attempt has now been retrieved: it failed at its wall-time limit while building libtapi, before completing the linker. Selected-image construction, executable linking, Burdettie runtime execution, Apple applications, display/session and enforcement remain separate gates.

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

The individual SDK plan was 3 local derivations and 72 fetched paths (412.25 MiB download, 1908.90 MiB unpacked). A bounded SDK-only build then passed in **75.11 seconds** using the unchanged selected source. The final SDK's stdio, pthread and Mach headers and libSystem link stub were checked and hashed. Free space afterward was **12243050496 bytes**, above the 8-GiB reserve; the 3-GiB growth guard was not reached. SDK warnings remain in its log. This is a source-derived SDK result, not an image, linked fixture or runtime result. At that SDK checkpoint, larger linker/libSystem stages had not been launched. The later bounded linker failure and its separately measured recovery capacity are recorded below.

The subsequent **offline object-only probe passed** using that already-realized SDK and its LLVM 21.1.8 compiler: 12 seconds of input preflight followed by 11 seconds for compilation and inspection. `simple_cli` produced a 768-byte x86_64 `MH_OBJECT`; `m1_abi_probe` produced a 5136-byte x86_64 `MH_OBJECT`. Both report minimum OS 26.5.0 and SDK command field 0.0.0; the selected SDK is bound independently by its input and header hashes. The run used no new dependency download and finished with 12219154432 disk bytes free, a stage decrease of 24576 bytes. Symbol inspection was not run because LLVM nm was absent. The [object receipt](evidence/objects-2026-11-14.json) preserves those exact boundaries: no executable link, guest execution, Burdettie runtime execution or enforcement was attempted.

See [the fixture and ABI gate](M1_FIXTURE_ABI.md) for the pinned SDK/linker/libSystem recipe and source-owned C probe. It refuses unavailable outputs and never launches a dependency build implicitly.

## Bounded linker continuation

The [bounded recipes](M1_BOUNDED_BUILD_RECIPES.md) now explicitly constrain raw
Ninja invocations and carry the derived native linker through the cross-toolchain,
compiler-rt and libSystem graph. The compiler-rt staging recipe copies only its
writable subtree and retains all siblings through immutable source symlinks. The
complete LLVM source input and upstream build/target flags remain required.
These implementation changes do not establish that the downstream stages fit or
build successfully.

The actual native-linker metadata plan passed in **20.09 seconds**, reusing the
unchanged realized SDK. It listed **20 local derivations and 190 fetched paths:
356.27 MiB download and 1085.00 MiB unpacked**. The plan queried cache metadata
without realizing or downloading dependencies. Its full private capture and
eight-file checksum manifest were recovered and verified. The [sanitized
receipt](evidence/native-linker-2026-11-14.json) records the exact source, recipe,
SDK, candidate and plan hashes.

One bounded build attempt started at **08:36:22 UTC**. It used one build job/core,
25% of one CPU, a 1.5-GiB memory ceiling, no swap, an 8-GiB disk reserve, a 3-GiB
growth guard, a 1200-second phase deadline and a 1500-second service deadline.
Remote builders and automatic garbage collection were disabled; the command
requested retention of failed build trees and printed build logs. No instance or disk expansion was introduced.

The final receipt and complete **373573-byte build log** have now been recovered
and verified against their hashes. All 20 members of the private recovery
manifest also matched. The receipt reports **failure after 1201.12 seconds**, at
**08:56:24 UTC**, with `stage wall-time budget reached`. The last compile marker
was libtapi at **26%**, building `CodeGenDAGPatterns.cpp` for LLVM TableGen. The
harness stopped its own build process group; Nix reported interruption and kept
the failed build tree. The earlier 08:53:16 observation and its retrieval gap
remain in the sanitized receipt as historical evidence.

The full captured log contains warnings but no compiler-fatal diagnostic. The
cgroup memory peak was **1343332352 bytes**; memory-high events were **3160**, while
memory-max, OOM and OOM-kill events were all **zero**. The observed minimum free
disk space during the phase was **10114813952 bytes**, above the 8-GiB reserve.
This result establishes a time-budget failure under the configured limits. It
does not establish a compiler defect, a completed libtapi/native-linker output,
fixture linking or selected-guest execution.

The later **18:01:21 UTC recovery checkpoint** measured **9554071552 free bytes**.
After preserving the 8589934592-byte reserve, only **964136960 bytes (about
0.90 GiB)** remained. The same driver's 3-GiB growth-budget preflight requires
11811160064 initial free bytes and would reject another attempt at that
checkpoint. The retained failed tree occupies 341536768 allocated bytes and
remains preserved. These are separate recovery measurements, not the build's
terminal free-space values or a new build-peak qualification.

Another Nix invocation does not resume the retained failed tree. Any changed
source-staging recipe needs independent review, new derivation/output
identities, an offline plan and fresh capacity measurements before a bounded
realization. Keep full source inputs, the failed tree, all VM disks/logs and
existing services. No resource expansion or reduction of the reserve follows
from this failure.

## Libtapi source-staging metadata plan

The separately reviewed [opt-in staging
recipe](M1_LIBTAPI_SOURCE_STAGING.md), at Burdettie
`f8c59175784edb89e0087734d5020d142256297d`, passed its actual retained-host plan
in **24.08 seconds**, finishing at **18:46:00 UTC**. Its status is
**`planned_not_built`**. The complete captured result and 104-byte dry-run log
were hash-verified. The dry-run listed only the new libtapi candidate derivation,
with no dependency fetch paths, downloads or realization. The retained SDK and
all 12 unchanged-recipe fields matched; the existing native-linker and downstream
helpers remain unchanged.

The complete source remains present at **217,415,680 allocated bytes**. The
inventory proposes a **50,094,080-byte staging allocation estimate**, including
74 private file copies and links for the other 10,592 regular files. This metadata plan ran no actual
unpack or compilation. The estimate excludes build objects, generated data,
linking, outputs and additional filesystem metadata. The full source manifest
remains on the host; the public [plan
receipt](evidence/libtapi-plan-2026-11-14.json) records its hash and the local
capture limitation.

The metadata phase observed a minimum **9,450,496,000 free disk bytes** and
finished with **9,525,194,752 bytes**, leaving **935,260,160 bytes** above the
unchanged 8-GiB reserve. Its cgroup memory peak was **657,637,376 bytes**, with
no memory-high, memory-max or OOM events. These are metadata-phase measurements,
not a new build-peak qualification. A bounded candidate build and output
inspection remain pending in this receipt. The earlier failed tree, full source,
VM artifacts and existing services remain preserved.

A separate bounded staged-libtapi build was launched at **19:00:16 UTC** from
`f8c5917`. Its final result has not yet been retrieved. That subsequent launch is
outside the historical plan receipt and does not establish successful
realization, output inspection or native linking.

## Runtime and security work

[PR #7](https://github.com/burdettadam/burdettie-os/pull/7), at `d3ee4a2dd9483193bb3d01e38e4b2051751fdb17`, combines optional native dialogs, immediate-exit handling and launch-owned normal cleanup with the admission changes from #5. The conflict resolution retains both the initializer-aware exit status and the evaluated security report. Cleanup retains callback ABI/DSO/image ownership, runs ARM64 callbacks through the suspended interpreter, shares the instruction budget with cleanup and drains before image unmapping. All six functional CI jobs passed at this exact head, including 17 admission tests and 17 isolated lifecycle cases. The required dependency-policy and history-scan checks remain failed; see the [runtime integration receipt](evidence/runtime-integration-2026-11-14.json). Native explicit exit, worker lifetime, shared streams and unsupported lifecycle routes remain documented limits. These host results do not qualify Darwin linking or execution inside the selected guest.

[PR #5](https://github.com/burdettadam/burdettie-os/pull/5) contains the admission work for native, interpreter and typestate routes now included in #7. Neither PR has merged into main or this platform branch, and admission does not establish complete mode, provider or lifecycle enforcement. Remaining integration is tracked in [issue #6](https://github.com/burdettadam/burdettie-os/issues/6).

The [backend allocation](../contracts/BACKEND_ALLOCATION.md) assigns 17 control families to concrete candidate mechanisms and qualification tests. All start unavailable/unsupported with no qualification evidence. The contract validator now binds evidence to the specific capability, evidence kind and scopes as well as platform context. Neither that consistency check nor this bootstrap promotes a security capability to enforced.

## Remaining gates

1. Resolve the measured linker time/capacity blocker within the unchanged host budget, then realize and inspect the remaining inputs. The existing attempt has been retrieved and diagnosed; its failed tree remains preserved. The SDK and object-compilation stages have passed; linked executable fixtures remain unqualified.
2. Build and independently boot the selected image, including its own persistence, recovery and graceful-shutdown assessment.
3. Qualify Rust standard-library/libSystem linkage and execute the x86_64 fixture through Burdettie inside that guest.
4. Resolve the concrete worker-lifetime, display/input and backend gaps before advertising the corresponding behaviors.
5. Respect all repository checks. At `f8c5917`, all six functional CI jobs and the 100 script/37 contract tests pass, while `cargo-deny` still fails for unmaintained `ttf-parser` and the history scan still reports two public-source checksum findings. The [plan receipt](evidence/libtapi-plan-2026-11-14.json) links those exact-head checks. Neither failure is suppressed by this work.

Physical Mac ownership and a real Apple application are not M1 prerequisites. The independent OS, real-application and selectable-security goals remain unchanged, and retain their own later acceptance evidence.

Sources: [20.5 release](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5), [matching release runner](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/nix/image-contents.nix), [selected source lock](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.lock), and [upstream build budget](https://puredarwin.org/handbook/getting-started.html).
