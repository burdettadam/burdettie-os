#!/usr/bin/env python3
"""Verify the retained official release and its sole dependency migration."""

import hashlib
import json
from pathlib import Path


BINARY_FIXTURES = {
    "fonts/FiraMono-Medium.ttf",
    "fonts/Inter-Regular.ttf",
    "fonts/InterVariable-Italic.ttf",
    "fonts/InterVariable.ttf",
    "fonts/NotoSans-Regular.ttf",
    "fonts/NotoSansArabic.ttf",
    "fonts/NotoSansHebrew.ttf",
    "screenshots/arabic.png",
    "screenshots/chinese-simplified.png",
    "screenshots/hindi.png",
    "tests/images/a_hebrew_paragraph.png",
    "tests/images/a_hebrew_word.png",
    "tests/images/an_arabic_paragraph.png",
    "tests/images/an_arabic_word.png",
    "tests/images/ellipsize_bidi_isolates_middle_bug.png",
    "tests/images/ellipsize_ltr_end_single_line.png",
    "tests/images/ellipsize_ltr_end_single_line_aligned_right.png",
    "tests/images/ellipsize_ltr_end_two_lines.png",
    "tests/images/ellipsize_ltr_middle_single_line.png",
    "tests/images/ellipsize_ltr_middle_small_buffer.png",
    "tests/images/ellipsize_ltr_single_word_middle_two_lines.png",
    "tests/images/ellipsize_ltr_start_single_line.png",
    "tests/images/ellipsize_mixed_end_single_line.png",
    "tests/images/ellipsize_mixed_ltr_rtl_ltr_middle_three_lines.png",
    "tests/images/ellipsize_mixed_ltr_rtl_middle_two_lines.png",
    "tests/images/ellipsize_mixed_middle_single_line.png",
    "tests/images/ellipsize_mixed_rtl_ltr_middle_two_lines.png",
    "tests/images/ellipsize_rtl_end_single_line.png",
    "tests/images/some_english_mixed_with_arabic.png",
    "tests/images/some_english_mixed_with_hebrew.png",
    "tests/images/text_decoration_bidi.png",
    "tests/images/text_decoration_multiline_bidi.png",
    "tests/images/text_decoration_rtl.png",
    "tests/images/text_decorations.png",
}


def main() -> None:
    vendor = Path(__file__).resolve().parent
    manifest = json.loads((vendor / "cosmic-text-upstream-files.json").read_text())
    source = vendor / "cosmic-text"
    assert manifest["crate"] == "cosmic-text" and manifest["version"] == "0.19.0"
    assert manifest["git_commit"] == "c24886c2471e5606587c46090cd25dbbf209186b"
    assert manifest["crate_sha256"] == "be17b688510d934ce13f48a2beba700e11583e281e0fda99c22bb256a14eda73"
    omitted = {".gitattributes", "Cargo.lock", "deny.toml"} | BINARY_FIXTURES
    assert set(manifest["omitted_files"]) == omitted
    expected = manifest["upstream_files_sha256"]
    assets = manifest["omitted_binary_assets"]
    assert len(assets) == len(BINARY_FIXTURES)
    assert {asset["path"] for asset in assets} == BINARY_FIXTURES
    for asset in assets:
        assert asset["sha256"] == expected[asset["path"]], asset["path"]
        assert isinstance(asset["bytes"], int) and asset["bytes"] > 0
    actual = {p.relative_to(source).as_posix() for p in source.rglob("*") if p.is_file()}
    assert actual == set(expected) - omitted, "unexpected or missing vendored source files"
    changes = {
        "Cargo.toml": (b'[dependencies.fontdb]\nversion = "0.24"', b'[dependencies.fontdb]\nversion = "0.23"'),
        "Cargo.toml.orig": (b'fontdb = { version = "0.24"', b'fontdb = { version = "0.23"'),
    }
    for relative in sorted(actual):
        data = (source / relative).read_bytes()
        if relative in changes:
            new, old = changes[relative]
            assert data.count(new) == 1, f"unexpected patch in {relative}"
            data = data.replace(new, old, 1)
        assert hashlib.sha256(data).hexdigest() == expected[relative], relative
    print(f"Verified {len(actual)} retained upstream files and records for {len(assets)} omitted binary fixtures; "
          "only the two recorded fontdb manifest edits differ.")


if __name__ == "__main__":
    main()
