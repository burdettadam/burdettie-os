//! Unicode text processing utilities for the text layout engine.
//!
//! Provides NFC normalization, grapheme cluster segmentation, combining
//! character detection, zero-width character filtering, and missing glyph
//! (emoji) placeholder rendering.

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Normalize a string to NFC form.
///
/// This composes combining character sequences into precomposed characters
/// where possible (e.g., `e` + `\u{0301}` → `é`), which improves glyph
/// lookup success in most fonts.
pub fn normalize_nfc(text: &str) -> String {
    text.nfc().collect()
}

/// Return the grapheme clusters of a string.
pub fn grapheme_clusters(text: &str) -> Vec<&str> {
    text.graphemes(true).collect()
}

/// Returns `true` if `ch` is a Unicode combining mark (general categories M*).
///
/// Combining marks should be positioned over the preceding base character
/// rather than advancing the cursor.
pub fn is_combining_mark(ch: char) -> bool {
    // Unicode general categories Mn (nonspacing), Mc (spacing combining), Me (enclosing)
    matches!(unicode_category(ch),
        UnicodeCategory::Mn | UnicodeCategory::Mc | UnicodeCategory::Me)
}

/// Returns `true` if `ch` is a zero-width formatting character that should
/// not produce any visible output or cursor advance.
pub fn is_zero_width(ch: char) -> bool {
    matches!(ch,
        '\u{200B}'  // Zero-width space
        | '\u{200C}' // Zero-width non-joiner (ZWNJ)
        | '\u{200D}' // Zero-width joiner (ZWJ)
        | '\u{FEFF}' // Byte order mark / zero-width no-break space
        | '\u{00AD}' // Soft hyphen
        | '\u{2060}' // Word joiner
        | '\u{2061}' // Function application (invisible)
        | '\u{2062}' // Invisible times
        | '\u{2063}' // Invisible separator
        | '\u{2064}' // Invisible plus
        | '\u{034F}' // Combining grapheme joiner
        | '\u{061C}' // Arabic letter mark
        | '\u{180E}' // Mongolian vowel separator
    ) || is_bidi_control(ch)
}

/// Returns `true` if `ch` is a Unicode bidirectional control character.
fn is_bidi_control(ch: char) -> bool {
    matches!(ch,
        '\u{200E}' // Left-to-right mark
        | '\u{200F}' // Right-to-left mark
        | '\u{202A}' // Left-to-right embedding
        | '\u{202B}' // Right-to-left embedding
        | '\u{202C}' // Pop directional formatting
        | '\u{202D}' // Left-to-right override
        | '\u{202E}' // Right-to-left override
        | '\u{2066}' // Left-to-right isolate
        | '\u{2067}' // Right-to-left isolate
        | '\u{2068}' // First strong isolate
        | '\u{2069}' // Pop directional isolate
    )
}

/// Returns `true` if the given character is likely an emoji that requires a
/// color/symbol font (not present in DejaVu Sans).
///
/// This is an approximation covering the major emoji ranges.
pub fn is_emoji(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        // Miscellaneous Symbols and Dingbats
        0x2600..=0x27BF |
        // Emoticons
        0x1F600..=0x1F64F |
        // Miscellaneous Symbols and Pictographs
        0x1F300..=0x1F5FF |
        // Transport and Map Symbols
        0x1F680..=0x1F6FF |
        // Supplemental Symbols and Pictographs
        0x1F900..=0x1F9FF |
        // Symbols and Pictographs Extended-A
        0x1FA00..=0x1FA6F |
        // Symbols and Pictographs Extended-B
        0x1FA70..=0x1FAFF |
        // Regional Indicator Symbols
        0x1F1E0..=0x1F1FF |
        // Variation selectors (emoji presentation)
        0xFE00..=0xFE0F |
        // Tags block (used in flag sequences)
        0xE0020..=0xE007F
    )
}

/// Simplified Unicode general category for combining mark detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnicodeCategory {
    /// Nonspacing combining mark (Mn)
    Mn,
    /// Spacing combining mark (Mc)
    Mc,
    /// Enclosing combining mark (Me)
    Me,
    /// Anything else
    Other,
}

/// Determine the Unicode general category for combining mark detection.
///
/// Uses Unicode ranges for combining marks rather than a full UCD table.
fn unicode_category(ch: char) -> UnicodeCategory {
    let cp = ch as u32;

    // Nonspacing marks (Mn) — major ranges
    if matches!(cp,
        0x0300..=0x036F   // Combining Diacritical Marks
        | 0x0483..=0x0489 // Cyrillic combining marks
        | 0x0591..=0x05BD // Hebrew combining marks
        | 0x05BF
        | 0x05C1..=0x05C2
        | 0x05C4..=0x05C5
        | 0x05C7
        | 0x0610..=0x061A // Arabic combining
        | 0x064B..=0x065F
        | 0x0670
        | 0x06D6..=0x06DC
        | 0x06DF..=0x06E4
        | 0x06E7..=0x06E8
        | 0x06EA..=0x06ED
        | 0x0711
        | 0x0730..=0x074A // Syriac
        | 0x07A6..=0x07B0 // Thaana
        | 0x07EB..=0x07F3 // NKo
        | 0x0816..=0x0819
        | 0x081B..=0x0823
        | 0x0825..=0x0827
        | 0x0829..=0x082D
        | 0x0859..=0x085B
        | 0x08D3..=0x08E1 // Arabic Extended-A
        | 0x08E3..=0x0902 // Arabic Extended-A / Devanagari
        | 0x093A
        | 0x093C
        | 0x0941..=0x0948
        | 0x094D
        | 0x0951..=0x0957
        | 0x0962..=0x0963 // Devanagari
        | 0x0981
        | 0x09BC
        | 0x09C1..=0x09C4
        | 0x09CD
        | 0x09E2..=0x09E3 // Bengali
        | 0x0A01..=0x0A02
        | 0x0A3C
        | 0x0A41..=0x0A42
        | 0x0A47..=0x0A48
        | 0x0A4B..=0x0A4D
        | 0x0A51
        | 0x0A70..=0x0A71
        | 0x0A75 // Gurmukhi
        | 0x0A81..=0x0A82
        | 0x0ABC
        | 0x0AC1..=0x0AC5
        | 0x0AC7..=0x0AC8
        | 0x0ACD
        | 0x0AE2..=0x0AE3 // Gujarati
        | 0x0AFA..=0x0AFF
        | 0x0B01
        | 0x0B3C
        | 0x0B3F
        | 0x0B41..=0x0B44
        | 0x0B4D
        | 0x0B56
        | 0x0B62..=0x0B63 // Oriya
        | 0x0B82
        | 0x0BC0
        | 0x0BCD // Tamil
        | 0x0C00
        | 0x0C04
        | 0x0C3E..=0x0C40
        | 0x0C46..=0x0C48
        | 0x0C4A..=0x0C4D
        | 0x0C55..=0x0C56
        | 0x0C62..=0x0C63 // Telugu
        | 0x0C81
        | 0x0CBC
        | 0x0CBF
        | 0x0CC6
        | 0x0CCC..=0x0CCD
        | 0x0CE2..=0x0CE3 // Kannada
        | 0x0D00..=0x0D01
        | 0x0D3B..=0x0D3C
        | 0x0D41..=0x0D44
        | 0x0D4D
        | 0x0D62..=0x0D63 // Malayalam
        | 0x0DCA
        | 0x0DD2..=0x0DD4
        | 0x0DD6 // Sinhala
        | 0x0E31
        | 0x0E34..=0x0E3A // Thai
        | 0x0E47..=0x0E4E
        | 0x0EB1
        | 0x0EB4..=0x0EBC // Lao
        | 0x0EC8..=0x0ECD
        | 0x0F18..=0x0F19
        | 0x0F35
        | 0x0F37
        | 0x0F39
        | 0x0F71..=0x0F7E
        | 0x0F80..=0x0F84
        | 0x0F86..=0x0F87
        | 0x0F8D..=0x0F97
        | 0x0F99..=0x0FBC
        | 0x0FC6 // Tibetan
        | 0x102D..=0x1030
        | 0x1032..=0x1037
        | 0x1039..=0x103A
        | 0x103D..=0x103E
        | 0x1058..=0x1059
        | 0x105E..=0x1060
        | 0x1071..=0x1074
        | 0x1082
        | 0x1085..=0x1086
        | 0x108D // Myanmar
        | 0x135D..=0x135F // Ethiopic
        | 0x1712..=0x1714 // Tagalog
        | 0x1732..=0x1734 // Hanunoo
        | 0x1752..=0x1753 // Buhid
        | 0x1772..=0x1773 // Tagbanwa
        | 0x17B4..=0x17B5
        | 0x17B7..=0x17BD
        | 0x17C6
        | 0x17C9..=0x17D3
        | 0x17DD // Khmer
        | 0x180B..=0x180D // Mongolian
        | 0x1885..=0x1886
        | 0x18A9
        | 0x1920..=0x1922
        | 0x1927..=0x1928
        | 0x1932
        | 0x1939..=0x193B // Limbu / New Tai Lue
        | 0x1A17..=0x1A18
        | 0x1A1B
        | 0x1A56
        | 0x1A58..=0x1A5E
        | 0x1A60
        | 0x1A62
        | 0x1A65..=0x1A6C
        | 0x1A73..=0x1A7C
        | 0x1A7F // Tai Tham
        | 0x1AB0..=0x1ABE // Combining Diacritical Marks Extended
        | 0x1B00..=0x1B03
        | 0x1B34
        | 0x1B36..=0x1B3A
        | 0x1B3C
        | 0x1B42
        | 0x1B6B..=0x1B73 // Balinese
        | 0x1B80..=0x1B81
        | 0x1BA2..=0x1BA5
        | 0x1BA8..=0x1BA9
        | 0x1BAB..=0x1BAD // Sundanese
        | 0x1BE6
        | 0x1BE8..=0x1BE9
        | 0x1BED
        | 0x1BEF..=0x1BF1 // Batak
        | 0x1C2C..=0x1C33
        | 0x1C36..=0x1C37 // Lepcha
        | 0x1CD0..=0x1CD2
        | 0x1CD4..=0x1CE0
        | 0x1CE2..=0x1CE8
        | 0x1CED
        | 0x1CF4
        | 0x1CF8..=0x1CF9 // Vedic Extensions
        | 0x1DC0..=0x1DF9 // Combining Diacritical Marks Supplement
        | 0x1DFB..=0x1DFF
        | 0x20D0..=0x20F0 // Combining Diacritical Marks for Symbols
        | 0xFE20..=0xFE2F // Combining Half Marks
    ) {
        return UnicodeCategory::Mn;
    }

    // Spacing combining marks (Mc) — simplified major ranges
    if matches!(cp,
        0x0903 | 0x093B | 0x093E..=0x0940 | 0x0949..=0x094C | 0x094E..=0x094F
        | 0x0982..=0x0983 | 0x09BE..=0x09C0 | 0x09C7..=0x09C8 | 0x09CB..=0x09CC
        | 0x09D7
        | 0x0A03 | 0x0A3E..=0x0A40 | 0x0A83 | 0x0ABE..=0x0AC0 | 0x0AC9 | 0x0ACB..=0x0ACC
        | 0x0B02..=0x0B03 | 0x0B3E | 0x0B40 | 0x0B47..=0x0B48 | 0x0B4B..=0x0B4C | 0x0B57
        | 0x0BBE..=0x0BBF | 0x0BC1..=0x0BC2 | 0x0BC6..=0x0BC8 | 0x0BCA..=0x0BCC | 0x0BD7
        | 0x0C01..=0x0C03 | 0x0C41..=0x0C44 | 0x0C82..=0x0C83 | 0x0CBE | 0x0CC0..=0x0CC4
        | 0x0CC7..=0x0CC8 | 0x0CCA..=0x0CCB | 0x0CD5..=0x0CD6
        | 0x0D02..=0x0D03 | 0x0D3E..=0x0D40 | 0x0D46..=0x0D48 | 0x0D4A..=0x0D4C | 0x0D57
        | 0x0D82..=0x0D83 | 0x0DCF..=0x0DD1 | 0x0DD8..=0x0DDF | 0x0DF2..=0x0DF3
        | 0x0F3E..=0x0F3F | 0x0F7F
        | 0x1031 | 0x103B..=0x103C
        | 0x1056..=0x1057 | 0x1084 | 0x17BE..=0x17C5 | 0x17C7..=0x17C8
    ) {
        return UnicodeCategory::Mc;
    }

    // Enclosing combining marks (Me)
    if matches!(cp, 0x0488..=0x0489 | 0x1ABE | 0x20DD..=0x20E0 | 0x20E2..=0x20E4 | 0xA670..=0xA672) {
        return UnicodeCategory::Me;
    }

    UnicodeCategory::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_normalization() {
        // NFD: e + combining acute → NFC: é
        let nfd = "e\u{0301}";
        let nfc = normalize_nfc(nfd);
        assert_eq!(nfc, "é");
        assert_eq!(nfc.chars().count(), 1);
    }

    #[test]
    fn grapheme_cluster_basic() {
        // Flag emoji: two regional indicators = one grapheme
        let flag = "\u{1F1FA}\u{1F1F8}"; // 🇺🇸
        let clusters = grapheme_clusters(flag);
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn grapheme_cluster_combining() {
        // e + combining acute = one grapheme cluster
        let text = "e\u{0301}";
        let clusters = grapheme_clusters(text);
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn combining_mark_detection() {
        assert!(is_combining_mark('\u{0301}')); // combining acute
        assert!(is_combining_mark('\u{0308}')); // combining diaeresis
        assert!(is_combining_mark('\u{0327}')); // combining cedilla
        assert!(!is_combining_mark('a'));
        assert!(!is_combining_mark(' '));
    }

    #[test]
    fn zero_width_detection() {
        assert!(is_zero_width('\u{200B}')); // ZWSP
        assert!(is_zero_width('\u{200D}')); // ZWJ
        assert!(is_zero_width('\u{200C}')); // ZWNJ
        assert!(is_zero_width('\u{FEFF}')); // BOM
        assert!(is_zero_width('\u{00AD}')); // soft hyphen
        assert!(!is_zero_width('a'));
        assert!(!is_zero_width(' '));
    }

    #[test]
    fn emoji_detection() {
        assert!(is_emoji('😀'));       // U+1F600
        assert!(is_emoji('🎉'));       // U+1F389
        assert!(is_emoji('🚀'));       // U+1F680
        assert!(is_emoji('\u{2600}')); // ☀ sun
        assert!(!is_emoji('a'));
        assert!(!is_emoji(' '));
    }
}
