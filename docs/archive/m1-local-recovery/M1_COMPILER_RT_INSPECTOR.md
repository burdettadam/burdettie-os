# Compiler-rt archive inspection and link-evidence checks

Status: implemented and locally tested with owned inputs. **No selected
PureDarwin compiler-rt archive, libSystem output, or guest has been qualified.**
This additive overlay does not replace the published
`inspect_puredarwin_archive.py` or modify that helper's frozen hashes. Review
possible consolidation with that existing parser before adopting another
production qualification path.

## Archive inspection

`inspect_puredarwin_compiler_rt.py` checks a bounded ordinary ar snapshot,
without extracting or executing it. It supports GNU indexes/name tables and
BSD extended names, checks all member headers and padding, rejects duplicate
or unsafe ordinary names, and records offsets, sizes, modes and per-member
SHA-256. Archive indexes are identified but **not semantically validated**.

Each ordinary member must be little-endian x86_64 `MH_OBJECT`. The implemented
subset bounds load commands, object sections, relocations' byte ranges, and
symbol/string tables. External concrete definitions, undefined references,
common/tentative symbols, aliases and private-external names are distinguished.
`N_SECT` definitions require a valid section ordinal/value. A missing symbol
table is explicit, never evidence of no symbols. Unknown uninterpreted load
commands are reported; complete object/fixup/relocation semantics are not
qualified by this helper.

Symbol classification follows the selected
[nlist.h](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/src/Kernel/xnu/EXTERNAL_HEADERS/mach-o/nlist.h).
Common and indirect symbols are not silently counted as concrete definitions.
Neither an architecture field nor the historical wrapper triple establishes a
selected compiler, deployment target or SDK. Observed build-version commands
are retained separately.

The expectation document remains schema 1:

```json
{
  "schema_version": 1,
  "scope": "m1_compiler_rt_expectations",
  "source_revision": "0163b2db225b85e3bf96fc25646ea290105ac298",
  "target_triple": "x86_64-apple-darwin20.4",
  "archive_sha256": "<independently reviewed digest>",
  "member_names": ["<complete independently reviewed ordinary member list>"]
}
```

Placeholders are invalid. Unknown/duplicate JSON fields, non-integer schema
versions, wrong hashes and differing complete member sets fail.

```sh
python3 scripts/inspect_puredarwin_compiler_rt.py \
  --archive "$VERIFIED_ARCHIVE" \
  --expectations "$REVIEWED_EXPECTATIONS" \
  --expectations-sha256 "$REVIEWED_EXPECTATIONS_SHA256" \
  --output-dir "$NEW_PRIVATE_DIRECTORY"
```

**Receipts now use schema 2.** Regenerate old schema-1 inspection receipts from
verified bytes; the new link verifier refuses them. The receipt includes the
complete freshly parsed member inventory and explicit unqualified fields.

Bounds: 512 MiB per archive, 64 MiB per stored member, 100,000 total ar headers,
512 KiB per expectation, 4 MiB/4,096 load commands per object, 255 sections,
262,144 symbols and 8 MiB of strings per object, 32 MiB per consumed inspection
receipt. These parser bounds do not qualify build capacity or memory peaks.

## Link evidence

`verify_compiler_rt_link.py` consumes a v2 direct-linker capture, its map, the
archive and its v2 inspection receipt, and the linked output bytes. Each is
bound by a separately supplied SHA-256. It executes nothing and freshly
reparses the archive instead of trusting a fabricated member list in a receipt.
See [the capture contract](M1_LOCAL_EVIDENCE_CAPTURE.md).

The exact archive must be one ordinary argument. A compiler-rt path consumed
as an option operand is not an ordinary argument. Global whole-archive modes,
compiler-rt force-loading, response files, file lists, unexpanded compiler
arguments, blanket undefined-symbol suppression and unknown direct-linker
options fail. **Force-loading unrelated archives is permitted:** the selected
libSystem recipe deliberately does so. The old overlay's blanket prohibition
on all `-force_load` options was inconsistent with that recipe.

The map's original output path, architecture and ordered tables are checked.
Only the `# Object files:` table supplies member selections. Each selected
archive path is resolved lexically against the captured working directory and
must identify the exact captured archive, not a different file with the same
basename. Selection names must exist in the freshly parsed inventory. Duplicate
ordinals/names, malformed rows, unknown symbol ordinals and incomplete maps fail.
Live/dead map symbol lists remain separate. Map membership is not proof that
all of a member's code survives dead stripping or that imports resolve.

Known availability-check member names and definitions/references are checked,
including a renamed member exposing the same known availability symbols. This
is not a complete semantic equivalence detector.

The supported textual format is based on LLVM's pinned
[Mach-O map writer](https://github.com/llvm/llvm-project/blob/llvmorg-17.0.0/lld/MachO/MapFile.cpp).
Compatibility with the **selected native ld64 writer** still requires its real
output. No real successful linker map was obtained in this environment.

## Retention and limitations

`m1_evidence.py` supplies duplicate-key-safe JSON, nonblocking/no-final-symlink
reads and descriptor/path drift checks. Success is published exclusively only
after writing, flushing, syncing and closing a partial file. The `.partial`
file stays as nonterminal evidence. No fallible cleanup occurs after the atomic
link publication. A post-publication directory fsync/power-loss durability
claim is deliberately not made. Existing evidence is never overwritten.

Publication failure returns nonzero and reports `fail`, not a contradictory
`pass`. A writable hostile same-user environment is outside this check's trust
boundary; verify source and host tools before executing Python.

All receipts leave build provenance, selected libSystem qualification, symbol
resolution, guest execution and security enforcement unqualified. Hash-pinning
an externally supplied capture does not authenticate its claimed process exit
or prove which source/toolchain produced an artifact. An independently trusted
producer, Nix registration/NAR verification and actual selected-target tests
remain necessary.
