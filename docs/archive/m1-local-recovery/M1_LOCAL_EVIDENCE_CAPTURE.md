# Retained linker capture and local toolchain probe

## Capture schema 2

The capture must describe the actual expanded **direct ld64 argument vector**,
not an unexecuted plan or a shell command. The successful producer records its
real exit status, timeout state, original working directory, and original
archive/map/output paths and bytes. Store the capture privately and independently
review its digest before consumption.

```json
{
  "schema_version": 2,
  "scope": "m1_compiler_rt_link_capture",
  "source_revision": "0163b2db225b85e3bf96fc25646ea290105ac298",
  "input_origin": "selected_source_build",
  "argv_kind": "ld64_direct",
  "argv": ["<absolute reviewed linker>", "<actual expanded arguments>"],
  "cwd": "<absolute original working directory>",
  "exit_code": 0,
  "timed_out": false,
  "archive": {"path": "<original absolute archive>", "sha256": "<digest>"},
  "link_map": {"path": "<original absolute map>", "sha256": "<digest>"},
  "output": {"path": "<original absolute output>", "sha256": "<digest>"}
}
```

The placeholders are invalid. For source-owned experiments use
`input_origin: "owned_fixture"`; never relabel them as selected-source output.
Original paths are lexically normalized and hashes must match externally
selected values. Recovered local copies can be stored at different paths;
command/map identity comparison uses the original capture paths, not the
recovery machine's filesystem aliases.

```sh
python3 scripts/verify_compiler_rt_link.py \
  --argv-json "$CAPTURE" --argv-sha256 "$CAPTURE_SHA256" \
  --map "$MAP" --map-sha256 "$MAP_SHA256" \
  --archive "$ARCHIVE" --archive-sha256 "$ARCHIVE_SHA256" \
  --archive-inspection "$ARCHIVE_RECEIPT" \
  --archive-inspection-sha256 "$ARCHIVE_RECEIPT_SHA256" \
  --linked-output "$LINKED_OUTPUT" --linked-output-sha256 "$LINKED_OUTPUT_SHA256" \
  --output "$NEW_PRIVATE_RECEIPT"
```

Version 1 captures are deliberately unsupported. Obtain expanded direct argv
from the actual build and retain all source/SDK/toolchain/Nix provenance
separately. Do not manufacture this JSON from an assumed command or replace a
failed process result with zero. The verifier checks consistency, not the
honesty of the capture mechanism. Its `capture_execution_authenticated` and
`build_provenance_verified` fields remain false.

## Owned local toolchain probe

`probe_m1_local_toolchain.py` compiles three tiny C sources without SDK headers,
archives two Mach-O objects and attempts a dylib link. On success it exercises
the archive and captured-link verifiers together. On failure it retains the
real exit status and stdout/stderr and returns nonzero. It does not silently
skip a failed linker, strip platform commands, lower the target version,
install software, download a toolchain or retry using another linker.

```sh
python3 scripts/probe_m1_local_toolchain.py \
  --clang /absolute/path/to/already-trusted/clang \
  --ar /absolute/path/to/already-trusted/llvm-ar \
  --linker /absolute/path/to/already-trusted/ld64.lld \
  --output-dir /existing/private-parent/new-owned-probe
```

Tool paths may be omitted to discover installed executables through PATH.
The invocation basename is preserved for multicall symlinks, while the
resolved executable's SHA-256 and size are recorded. Sources, command vectors,
phase exit codes, timeout flags, output logs, code hashes and receipts are
retained. Each command has a 15-second deadline and a sampled 2-MiB-per-stream
log threshold. Oversized logs remain failures; polling is not an adversarial
hard quota. Timeout cleanup signals only the process group this probe created.
Detached hostile descendants are outside this trusted-tool smoke test, which
is **not** a replacement for the selected shared-host service guard.

The test-generated expectation is explicitly marked as owned-smoke data, not
independent provenance review. Even a complete smoke pass never qualifies
selected compiler-rt, libSystem, dyld, a guest image, persistence or security.

## Observed continuation result

In this continuation the installed Clang and llvm-ar produced the three
objects and a two-member Darwin archive; archive inspection passed. The
installed Swift-distribution `ld64.lld` returned exit 1 with:

```text
ld64.lld: error: This version of lld does not support linking for platform macOS
```

The attempt is a **failed link**, not a successful Darwin build. Exact logs and
tool/code hashes are in the separate handoff evidence directory, not embedded
as a selected-build receipt. Native Darwin execution, selected image boot and
all previously open qualification gates remain untested.
