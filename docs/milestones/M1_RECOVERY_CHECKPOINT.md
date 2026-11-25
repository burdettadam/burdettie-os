# M1 recovery checkpoint — November 16, 2026 (America/Denver)

## Recovery scope

The available conversation artifacts were reconciled against public PR #3 at
`4073ffcf59852c1ed92d94df53d917f92d227933`, tree
`850b7c568bc4c42b2793edead70f4444defec700`. Main was independently observed at
`7bf375562ca9edf513c353da7ec3309c0337f457`; the earlier PR #1 recovery is merged
there. This checkpoint adds to the existing M1 branch, not a replacement history.

Eleven retained ZIPs were inspected, including 504 matching file checksums in
six embedded manifests, with no missing or mismatched manifest entries. All
eight distinct Git bundles verify, and every advertised tip is an ancestor of
the latest local overlay tip, `d3aca41ea49c59d4ba4fe4f046b461f1af1504ed`.
Its 20 commits remain preserved in the original recovery bundle.

The latest overlay has 45 tracked paths. The
[machine-readable reconciliation](evidence/m1-recovery-20261116.json) records
their local and published Git blob identities and dispositions:

| Disposition | Paths | Action |
| --- | ---: | --- |
| Already published, byte-identical | 35 | Keep the public source without duplicate commits |
| Divergent test runner | 1 | Retain every public group and add the recovered thread-state group |
| Historical guides absent from public source | 3 | Restore exact bytes under the explicitly historical archive |
| Newer ABI guide already published | 1 | Keep the public guide and its native-fixture integration section |
| Overlay-only root metadata | 5 | Preserve in the recovery bundle; do not overwrite repository metadata |

## Reconciled runner

The local runner included `test_m1_thread_state_probe.py` but lacked the newer
public `test_build_native_fixtures.py` group. Replacing either runner wholesale
would lose coverage. The only executable change is one additional group in
`scripts/run_m1_overlay_tests.py`: both are now included, for 12 unique groups.
The existing nonzero result for skipped or undiscovered groups is unchanged.
No test assertion, timeout, CI job, runner configuration, or scanner setting is
changed by this recovery.

The recovered local runner actually completed **177 tests, zero failures and
zero skips**. This does not claim that the newly combined runner or the public
native-builder group ran locally. The combined runner must pass in the existing
M1 CI environment; current-head CI results belong in the PR's run records.
The preceding `4073ffc` seven-job CI and M1 successes are historical validation,
not a substitute for checking this recovery commit.

## Continue from the public branch

Use `milestone/m1-platform-security-contracts` in the public repository. Do not
apply the pthread patch again: `4073ffc` already contains the implementation,
seven production-shim regressions, Docker integration and its guide. Do not push
the bundled overlay onto a public branch: it is a partial tree with a separate
root history and contains historical generated files. The public README,
ignore rules, existing source, and newer documentation must remain intact.

The [three recovered guides](../archive/m1-local-recovery/README.md) are archival.
Their original status statements remain unchanged and are not current pass/fail
claims. The original archives and auxiliary evidence scripts are retained in
the private recovery package, not copied into production source or executed as
new qualification.

## Unresolved checks and access limits

The preceding full-history hygiene run reported two findings. Recovery does not
suppress, rewrite, or bypass that failure; PR #3 remains unmerged. A source-only
comparison or private-data pattern check is not a replacement for Gitleaks.

The connected GitHub API was available. The browser extension returned a
missing-originating-context error, so this operation did not inspect the AWS
disk, remote working tree or additional machine-local changes. It did not start
instances, run cloud commands, change capacity, or modify shared workloads.
"Recovered" here means the available conversation archives, bundles and their
comparison with GitHub, not an audit of inaccessible machines.

Selected PureDarwin libraries, guest ABI execution, boot, persistence, recovery
and security enforcement remain independent qualification work. This recovery
makes no new guest-execution or M1-completion claim.
