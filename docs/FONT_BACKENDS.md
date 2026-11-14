# Maintained font backends

The font migration addresses
[issue #2](https://github.com/burdettadam/burdettie-os/issues/2) and
[RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192.html).
The advisory reports that `ttf-parser` is unmaintained and has no patched
release. It recommends Fontations' `skrifa` as a maintained parser.

## Dependency paths

| Path | Replacement |
| --- | --- |
| `graphics-core → ab_glyph → owned_ttf_parser → ttf-parser` | `graphics-core → skrifa 0.50.0 → read-fonts`; `ab_glyph_rasterizer` remains as the separate curve rasterizer and has no font-parsing dependency |
| `graphics-core → cosmic-text 0.18.2 → fontdb 0.23 → ttf-parser` | Official `cosmic-text 0.19.0` with its upstream `fontdb 0.24` manifest migration; HarfRust shaping and Swash rendering remain enabled |
| Linux `winit → sctk-adwaita → ab_glyph` | Winit's supported `wayland-csd-adwaita-crossfont` renderer, using Crossfont/FreeType and Fontconfig |

The root lockfile contains no `ab_glyph`, `owned_ttf_parser`, or `ttf-parser`
package. Both window backends inherit the same Winit declaration. Raw-window
handle 0.6, X11, Wayland, Wayland dynamic loading, and client-side window
titles remain enabled. Crossfont is reachable only on Winit's supported
Unix windowing targets; it is absent from both Darwin target graphs.

The official cosmic-text release had not yet published its upstream fontdb
migration when this change was made. The source bridge, exact patch, file
hashes, exact binary-fixture omissions, retained upstream licenses, and removal condition are recorded in
[vendor/README.md](../vendor/README.md). The root `deny.toml` policy is unchanged.
No package identity is rewritten to conceal the old parser.

The vendored package is source-only. Its seven upstream test fonts and 27
PNG fixtures/screenshots are omitted because library features do not depend
on them. All upstream Rust test source remains unchanged, but its separate
fixture-dependent integration suites require an official private extraction
with those assets present. Burdettie's required workspace tests and captured
regressions use the existing DejaVu fonts and are unchanged; no upstream
fixture-suite result is claimed for the source-only checkout.

## Font abstraction and compatibility

`FontFace` owns immutable bytes validated through Fontations and exposes the
existing metrics and measurement API. `FontFace::from_bytes_at_index` also
allows explicit TrueType/OpenType collection selection; standalone files
accept only index zero. Invalid directories, out-of-range table payloads,
missing required layout tables, and invalid collection indices return `None`.
Parsing borrowed handles never requires an unsafe lifetime extension.

The layout engines, software renderer, and GPU atlas share this same backend.
`LayoutGlyph::glyph_id` now uses the public `graphics_core::GlyphId(u16)` type,
so external code constructing glyphs must use that type instead of
`ab_glyph::GlyphId`. Glyph indices remain local to a font face and zero remains
the missing-glyph index.

The pixel-size convention is preserved explicitly: a requested size measures
the ascent-to-descent height. Design-unit metrics, horizontal advance, legacy
horizontal pair kerning, and unhinted outlines are multiplied by
`size / (ascent - descent)`. Using a font's units per em directly would change
the existing layout and rendering size. The simple layout path preserves its
first applicable horizontal pair-kerning behavior; complex-script shaping
continues through cosmic-text/HarfRust.

Outline rasterization uses the existing independent `ab_glyph_rasterizer`.
The Fontations pen supplies lines, quadratic curves, cubic curves, and closed
contours. Pixel bounds retain subpixel-origin rounding. Invalid sizes and
non-finite positions are rejected, and a single glyph's raster allocation is
limited to 16,777,216 pixels. The GPU atlas retains its stricter size limit,
per-face identity, exact fractional-size key, and transactional placement.

The cosmic-text 0.19 API makes text/size changes mark buffers dirty. The
adapter uses its updated setters and retains explicit `shape_until_scroll`
calls before measurement, drawing, cursor operations, or wrapped layout.

## Regression evidence

The recorded baselines were generated from the pre-migration dependency
versions at main commit `7bf375562ca9edf513c353da7ec3309c0337f457`, before
switching the implementations. Both baselines use only the five bundled
DejaVu faces; host fallback fonts are excluded from these comparisons.

| Coverage | Assertions |
| --- | --- |
| Five DejaVu faces, sizes 13, 17.25, and 24.5 pixels | Glyph IDs, ascent/descent/line gap, advances, and `AV` kerning match within 0.00001 pixels |
| `A`, `V`, `g`, `é`, combining acute, missing codepoint, and space | 105 glyph samples; exact unscaled curve geometry after ignoring zero-length closing segments; equal pixel bounds and at most one 8-bit coverage level of raster difference |
| NFC/NFD, combining marks, zero-width joiner, and multiple accents | Measured widths match the captured old backend |
| Ligatures, mixed Hebrew/Latin, Arabic, and wrapped/unwrapped text | Shaped glyph IDs, cluster byte ranges, direction, positions, advances, and line geometry match cosmic-text 0.18.2 |
| Font database selection | Regular, bold, italic, bold italic, and monospace queries return the exact intended embedded font bytes |
| Editor integration | Hit testing stays on the expected UTF-8 cluster; highlighting has visible spans; full selection copies the original mixed-script text; shaped glyphs rasterize |
| Invalid inputs and collections | Truncated font data, invalid table offsets, invalid glyph/size/position inputs and invalid indices fail without panic; distinct collection faces retain distinct metrics |

The data files are
[dejavu-baseline.json](../crates/graphics-core/src/font_backend/data/dejavu-baseline.json)
and [shaping-baseline.json](../crates/graphics-core/src/font_backend/data/shaping-baseline.json).
The DejaVu baseline records the original source-font SHA-256 values as well
as parser versions. Expected values are retained as data rather than
recomputed through the replacement implementation.

## Validation and remaining qualification

Use the pinned Rust 1.93.0 toolchain and the checked-in lockfile:

```bash
python3 vendor/verify_cosmic.py
cargo test --locked -p graphics-core --lib
cargo test --locked -p graphics-core --lib --features gpu
cargo test --locked -p app-kit-lite -p ui-controls --lib
cargo check --locked -p window-backend-linux -p control-gallery \
  -p foundation-explorer -p launchpad -p file-explorer --all-targets
cargo deny --locked check
cargo deny --locked --target x86_64-unknown-linux-gnu check
cargo deny --locked --target aarch64-unknown-linux-gnu check
```

The local Linux validation includes actual native GUI compilation with
FreeType and Fontconfig, graphics/GPU tests, the controls tests, strict
Clippy with the existing CI allowances, and cargo-deny. Resolved all-feature
graphs were also inspected for `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, and
`aarch64-apple-darwin`: all exclude the three retired parser-path packages.
The default and separate Linux x86_64/ARM64 cargo-deny checks pass advisories, bans,
licenses, and sources. Duplicate-version and unused-license-allowance
warnings remain visible.

The existing Security Scanning CI job requires the vendor source verifier
and both Linux scans in addition to its existing cargo-audit and
configured cargo-deny checks. Explicit Linux targets extend coverage beyond
the configured Darwin graph without changing advisory, license, ban, or
source policy. cargo-deny 0.20.2 rejects repeated `--target` arguments, so
each architecture has its own required invocation; `--locked` rejects any
lockfile change.

Native macOS execution, full workspace CI, cargo-audit, and final repository
checks must pass on the exact published change before integration. A local
cargo-audit installation and initial controls link attempt encountered a full
scratch disk; their failure logs were retained, disposable compilation
outputs were reclaimed, and the controls tests were rerun successfully.
No advisory check was ignored. This change validates the host font stack;
it does not qualify selected-source OS boot, guest ABI execution, or security
enforcement for M1.
