# M1 selected dyld executable inspection

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

Status: **inspection implementation and synthetic tests only**. No selected
`/usr/lib/dyld` output has been built or inspected by this overlay.

## Source-derived target

The selected PureDarwin source revision is
`0163b2db225b85e3bf96fc25646ea290105ac298`. Its
`src/Libraries/dyld/CMakeLists.txt` declares `add_darwin_executable(dyld
MACOSX_VERSION_MIN 26.5)`, links the entry symbol `__dyld_start`, and passes
`-Wl,-dylinker` plus `-Wl,-dylinker_install_name,/usr/lib/dyld`. The install
phase places the executable at `usr/lib/dyld`. This is a distinct artifact from
`usr/lib/system/libdyld.dylib` inspected by the existing libSystem helper.

The overlay inspector requires a thin little-endian x86_64 `MH_DYLINKER`, one
`LC_ID_DYLINKER` naming `/usr/lib/dyld`, one macOS `LC_BUILD_VERSION`, one
x86_64 `LC_UNIXTHREAD` state with a nonzero RIP in a file-backed executable
segment, and a file-backed executable `__TEXT` segment. It rejects rpaths,
load-dylinker commands, `LC_MAIN`, unknown command layouts, malformed bounds,
unsupported segment protections and inconsistent section mappings.

## Important qualification caveat

The `LC_UNIXTHREAD` requirement and supported load-command subset are
conservative assumptions to verify against the actual linker output. The
selected CMake target establishes the `MH_DYLINKER` intent, entry symbol and
install name, but this overlay has not observed the produced Mach-O. If the
selected linker emits a different valid entry representation, update the
contract only after inspecting the exact pinned linker output and documenting
the source/toolchain basis. Do not weaken checks merely to obtain a pass.

The helper takes a reviewed JSON expectation file with exact source revision,
minimum OS and SDK version, plus externally supplied SHA-256 hashes for both
input and expectation. A pass is static evidence only. It does not verify
build provenance, NAR/store registration, signature trust, chained fixups,
symbol binding, dyld runtime behavior, guest boot, Apple application workflows
or security enforcement. No compiler, linker, loader, Nix or subprocess runs.

## Invocation

```sh
python3 scripts/inspect_puredarwin_dyld.py \
  --dyld "$VERIFIED_DYLD_EXECUTABLE" \
  --dyld-sha256 "$VERIFIED_DYLD_SHA256" \
  --expectations "$REVIEWED_EXPECTATIONS_JSON" \
  --expectations-sha256 "$REVIEWED_EXPECTATIONS_SHA256" \
  --output "$NEW_PRIVATE_RESULT_JSON"
```

Expectation schema:

```json
{
  "schema_version": 1,
  "scope": "m1_dyld_executable_expectations",
  "source_revision": "0163b2db225b85e3bf96fc25646ea290105ac298",
  "install_name": "/usr/lib/dyld",
  "minimum_os": "26.5.0",
  "sdk_version": "<independently reviewed SDK version>"
}
```

Do not put private local paths, cloud identifiers, credentials or recovery
records in the public repository. Keep raw build artifacts and receipts in the
private evidence store; publish only reviewed non-sensitive summaries.

## Evidence retention in this continuation

The inspector now shares bounded descriptor/path reads and exclusive atomic
publication with `m1_evidence.py`. It rechecks the expectation snapshot before
success, rejects non-object/boolean-schema expectation documents, and checks
string padding before as well as after install-name strings. The partial file
is retained; it is not terminal evidence. These changes do not resolve the
unobserved selected-linker entry-layout caveat above.

## Publication-time layout regressions

The parser rejects entry points within the header/load-command region, segment
file mappings larger than their VM mappings, duplicate singleton symbol/link-edit
commands, and symbol/string tables overlapping the header. It records the entry's
file offset and requires the executable `__TEXT` mapping to cover the complete
header region. Seven additional owned-byte tests exercise these gates and their
file-backed/zero-fill boundaries. Instruction semantics and runtime execution
remain unverified.
