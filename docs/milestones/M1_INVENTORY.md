# M1 application and hardware inventory

The initial M1 reference platform is the Linux-hosted x86_64 Darwin VM. This collector supports later application and physical-hardware qualification; a Mac inventory is not required to close M1. An existing `.app` can be inspected on Linux or Windows with `--no-hardware`, without running it or invoking Apple's `codesign` tool.

Run this passive collector on the intended Mac. It does not launch the selected
application, use a network service, request administrator privileges, copy
application code, or change system configuration. Python 3.9 or later is needed.

```sh
python3 scripts/collect_m1_inventory.py --host-role physical-target --app /System/Applications/Stickies.app > m1-inventory.json
```

If Stickies is elsewhere, pass the installed bundle's actual location. Omit
`--app` for hardware only. Use `--no-hardware --app PATH` to inspect an explicitly
selected bundle without hardware collection; the Mach-O parser also works on
non-Mac hosts. Do not commit a real machine's output automatically; inspect the
report and supply it as the target record. Generated Apple app binaries and
resources remain outside the public repository.

If the Mac supplies the app but is not the physical OS target, set `--host-role
source-oracle` and collect the intended older Mac separately. The observed macOS
version describes the machine running the collector; it does not prove the
original OS source of a previously copied bundle. Record extraction provenance
separately. The collector leaves this distinction explicit in its JSON.

The report captures exact `CFBundleShortVersionString` and `CFBundleVersion`,
main-executable architecture slices, direct dylib load commands, minimum OS/SDK
load commands, and separate SHA-256 hashes of the main executable and Info.plist.
Those two hashes are not a whole-bundle identity. System-profiler data is reduced
to allowlisted hardware fields; serials, UUIDs, network addresses, SSIDs and raw
personal device names are omitted. Selected app paths are not emitted. Absolute
non-system dependency paths are redacted and classified as unresolved.

`codesign --display` contributes display metadata only on macOS. The report
explicitly records that signature verification and trust assessment were not
performed. A signature load command, CDHash or SHA-256 is not proof of a valid
signature, signer, entitlement grant, or sandbox. The collector never runs the
app to discover its runtime-loaded libraries.

## What remains to qualify a target

Supply or verify missing storage/controller and network fields for the exact
Mac model; system-profiler keys differ between OS/hardware versions. Confirm the
actual firmware/boot route and whether devices work under the candidate OS.
Record the app's transitive dependency graph, embedded executable/resource
manifest, runtime dynamic loads and expected workflow separately. No local
inventory result itself qualifies that hardware or app as supported.

## Regression checks

```sh
python3 -m unittest discover -s scripts/tests -p 'test_collect_m1_inventory.py' -v
```

The tests use synthetic Mach-O/Info.plist data and mocked macOS tools. They cover
thin and universal headers, architecture consistency, load-command bounds,
metadata extraction, path redaction, bundle escape rejection, and separation of
signature display from verification. Run the collector on a real Mac before
claiming its hardware fields are complete for that machine.
