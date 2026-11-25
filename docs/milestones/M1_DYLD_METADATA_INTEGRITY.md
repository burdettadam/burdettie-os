# M1 dyld metadata integrity

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

This increment closes false-positive structural checks in
`scripts/inspect_puredarwin_dyld.py`. It does not build or execute dyld, qualify
source provenance, or establish guest boot or security enforcement.

## Reproduced defects

The existing inspector bounded dynamic-table ends against the file length, but
allowed nonempty tables to begin inside the Mach-O header or load-command
region. Its original symbol and string tables could overlap one another, and
both could alias dynamic tables or other checked link-edit payloads. A dynamic
symbol command could also omit the original symbol command or identify groups
outside that original table or overlapping each other.

The pre-change script's Git blob is
`34693c2ab1d6c321d699a49656cd2ea5509d41a7`. That exact blob was independently
read back from public head `8a2d21c64546d01d562fd8d969fbeba49b35d73b` before
editing. Source-owned regression fixtures reproduce the invalid acceptances;
these are malformed synthetic inputs, not observations of a selected binary.

## Correction

Nonempty ranges for all six dynamic-table offset/count pairs must begin after
the complete declared command region and end within the input. Their sizes
remain TOC 8, 64-bit module 56, external reference 4, indirect symbol 4, and
external/local relocation 8 bytes per entry. Symbol and string ranges now join
these tables and the already checked link-edit payloads in one disjoint-range
check. Half-open ranges may touch; empty ranges do not create false overlaps.
The range comparison sorts numeric bounds and does not parse payload contents.

Every `LC_DYSYMTAB` requires `LC_SYMTAB`, regardless of command order. Local,
externally defined, and undefined symbol groups must fit within `nsyms`, and
nonempty groups cannot overlap. Python integer arithmetic avoids unsigned
addition wrapping when validating the declared indices and counts. The helper
does not require these groups to exhaust the symbol table.

The public result schema is unchanged. Existing input hashes, expectation
checks, exclusive publication, architecture/entry checks and unqualified
runtime/provenance fields remain intact. Malformed metadata returns nonzero
and cannot publish a successful result or partial successful result.

## Tests and integration

Twenty added regression methods are part of the **existing**
`test_dyld_layout_regressions.py` group, so both the public strict runner and
normal unittest discovery include them without a workflow or runner change.
They cover all six table widths, header/command aliases, end overruns, exact
end boundaries, symbol/string and cross-command aliases, missing companion
commands, group overruns/overlaps, empty ranges, reversed command order, and
CLI refusal to publish success. The prior seven layout and thirteen inspector
tests remain in place.

Run the focused checks with:

```sh
python3 -m unittest discover -s scripts/tests -p test_dyld_layout_regressions.py -v
python3 -m unittest discover -s scripts/tests -p test_inspect_puredarwin_dyld.py -v
```

The retained before/after logs are in the separate recovery package. They are
not a substitute for current-head full-repository CI, selected-library evidence
or independent review. The package also preserves baseline timing failures and
an interrupted full-suite attempt; those results must not be relabeled passes.

## Format references and limits

Apple's [Mach-O loader definitions](https://github.com/apple-oss-distributions/xnu/blob/main/EXTERNAL_HEADERS/mach-o/loader.h),
under `symtab_command`, `dysymtab_command`, `dylib_module_64`, and
`dylib_reference`, define these original/dynamic table relationships, indices,
and entry layouts. The header was consulted on November 17, 2026; its mutable
main reference is not selected-source build provenance.

This remains range-level inspection. It does **not** validate individual nlist
names/types, TOC/module references, relocation semantics, indirect-symbol entry
values, code signatures, chained fixups, binding resolution or mapping of all
metadata into a selected `__LINKEDIT` segment. LC_NOTE contents and aliases are
also outside this increment. No architecture/target downgrade, scanner change,
history rewrite, new CI job, AWS action or guest-success claim is introduced.
