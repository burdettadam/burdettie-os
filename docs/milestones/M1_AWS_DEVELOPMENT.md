# M1 development on the retained AWS host

## Current PureDarwin source selection — 2026-11-23

The current candidate is PureDarwin `main` at `0c1990c803c13b68cad4df3f6dc34ba4306fc789`, not the prior `0163b2d` checkout. The current repository README documents `nix build .#image-minimal` and `nix run .#vm`. The official 20.5 image bootstrap and all source-specific plans/builds below remain historical evidence for their exact older inputs; they do not qualify the current main tree. The flake lock is unchanged, but the corecrypto submodule changed and the source tree is 155 commits newer. Fresh checkout, submodule verification, no-build evaluation and capacity review are required before any realization. See [the source refresh record](M1_PUREDARWIN_SOURCE_REFRESH_20261123.md).


Status on 2026-11-14 UTC: **the staged native linker built successfully and passed installed-output inspection; its complete selected evidence archive is recovered and independently reviewed. M1 remains in progress.** The [recovered evidence](evidence/staged-native-linker-recovered-2026-11-14.json) records the full result, build log, registration sidecars, loader/version checks and sampled resource history. The earlier [terminal observation](evidence/staged-native-linker-terminal-observation-2026-11-14.json) remains unchanged as a historical record of the transfer interruption. The staged libtapi build, selected SDK and two Mach-O objects have separate successful evidence. The older PureDarwin 20.5 bootstrap passes its narrow shell/persistence scope; the original native-linker attempt remains a retained wall-time failure. Fresh downstream verification, executable linking, selected-image construction, Burdettie runtime execution, Apple applications, display/session and enforcement remain separate gates.

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

## Historical source preparation and capacity — superseded 0163b2d candidate

The historical M1 candidate was `0163b2db225b85e3bf96fc25646ea290105ac298`. Its complete shallow checkout and exact corecrypto submodule were obtained; `flake.lock` remains unchanged at SHA-256 `00c598d3efca4bbda302fec4a43fffbfa787dc642c8c0b2c4202c65339e684d0`. This entire section records work on that superseded source, not current-main validation.

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

## Completed staged-libtapi build

The separate build from `f8c5917` started at **19:00:16 UTC** and finished at
**20:08:20 UTC**, with its realization phase passing in **4,082.6 seconds**.
The complete terminal result, 1,235,828-byte build log, registered-output sidecar
and unit journal have been recovered and hash-verified. The [build
receipt](evidence/libtapi-build-2026-11-14.json) is additive; the earlier failed
build and metadata-only plan keep their original outcomes.

The new output contains the 4,910,128-byte x86_64 ELF `libtapi.so.12git`, its
relative `libtapi.so` link and eight regular public headers. Its registered NAR
size is **4,940,088 bytes**. The inspected references contain the existing
glibc and GCC runtime outputs, with no complete-source or temporary-build path
reference. Actual staging allocated **50,089,984 bytes**. The unchanged-source
staging proof applies before the upstream patch and setup hooks; it is not a
claim that this build rehashed the source after compilation.

The monitored build observed a minimum **8,995,893,248 free disk bytes** and
finished with **9,317,019,648 bytes**. Maximum observed whole-host disk growth
was **526,663,680 bytes**, within its 640-MiB stop budget. The cgroup peaked at
**1,096,855,552 memory bytes**, with no memory-high, memory-max or OOM events.
The existing 25%-of-one-CPU quota, 1280/1536-MiB memory high/max, no swap,
one-job/one-core build and 8-GiB disk reserve were retained. Upstream compiler
warnings remain in the full log. The old failed tree, sources, disks and logs
remain preserved, and k3s was active at capture. This completed library build
does not predict a later stage's peak capacity.

## Staged-native-linker metadata plan

The opt-in [linker rebind](M1_STAGED_LIBTAPI_LINKER.md), executed at
`e5ff8bf1fc332b6fc0c08dd904a38e8de34f8af4`, passed its metadata phase in
**32.21 seconds**, finishing at **20:18:42 UTC**. The [separate
receipt](evidence/staged-linker-plan-2026-11-14.json) binds the exact successful
libtapi result, unchanged SDK, fresh complete-source inventory check, installed
file checks, NAR metadata and default offline store verification.

Its dry-run lists **nine local derivations and zero fetch paths**. No native
compilation, dependency download or realization ran. The original linker,
previous bounded linker, rebound baseline and new candidate have distinct
identities; the source, other flags, install phase and bootstrap toolchain stay
unchanged. The metadata phase finished with **9,289,572,352 free bytes**, or
**699,637,760 bytes** above the reserve. Its observed memory peak was
**741,339,136 bytes**, with no high/max/OOM events. These are preparation and
metadata measurements, not native-build capacity evidence.

The complete helper result and selected ten result/log/check files were
recovered and verified. Two initial single-response readers refused their
payload-size bound; their failures remain retained. A reviewed chunk capture
retrieved the existing records without rerunning the plan. The new
[downstream planner and shared native inspector](M1_STAGED_LIBRARIES.md)
require an actual successful native build, registered content, observed loader
resolution of this staged libtapi and a bounded version probe before planning
cross-toolchain, compiler-rt or libSystem. Their source tests do not supply that
live result.

## Recovered native-linker build and installed-output inspection

The build from `dd1c5710c12bdcf23ba091eaa09c859a9a7fec2a` started at
**21:00:39 UTC** and finished at **21:08:52 UTC**. Its complete recovered result
records five passing phases: exact source fetch (**1.12 seconds**), exclusive
worktree preparation (**1.09 seconds**), input verification (**14.39 seconds**),
native-linker realization (**459.99 seconds**) and installed-output inspection
(**14.29 seconds**). Every phase has an integer-zero exit and explicitly records
no timeout. The [new recovered-evidence
receipt](evidence/staged-native-linker-recovered-2026-11-14.json) binds these
results to the original plan, source, SDK, staged TAPI and reviewed code.

The **117,306-byte compressed archive**, **700,539-byte raw bundle** and all
**32 selected files** have been retrieved and rehashed. The complete result,
**292,452-byte build log**, registration sidecars and inspection streams agree;
there are no missing selected files or capture-consistency errors. Independent
review passed **116 evidence/contract checks**, including the actual downstream
consumer's receipt validator. These were checks of recovered records, with no
build or synthetic-test rerun. The earlier [terminal
observation](evidence/staged-native-linker-terminal-observation-2026-11-14.json)
retains its historical `driver_reported_success_full_recovery_pending` status;
the connection interruption did not require rebuilding the output.

The historical inspection binds the exact derivation and **4,831,360-byte NAR**,
the regular `ld` wrapper, x86_64 ELF `ld.real` and `strip`, and their complete
file hashes. Default offline store verification passed. Static ELF metadata
and the checked loader's `--list` output both resolve `libtapi.so.12git` to the
inspected staged library; the original TAPI output is absent from native-output
references. The bounded `ld -v` check passed and reports TAPI **1300.6.5**. Its
linker version is the literal token **`LD64_VERSION_NUM`**, so a numeric ld64
version is not established. Compiler warnings remain in the full log.

Across the recorded resource samples, minimum free disk was
**9,175,244,800 bytes** and the maximum observed whole-host net decline from the
pre-dispatch baseline was **203,493,376 bytes**. This stayed within the
**512-MiB growth bound** and **8-GiB reserve**. Minimum available memory was
**3,280,130,048 bytes**, and the driver's direct cgroup `memory.peak` was
**553,099,264 bytes**, with no high/max/OOM events. The journal separately
reports **478.5M** memory peak; those two measurements are retained without
equating them. CPU, memory, swap and process bounds remained unchanged. These
are this completed stage's sampled history, not a continuous sample series,
current host census or capacity proof for a future stage.

The terminal capture observed the unit inactive with both PIDs zero and k3s
active. Collected-unit defaults are not used as build-success proof. The two
complete source-manifest contents remain host-only; their recovered matching
hashes and inventory summaries are distinct from locally inspected manifest
contents. The historical native-evidence prerequisite is now satisfied. Before
downstream planning, the unchanged consumer must freshly verify the current
source, registered outputs, file inventories, store contents and loader/version
probes under a fresh capacity census. Cross-toolchain/compiler-rt/libSystem
realization, executable fixtures and the selected guest remain unqualified.

## Runtime and security work

At the historical `aade8484a773bfa88433526a2039084cc31d6b20` checkpoint, [PR #7](https://github.com/burdettadam/burdettie-os/pull/7) combined optional native dialogs, launch-owned cleanup, #5 admission, #8 maintained fonts/CFConstantString alignment and the typed NSRange adapter. All seven full-CI jobs passed there, including Security Scanning, 17 admission tests, the 17-child lifecycle harness, six entitlement-report tests, two CFConstantString regressions and the 15-child NSRange harness on macOS ARM64. The separate Linux x86_64 regression established the exact-IMP adapter, and the original NSCharacterSet fixture passed its semantic exit check while its formatted variadic values remained corrupted.

At that same historical checkpoint, cargo-audit retained the paste warning; default and both Linux-target cargo-deny checks passed. Repository hygiene failed on two public checksum findings. The [runtime/platform receipt](evidence/runtime-platform-integration-2026-11-14.json) retains `aade848`, preceding `d865636` and the platform parent's checks; the [older integration record](evidence/runtime-integration-2026-11-14.json) remains unchanged. Native explicit exit, worker lifetime, shared streams and unsupported lifecycle routes remain limits.

The M1 platform feature branch now includes [PR #7](https://github.com/burdettadam/burdettie-os/pull/7) through `c6b40679da2a814008bc5f6ec030e784c369d0ff`. The [bounded printf adapter](https://github.com/burdettadam/burdettie-os/blob/c6b40679da2a814008bc5f6ec030e784c369d0ff/docs/INTERPRETED_PRINTF.md) adds explicit Darwin ARM64 stack-format handling and interpreter-owned immutable string exports. The original ARM64 fixture executes on the Linux x86_64 interpreter host with a zero exit and the exact 415-byte stdout golden, without a missing-input skip. The [source-bound receipt](evidence/interpreted-printf-2026-11-14.json) retains the earlier corrupted output and the nested-invalidation red/green regression, and separates the passing local checks from the additional core Clippy probe's 37 existing JIT failures. This was host-interpreter execution, not an AWS Darwin-guest run.

The runtime source merge preserves both public parents without overlapping changed paths. Initializer-aware exit status, evaluated security reports, callback ownership, shared interpreter budgets and cleanup-before-unmap ordering remain intact. Main remains unchanged. All seven `c6b4067` full-CI jobs passed at **20:23:13 UTC**; both required repository-hygiene runs failed with two history findings. The [additive CI receipt](evidence/interpreted-printf-ci-2026-11-14.json) retains those final component results without rewriting the earlier pending checkpoint. Exact combined-feature-head checks remain required. These results do not qualify selected-guest linking/runtime execution or an enforced backend. Remaining admission integration is tracked in [issue #6](https://github.com/burdettadam/burdettie-os/issues/6); generic host ABI translation remains in [issue #9](https://github.com/burdettadam/burdettie-os/issues/9).

The [backend allocation](../contracts/BACKEND_ALLOCATION.md) assigns 17 control families to concrete candidate mechanisms and qualification tests. All start unavailable/unsupported with no qualification evidence. The contract validator now binds evidence to the specific capability, evidence kind and scopes as well as platform context. Neither that consistency check nor this bootstrap promotes a security capability to enforced.

## Remaining gates

1. Use the recovered native success only after a fresh census and the downstream consumer's current source/output checks pass. Assess and realize each remaining cross-toolchain/compiler-rt/libSystem input separately within the unchanged host bounds. Preserve the prior failed tree and all existing outputs; this completed native stage does not establish later peak capacity or linked executable fixtures.
2. Build and independently boot the selected image, including its own persistence, recovery and graceful-shutdown assessment.
3. Qualify Rust standard-library/libSystem linkage and execute the x86_64 fixture through Burdettie inside that guest.
4. Resolve the concrete worker-lifetime, display/input and backend gaps before advertising the corresponding behaviors.
5. Respect all repository checks. The historical `f8c5917` [plan receipt](evidence/libtapi-plan-2026-11-14.json) keeps its `ttf-parser` advisory and history failures. Later runtime component `c6b4067` passes all seven full-CI jobs, including Security Scanning, while its required history scan still fails on two findings. The new combined platform head requires its own checks. Scanner policy, history scope and main remain unchanged.

Physical Mac ownership and a real Apple application are not M1 prerequisites. The independent OS, real-application and selectable-security goals remain unchanged, and retain their own later acceptance evidence.

Sources: [current PureDarwin main](https://github.com/PureDarwin/PureDarwin/tree/0c1990c803c13b68cad4df3f6dc34ba4306fc789), [20.5 historical release](https://github.com/PureDarwin/PureDarwin/releases/tag/20.5), [matching release runner](https://github.com/PureDarwin/PureDarwin/blob/1bbaeb2ee6aa41e6f792a0b1e2943cc295b3b290/nix/image-contents.nix), [current source lock](https://github.com/PureDarwin/PureDarwin/blob/0c1990c803c13b68cad4df3f6dc34ba4306fc789/flake.lock), and [upstream build budget](https://puredarwin.org/handbook/getting-started.html).
