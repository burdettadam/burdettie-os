# Maintained font dependency bridge

`cosmic-text/` is a source-only copy of the official **cosmic-text 0.19.0**
crates.io release, with the upstream `fontdb` 0.24 dependency change applied
to both Cargo manifests. All Rust library and test source, sample text,
README, and license notices are retained byte for byte. Seven upstream
binary test fonts and 27 PNG reference images/screenshots are not
redistributed. They are unnecessary for the library's features and the
Burdettie workspace tests, which use its existing bundled DejaVu fonts.
The normal workspace dependency remains named `cosmic-text`; the root
`[patch.crates-io]` selects this visible local source.

## Provenance and exact change

| Item | Identity |
| --- | --- |
| Official project | <https://github.com/pop-os/cosmic-text> |
| Published source | <https://crates.io/crates/cosmic-text/0.19.0> |
| Release commit | `c24886c2471e5606587c46090cd25dbbf209186b` |
| Release `.crate` SHA-256 | `be17b688510d934ce13f48a2beba700e11583e281e0fda99c22bb256a14eda73` |
| Upstream dependency migration | <https://github.com/pop-os/cosmic-text/commit/59089955e1c8698c6b83b2e6ab6ebceff825ff96> |
| Local patch | [cosmic-text-fontdb.patch](cosmic-text-fontdb.patch) |
| Original file hashes and exact omissions | [cosmic-text-upstream-files.json](cosmic-text-upstream-files.json) |

The dependency change is `fontdb = "0.23"` to `fontdb = "0.24"`.
[fontdb 0.24](https://docs.rs/crate/fontdb/0.24.0/source/CHANGELOG.md)
maintains its own minimal font metadata parser and no longer depends on
`ttf-parser`. The rest of cosmic-text's parsing and shaping already uses
Fontations and HarfRust. On 2026-11-14, the current official published
cosmic-text release still required fontdb 0.23; its main branch had accepted
this migration. This bridge makes that specific reviewed upstream change
available without adding a Git dependency or adopting an unrelated fork.

The upstream package's historical `Cargo.lock`, separate `deny.toml`, and
Git LFS `.gitattributes` are also omitted. This source-only bridge requires
no Git LFS service and contains no upstream font or PNG binary payloads.
The root workspace lockfile and unchanged root `deny.toml` policy
govern the active dependency graph. Every omitted file remains listed by
hash in the provenance manifest; the 34 binary omissions also record their
exact sizes, purpose, and official upstream pointer references. Cargo registry bookkeeping files
`.cargo-ok` and `.cargo-checksum.json` are not source and are also omitted.
All library code, features, and optional APIs are retained. The directory is
excluded from workspace membership, as an external
dependency, while remaining in Cargo's resolved graph and security scan.

Verify the retained upstream content, exact omission inventory, and the two
manifest edits:

```bash
python3 vendor/verify_cosmic.py
cargo deny --locked check
```

The existing Security Scanning CI job runs the verifier as a required step.
Its existing cargo-audit and configured cargo-deny gates remain, followed by
separate locked cargo-deny checks for Linux x86_64 and ARM64. No runner or policy
exception is added.

Remove this bridge when an official cosmic-text release has the maintained
fontdb dependency. Resolve the published graph, run the deterministic font
and shaping comparisons plus GUI tests, and rerun the required security
checks before removing the patch. Do not replace the pinned source by an
unreviewed moving branch.

## Upstream fixture availability and licenses

The Rust package is dual licensed under its retained
[MIT](cosmic-text/LICENSE-MIT) and
[Apache-2.0](cosmic-text/LICENSE-APACHE) licenses. The following upstream
test fonts are omitted. Their original notices are retained with the source
and their identities remain recorded in the omission manifest:

| Omitted upstream font assets | Copyright and retained license |
| --- | --- |
| `FiraMono-Medium.ttf` | Mozilla Foundation and Telefonica S.A.; [SIL OFL 1.1](cosmic-text/fonts/FiraMono-LICENSE) |
| `Inter-Regular.ttf`, `InterVariable.ttf`, `InterVariable-Italic.ttf` | The Inter Project Authors; [SIL OFL 1.1](cosmic-text/fonts/Inter-LICENSE) |
| `NotoSans-Regular.ttf`, `NotoSansArabic.ttf`, `NotoSansHebrew.ttf` | Google Inc.; [SIL OFL 1.1](cosmic-text/fonts/NotoSans-LICENSE) |

The upstream integration tests in `cosmic-text/tests/` include rendering,
wrapping, and variable-font cases that read these fonts and PNG reference
images. Their Rust source and test declarations are unmodified, but those
fixture-dependent suites cannot be qualified from this source-only checkout:
running them without the required assets will fail. The upstream README's
relative screenshot links likewise refer to omitted documentation images.

To investigate the separate upstream fixture suites, use a private extraction
of the official release identified above, verify its archive/file hashes,
and apply [the same dependency patch](cosmic-text-fontdb.patch). Retain its
original test inputs and reference images; do not generate replacement
goldens as a substitute for passing comparisons. This repository does not
automatically fetch or upload those omitted assets.

The required Burdettie workspace tests, including all glyph, shaping,
rasterization, editor, GPU-cache, malformed-font and collection regressions,
remain unchanged and run using the existing DejaVu fonts. No test is marked
ignored, no library feature is disabled, and no CI gate is removed by the
source-only packaging choice. This distinction preserves the actual test
coverage claim: Burdettie's regression suite is exercised; the complete
upstream fixture suite is available separately and is not claimed as run.
