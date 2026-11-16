# Observed compiler-rt linker capture

`capture_m1_link.py` fills the producer gap between the static archive inspector
and `verify_compiler_rt_link.py`: it observes a real direct linker invocation,
retains its exit and bounded logs, then runs the existing member-selection check.
A manually prepared capture can still be checked by the static verifier, but is
not interchangeable with this operation's observed subprocess evidence.

**M1 is incomplete.** This helper does not build selected compiler-rt, qualify
libSystem, execute a Darwin binary, boot an image, or establish a sandbox. It does
not automatically instrument or modify the frozen Nix/CMake/toolchain recipes.
Integration with those actual selected builds remains a separately reviewed step.

## Reviewed request

The CLI takes a JSON request and its independently selected SHA-256. The request
must contain exactly these fields (placeholder hashes below are not valid):

```json
{
  "schema_version": 1,
  "scope": "m1_direct_link_request",
  "source_revision": "0163b2db225b85e3bf96fc25646ea290105ac298",
  "input_origin": "selected_source_build",
  "argv": [
    "/verified/tools/ld", "-dylib", "-arch", "x86_64",
    "-platform_version", "macos", "26.5", "26.5",
    "/verified/main.o", "/verified/libcompiler_rt.a",
    "-o", "/private/new-link/linked.dylib",
    "-map", "/private/new-link/link.map"
  ],
  "cwd": "/verified/build",
  "linker_sha256": "<reviewed executable digest>",
  "archive_path": "/verified/libcompiler_rt.a",
  "archive_inspection": {
    "path": "/verified/archive-inspection.json",
    "sha256": "<reviewed receipt digest>"
  },
  "inputs": {
    "/verified/main.o": "<reviewed object digest>",
    "/verified/libcompiler_rt.a": "<reviewed archive digest>"
  },
  "timeout_seconds": 15
}
```

Use the SDK version recorded by the real selected input review, not an inferred
value from the deployment target. `input_origin` may also be `owned_fixture` for
test inputs. Neither label proves provenance. The scope intentionally supports
only a subset of expanded direct ld64 dylib commands; a compiler driver, response
file, implicit `-l` library, search directory, or unsupported option fails closed.
A required command outside that subset needs additional reviewed implementation,
not a rewritten command or an unrecorded fallback.

All explicit object/archive/dylib inputs and file-valued option operands must be
in `inputs` exactly once by canonical path. This includes symbol/order lists,
unrelated force-loaded archives, and explicit `-dylib_file` mappings. The normal
compiler-rt path must occur exactly once as a positional input. Existing verifier
rules reject force-loading compiler-rt and blanket undefined-symbol suppression;
individual `-U` entries remain visible in the exact reviewed argv. The selected
PureDarwin libSystem source uses named Objective-C dynamic-lookup entries, so
recording `-U` does not prove those references were resolved.

```sh
python3 scripts/capture_m1_link.py \
  --request "$REVIEWED_LINK_REQUEST" \
  --request-sha256 "$REVIEWED_LINK_REQUEST_SHA256" \
  --output-dir "$NEW_PRIVATE_LINK_DIRECTORY"
```

The output directory must be new, outside the Nix store, with an existing
canonical parent. The command must direct its two outputs to `linked.dylib` and
`link.map` immediately inside that directory. Existing files are not overwritten.
Snapshot files and process logs can expose source bytes and local paths: retain
selected-build evidence privately, not as a public CI artifact.

## Observed operation

Before launch the helper checks the exact request, executable hash, explicit
input hashes, and a successful v2 archive inspection against freshly parsed
archive bytes. It snapshots the request, archive receipt, and every explicit
input, and fingerprints its own helper modules. It preserves the linker's invoked
basename (important for multicall `ld64.lld` symlinks) and records the resolved
executable identity. The reviewed installed executable and its shared libraries
are trusted prerequisites; no tool is downloaded or installed.

One subprocess executes the recorded argv without a shell, inherited loader
configuration, environment flags, stdin, or automatic retry. Logs are captured
through pipes with 2 MiB per-stream retention limits. The timeout must be an
integer from 1 through 30 seconds; timeout, signal exit and log overflow fail.
The child inherits no looser limits than its caller: core output is disabled,
CPU time is capped at 30 seconds, and individual output files at 256 MiB.
Input limits are 64 MiB per explicit file, 256 MiB in aggregate, and 1,024 paths.
These are operational caps, not shared-host capacity or peak-memory proof.

Cleanup targets the operation's own process group, including ordinary descendants
when its leader exits. Detached/untrusted descendants require a qualified containing
service. The CLI handles SIGTERM and retains failure when publication remains
possible; callers of the Python API own their signal policy. Do not use this
helper as a substitute for the existing bounded AWS operation-service gates.

After zero exit, the helper repeats input, request, tool, receipt and helper
hash checks, reads actual map/output bytes, and publishes the compatible v2
capture. It then runs the static member-selection verifier, repeats drift checks,
and publishes the verification plus terminal operation receipt. A failed
selection can retain a successful *link-command* capture but never an overall
successful operation. Nonzero execution cannot produce a successful capture.
Evidence publication preserves partial files and never overwrites terminal files.

A successful result is `observed_link_and_selection_checked`; any failure exits
nonzero. Errors before creating a permitted output directory are returned on stdout
without writing there. Later failures retain `result.json` when possible; a
terminal-publication failure itself returns failure.

This is trusted local observation, not a cryptographically authenticated producer.
Receipts keep build provenance, transitive-input verification, shared-host service
qualification, selected-library qualification, guest execution and security
qualification unset/false. Pre/post hashing detects ordinary drift; it is not a
same-UID hostile-writer or hostile-host defense. No output-format, relocation,
signature, ABI, or symbol-resolution qualification is inferred from a link map.

## Real positive and negative integration test

The existing M1 job invokes `probe_m1_captured_link.py` after its original owned
compile/archive/link probe. Using the same already installed LLVM tools, it makes
a second real dylib link through this producer and requires `mul.o` to be selected.
It then compiles an owned object referring to `_owned_missing_for_negative_link`
and observes a deliberate failed link. The negative assertion requires a positive
nonzero linker exit, no timeout/truncation, the undefined-symbol diagnostic,
no success capture, and the exact retained terminal failure. Unavailable tools,
unsupported platforms, signals, missing output or arbitrary errors cannot satisfy
that assertion. The negative link remains `fail`; only the test of rejection passes.

Generated fixture requests are explicitly **not independently reviewed selected
build evidence**. The existing artifact upload retains the nested `captured-links`
evidence, including failures. No new CI job, runner, permission, tool download,
resource allocation, scanner exception or broader success claim is introduced.

The strict suite includes child-process, mutation, output-preservation, malformed
request/receipt, pipe-limit, timeout, signal and negative-assertion regressions:

```sh
python3 scripts/run_m1_overlay_tests.py
```
