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
