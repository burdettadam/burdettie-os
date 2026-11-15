# Compiler-runtime archive member check

Status: implemented and locally tested. **The selected compiler-rt output has
not been built or inspected by this change. M1 remains incomplete.**

The selected [upstream compiler-rt recipe](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/compiler-rt.nix)
copies its chosen builtins archive to `lib/libcompiler_rt.a`, but makes the
`otool -h` diagnostic optional with `|| true`. Build success therefore cannot
close the [archive and compiler-target gate](M1_BOUNDED_BUILD_RECIPES.md#compiler-target-and-archive-inspection-gate).

[`inspect_puredarwin_archive.py`](../../scripts/inspect_puredarwin_archive.py)
provides a separate, mandatory-success inspection when a caller adopts it. It
is not wired into the frozen build helpers, does not realize packages and does
not authorize a later link. The source/lock, successful build receipt, installed
output/NAR checks and actual compile commands remain separate prerequisites.

## Invocation and exact scope

Use the archive path and hash from the independently verified installed output,
and the exact required symbol spellings from the reviewed consumer's link
requirements. There is deliberately no guessed, universal builtin symbol list.
A name merely appearing in the archive does not satisfy a required definition.
The expected minimum version must come from the target decision, not be changed
after inspection to obtain a pass.

```sh
python3 scripts/inspect_puredarwin_archive.py \
  --archive "$VERIFIED_COMPILER_RT_ARCHIVE" \
  --archive-sha256 "$VERIFIED_ARCHIVE_SHA256" \
  --expected-minimum-os 26.5.0 \
  --required-symbol "$REQUIRED_BUILTIN_SYMBOL" \
  --output-dir "$NEW_PRIVATE_INSPECTION_DIRECTORY"
```

Repeat `--required-symbol` for all reviewed requirements. The output parent must
already exist; the evidence directory is created exclusively. Existing outputs
are never overwritten. A failure after directory creation retains `result.json`
when possible. A failed evidence write returns failure, even after successful
inspection. Receipts include local paths and remain private.

The helper reads one bounded regular file, refuses a symlink or special input,
checks its expected SHA-256, and inspects that in-memory snapshot. It never
extracts archive members, invokes tools, executes input code or accesses the
network. GNU and BSD/Darwin regular `ar` names are supported. Thin archives,
universal/fat archives, ELF, bitcode and nested archives are rejected rather
than silently ignoring members. Every occurrence is inspected by its header
offset, including members with identical names.

For each object it checks a little-endian baseline x86_64 `MH_OBJECT`, bounds
load commands, sections, relocation ranges, symbol tables and strings, and
records the actual macOS deployment and SDK fields. A required symbol must
have a public external section definition. Undefined/common, absolute, local,
debug and private-external records cannot satisfy that requirement. Weak
section definitions are reported as weak; this does not decide which duplicate
or weak definition a later linker will select.

| Result | Meaning |
| --- | --- |
| `pass`, exit 0 | All inspected members match the requested architecture/minimum OS and all required public section definitions are present. |
| `blocked`, exit 2 | Version metadata is absent from one or more otherwise matching members, or prerequisites such as a new evidence directory are unavailable. Missing metadata is never inferred from a wrapper filename. |
| `fail`, exit 1 | Wrong versions, missing definitions, unsupported/malformed bytes, changed input hash, read failure or failed evidence retention. |

An observed SDK field of `0.0.0` stays `0.0.0`; the inspector does not invent SDK
provenance. Members without version metadata need a separate explicit review of
actual producer commands before any qualified link. There is no flag to ignore
that block. Mixed wrong and missing versions remain a failure, not a pass.

The limits are 64 MiB per archive, 16 MiB per member, 8,192 members, 65,536 symbols
across the archive, and 1,024 bytes per symbol name. These are defensive inspection
limits, not estimates of the selected build's space or memory requirements.

## What this does not prove

Archive symbol-index members are enumerated and hashed, but their ranlib index
semantics are **not validated**. Relocation semantics, all possible Mach-O load
commands, full builtin coverage and linker resolution are also not qualified.
This is a narrow member/symbol check, not a general binary validator or a claim
that the archive will link. `archive_index_semantics_checked` and
`build_provenance_verified` remain false. `selected_compiler_rt_qualified`
remains false even on a pass. Required checked tool inspections and a real
strict fixture link must still succeed.

The helper leaves fixture linking, guest execution and security enforcement at
`not_run`. It does not change the 13 frozen helpers/recipes, any security backend
record, scanner policy, dependency policy, host resource limits or failure gates.

## Validation performed on November 15, 2026

The focused suite contains 34 passing tests, with no skips. It exercises complete
and truncated archives, duplicate names, GNU/BSD names, bounds, actual versus
hidden/undefined/common/debug/local symbol definitions, wrong or absent version
metadata, malformed Mach-O tables, output preservation, symlink/FIFO refusal and
receipt-write failures. The fixtures are source-generated synthetic records.

Separate local integration used the already available **Clang 17.0.0** and
`llvm-ar` to compile a header-free, source-owned C function into actual Mach-O
objects and Darwin-format archives. `llvm-objdump` independently displayed their
headers and symbols. These are not the selected LLVM 21.1.8 or AWS build outputs.

| Local integration case | Checker result |
| --- | --- |
| x86_64, macOS 26.5, public definition | Pass, exit 0 |
| ARM64 object | Fail, exit 1 |
| x86_64, macOS 11.3 | Fail, exit 1 |
| x86_64, macOS 26.5, hidden definition | Fail, exit 1 |
| macOS 26.5 target followed by `x86_64-apple-darwin20.4` target | Fail, exit 1; actual local object records minimum OS 11.0.0 |

The last case reproduces a target-precedence risk in this local compiler. It is
not a finding about the still-unexecuted selected compiler-rt build. No AWS
command, dependency build, selected guest or Apple application was run for these
checks. Full-repository CI has its own commit-specific result.

Run the focused tests with:

```sh
python3 -m unittest discover -s scripts/tests \
  -p 'test_inspect_puredarwin_archive.py' -v
```

The existing M1 workflow includes these tests through its existing discovery
command. This change adds only the three new paths to each push/PR filter; it
adds no runner, install step, permission, exception or skipped test.

Format references: [LLVM archive tool](https://llvm.org/docs/CommandGuide/llvm-ar.html)
and [LLVM Mach-O definitions](https://github.com/llvm/llvm-project/blob/llvmorg-21.1.8/llvm/include/llvm/BinaryFormat/MachO.h).
