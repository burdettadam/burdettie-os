# M2 parse-cache content identity

Status: implemented on the M2 admission branch; exact-head CI is required before
acceptance. This closes the stale-path parse-cache defect only. It does not
complete M2 admission or establish a signature trust boundary.

## Defect

The process-wide parse cache survived runtime resets and was keyed by raw and
canonical filesystem paths. A warm path hit returned the previous immutable
bytes and ParsedMachO without reading the current file. Replacing an executable
at the same path could therefore continue mapping and evaluating the old cached
image. Re-evaluating policy against a path did not repair that mismatch: the
mapped bytes and current path contents could describe different revisions.

## Change

All native, interpreted and typestate entry points now read the executable
through the configured FilesystemProvider on every invocation before looking
up the parse cache. The cache stores SHA-256 of the complete source file and
reuses its ParsedMachO and mapped bytes only when the current source digest
matches. A mismatch is a cache miss; the caller parses the bytes it just read
and replaces the raw/canonical entries.

The digest is calculated over the complete source file before architecture
selection. For universal/fat inputs, this binds the selected thin mapped slice
and ParsedMachO to the exact full container revision rather than hashing only
the slice. Runtime reset can still retain immutable entries safely because
content is revalidated on the next load. Security policy evaluation remains
outside the cache and runs on each invocation.

This deliberately gives up the old no-I/O warm-cache optimization. It retains
the parse-time saving while ensuring that the bytes parsed and mapped are from
the current FilesystemProvider read. A future optimization may use an
immutable, provider-issued file revision/descriptor, but path timestamps,
size-only checks and unverified metadata are not substitutes for content
identity.

## Regression coverage

Three source-owned tests cover:

- Reuse of the same immutable Arc data and parse for identical source bytes.
- Same-path, same-size replacement with only an otherwise ignored payload byte
  changed; both raw and canonical aliases must miss.
- A stale raw alias cannot hide a changed source revision.

The generated input is a minimal valid ARM64 MH_EXECUTE. No SDK, Apple binary,
network, user application or live clipboard is required.

Run the existing workspace checks:

```sh
cargo fmt --all -- --check
cargo test --locked -p macho-runtime parse_cache
cargo test --locked -p macho-runtime --test runtime_admission_tests
```

The exact-head required CI must run these tests. The source changes have not
been executed with a local Rust toolchain in the current environment.

## Explicit remaining gaps

- The policy evaluator still receives a path; this increment does not prove
  signature verification and mapping consume one immutable opened-file object.
- Dynamic dlopen admission is not implemented by this change.
- System-library path prefixes are still skipped by the dependency loader;
  provider identity/capability policy remains open.
- ImageRegistry install-name aliases can still conflate distinct resolved
  files; cache content identity does not repair registry alias collisions.
- Dependency-edge evaluation for already-mapped images and transactional
  rollback after a later denial remain separate work.
- This is not OS-level file/network/process confinement.

See [Issue #6](https://github.com/burdettadam/burdettie-os/issues/6) for the
remaining M2 acceptance criteria.


## Follow-on: dependency alias identity

The shared dependency loader now resolves, reads, parses and evaluates each
resolved dependency edge before considering registry reuse. A normalized
install name is not treated as proof that two paths identify the same image.
If an existing registry key resolves to different bytes, the loader returns a
fail-closed security denial instead of silently skipping the second path.

Dependency expansion is tracked by canonical resolved path plus SHA-256 of
the image bytes. This allows two directories to contain the same relative
`@loader_path/child.dylib` name without conflating their dependency closures.
Each edge is admitted before an already-mapped image can be reused. Inserted
libraries receive the same byte comparison for basename collisions; identical
bytes can reuse the mapping while their loader directories are still traversed.

Three additional cross-route tests cover:
- A second loader edge denied after the same normalized install name was mapped
  from a different directory.
- Different bytes resolving under the same normalized dependency install name.
- Different inserted roots with the same basename.

The PR is based on the M1 development branch so these M2 checks are exercised
with the M1 platform changes in the synthetic merge. The CI workflow now
includes that branch in its pull-request target filter. Security tools are
pinned to `cargo-audit 0.23.0` and `cargo-deny 0.21.0` for Rust 1.93.0.

The M2 acceptance gaps listed above remain open: dynamic-load admission,
versioned system-provider identity, required/weak dependency semantics,
signature-verification-to-mapping byte identity, complete mode integration,
and initializer-level negative execution tests.


## M1 substrate source refresh — 2026-11-23

The M2 PR is based on the refreshed M1 branch head
`a179710f681c08fce678fed051654c79b333e7e1`. M1 now pins the current
PureDarwin development head `0c1990c803c13b68cad4df3f6dc34ba4306fc789`
instead of the prior `0163b2d` candidate. The current-source image build and
guest boot are not yet qualified; the previous 20.5 bootstrap and previous
source-specific toolchain results remain historical.

This M1 source-selection change does not alter the M2 runtime implementation,
but the full M2 CI must run again against the updated synthetic merge base.
Host tests and Linux ARM64 interpreter tests do not establish current
PureDarwin guest execution or OS-level confinement.
