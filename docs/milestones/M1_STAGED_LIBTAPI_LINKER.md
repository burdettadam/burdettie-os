# Planning the linker with a validated staged libtapi

> Scope update, 2026-11-25: this component/reference record retains its original source and evidence scope. Current product work follows [ADR 0003](../adr/0003-linux-foundation-and-upstream-reuse.md) and the [Linux M1 plan](M1_PLATFORM_SECURITY.md). PureDarwin build/guest instructions below are optional reference or upstream-qualification work, not shipping prerequisites or the default next task. Existing receipts do not qualify the new Linux target.

[`plan_puredarwin_staged_linker.py`](../../scripts/plan_puredarwin_staged_linker.py)
is a separate opt-in follow-up to the
[libtapi source-staging recipe](M1_LIBTAPI_SOURCE_STAGING.md). It requires an
actual successful libtapi build, its exact inspected plan and fresh checks of
the realized output. A pending or failed build receipt is rejected even if an
output directory happens to exist.

The validated libtapi is an x86_64 Linux host dependency of Apple's cctools
linker. The selected Darwin guest, its libSystem/dyld, executable fixtures and
Burdettie runtime/security qualification have their own remaining gates.

## Narrow package rebind

The pinned
[`native-ld.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/nix/pkgs/toolchain/native-ld.nix)
accepts `libtapi` as a package argument. On the selected Linux host that argument
appears once in `buildInputs` and twice in the configure phase: the include
directory and the `libtapi.so` path.

The new recipe reconstructs the staged libtapi from the unchanged reviewed
recipe and stager. Its derivation and output must exactly match the successful
plan. It then calls `native-ld.override { libtapi = stagedTapi; }` and applies the
existing one-job Ninja patch to that rebound package.

The recipe asserts the following relationships:

| Field | Allowed difference |
| --- | --- |
| Native-linker build inputs | Exactly one libtapi output changes; order and other inputs remain equal. |
| Configure phase | Exactly two original libtapi path occurrences become the staged output path. Every other byte remains equal. |
| Build phase | The original single `ninja` command gains `-j1`. |
| Source, native build inputs, bootstrap toolchain and install phase | No change. |
| SDK derivation/output | Same identity as the original linker and successful libtapi plan. |

Keeping the bootstrap toolchain matters: the pinned
[`flake.nix`](https://github.com/PureDarwin/PureDarwin/blob/0163b2db225b85e3bf96fc25646ea290105ac298/flake.nix)
uses the bootstrap toolchain to build cctools, then uses cctools in the final
cross-toolchain wrappers. The explicit libtapi argument override preserves that
ordering. It does not replace the native linker's compiler/toolchain argument.

The report distinguishes the original linker, the earlier one-job linker,
the unbounded package with the new libtapi, and the new bounded candidate.
Existing linker, library and libtapi-staging helpers retain their default
identities and behavior.

## Required retained evidence

### Observed plan on 2026-11-14

After the staged libtapi build succeeded, this helper ran at Burdettie
`e5ff8bf1fc332b6fc0c08dd904a38e8de34f8af4`. Its metadata phase passed in
**32.21 seconds**, finishing at **20:18:42 UTC**. The [actual plan
receipt](evidence/staged-linker-plan-2026-11-14.json) binds that earlier helper
revision, the successful `f8c5917` libtapi build and the unchanged SDK.

Fresh complete-source inventory, installed-file, registered NAR and default
offline store checks passed. The dry-run listed **nine local derivations and
zero fetched paths**, without compiling or realizing the native linker. Final
free disk space was **9,289,572,352 bytes**, above the retained 8-GiB reserve.
The complete result, dry-run and selected live-check logs were recovered and
verified. This metadata result does not establish build peak capacity or a
working native linker. The [shared output inspector](M1_STAGED_LIBRARIES.md)
defines the separate live registration, dynamic-binding and smoke checks that
must follow a successful realization.

The later native build passed in **459.99 seconds** and its installed-output
inspection passed in **14.29 seconds**, finishing at **21:08:52 UTC**. The
[separate recovered-evidence receipt](evidence/staged-native-linker-recovered-2026-11-14.json)
binds all 32 retrieved result/log/inspection files and their independent review.
The exact registered NAR, complete native-output inventory, default store check,
static ELF binding, observed staged-TAPI loader resolution and bounded version
probe passed. The version banner retains the literal `LD64_VERSION_NUM` token;
no numeric ld64 version is claimed. The earlier [terminal
observation](evidence/staged-native-linker-terminal-observation-2026-11-14.json)
preserves the historical transfer interruption. Neither this completed host
stage nor its sampled capacity result supplies a downstream or guest result;
the next consumer must freshly verify its inputs. The earlier metadata plan
remains `planned_not_built` within its original scope.

### Inputs for a plan

Keep these private files together with the original build evidence:

- The libtapi plan's `result.json` and its reviewed SHA-256.
- The successful libtapi build's `result.json` and its reviewed SHA-256.
- That build's adjacent `output-path-info.json`, whose checksum is bound by the
  build receipt.

The successful build receipt must report `staged_libtapi_built`, a successful
realization phase, the observed unchanged-source staging inventory and a passed
output inspection. The plan and build must agree on source, SDK, package
identities, source inventory, source revision and original helper/code hashes.

The output inspection includes:

```json
{
  "registered_output": {
    "path": "/nix/store/…-apple-libtapi-1300.6.5",
    "deriver": "/nix/store/…-apple-libtapi-1300.6.5.drv",
    "nar_hash": "sha256-…",
    "nar_size": 1,
    "path_info_sha256": "…"
  }
}
```

These values are schema examples. Actual values must come from the successful
build's Nix output metadata. The deriver must match the planned candidate, the
NAR hash must be a canonical SHA256 SRI value, and the size must be a positive
integer. The same inspection records the complete file/hash and symlink
inventories, native ELF library, regular public headers and store references.

The helper reads checksummed JSON as data. It evaluates only the locally
reviewed recipe and stager snapshots, whose hashes must match the prior plan;
it does not execute recipe text taken from a receipt.

The new helper may run at a later Burdettie revision. Its report records that
current revision separately from `successful_libtapi.burdettie_revision`, the
shared revision of the original plan and successful build. The unchanged recipe,
stager and helper hashes, exact package identities and source/SDK checks still
bind the earlier evidence across that explicit revision transition.

## Fresh checks before the new plan

Use a new private output directory with an existing parent:

```sh
python3 scripts/plan_puredarwin_staged_linker.py \
  --puredarwin-source "$PUREDARWIN_SOURCE" \
  --libtapi-plan "$LIBTAPI_PLAN" \
  --libtapi-plan-sha256 "$LIBTAPI_PLAN_SHA256" \
  --libtapi-build-result "$LIBTAPI_BUILD_RESULT" \
  --libtapi-build-result-sha256 "$LIBTAPI_BUILD_RESULT_SHA256" \
  --expected-sdk-output "$SDK_OUTPUT" \
  --expected-sdk-derivation "$SDK_DERIVATION" \
  --output-dir "$NEW_LINKER_PLAN_DIRECTORY" \
  --query-substituters
```

After validating the receipts and re-evaluating the pinned packages, the helper
rehashes the complete retained libtapi source. It queries current Nix metadata
for both libtapi and the SDK, requires the expected registered derivers, and
compares libtapi's current NAR hash and size with the successful receipt. The
Nix 2.28
[`path-info` implementation](https://github.com/NixOS/nix/blob/2.28.3/src/nix/path-info.cc)
includes the extended registered-output metadata used by these checks.

It then runs `nix store verify --offline` for both outputs with the default
content and trust checks. It does not repair paths or waive either check. The
installed libtapi file hashes and link texts must still match the successful
build receipt; headers must remain ordinary files, output links must remain
inside the package, and the native libtapi library must remain x86_64 ELF.
Unexpected files, changed source bytes, changed references or failed integrity
checks stop the plan. A failed live check records a failure and retains its
stdout/stderr.

Only after these gates does the helper run a native-linker dry-run. Evaluation
uses offline, lock-preserving, no-import-from-derivation options. By default the
dry-run is also offline; `--query-substituters` explicitly permits binary-cache
metadata queries. The helper reports separate bounded realization arguments
but never executes them. It contains no store-registration operation.

The new plan records all evidence checksums, recipe snapshots, package
identities, current NAR metadata, source inventory and read-check logs. Keep
these private records out of the public repository.

## Qualification and tests

The dry-run remains a plan. Peak space is unqualified until assessed against a
fresh host census and monitored stage budget. Normal Nix realization of the new
linker is a separate reviewed action on the existing host, with the same CPU,
memory, task, job and disk-reserve constraints. The helper does not delete old
failed trees or change existing source/build outputs.

Focused checks run with:

```sh
NIX_BIN=/path/to/nix python3 -m unittest discover \
  -s scripts/tests -p 'test_plan_puredarwin_staged_linker.py' -v
```

The graph tests use a real Nix evaluator with synthetic package records. They
check input replacement, distinct identities, the one-job patch and rejection
of unrelated source/toolchain/configuration changes. Receipt tests use explicitly
synthetic evidence and mocked Nix metadata responses to check pending/failed
build rejection, checksum and identity mismatches, failed content verification,
changed files, escaping links and failed dry-runs. Those fixture tests do not
establish a successful real libtapi or native-linker build. Actual successful
evidence remains mandatory before this helper can be used for its next plan.
