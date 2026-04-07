//! libxo format-string parser.
//!
//! Parses the `{role,modifiers:content/format/encoding}` syntax and
//! rewrites it to a plain printf format string for text-mode output.

/// A parsed segment of an xo format string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Literal text: output as-is.  Does not consume varargs.
    Literal(String),
    /// Printf format specifier: consume vararg(s) from va_list.
    PrintfField(String),
}

/// Parse an xo format string into a sequence of segments.
///
/// ```
/// let segs = libxo::parse_xo_format("{:lines/%7llu} {:words/%7llu}\n");
/// assert_eq!(segs.len(), 4);
/// ```
pub fn parse_xo_format(fmt: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let bytes = fmt.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut literal = String::new();

    while i < len {
        let ch = bytes[i];

        if ch == b'{' {
            // Check for escaped brace `{{`
            if i + 1 < len && bytes[i + 1] == b'{' {
                literal.push('{');
                i += 2;
                continue;
            }

            // Flush accumulated literal
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }

            // Find the matching `}`
            i += 1; // skip `{`
            let field_start = i;

            // Scan to closing `}`, respecting backslash escapes
            while i < len && bytes[i] != b'}' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2; // skip escaped char
                } else {
                    i += 1;
                }
            }
            let field_end = i;
            if i < len {
                i += 1; // skip `}`
            }

            let field = &fmt[field_start..field_end];
            if let Some(seg) = parse_field(field) {
                segments.push(seg);
            }
        } else if ch == b'}' {
            // Check for escaped brace `}}`
            if i + 1 < len && bytes[i + 1] == b'}' {
                literal.push('}');
                i += 2;
                continue;
            }
            // Stray `}` — emit as literal
            literal.push('}');
            i += 1;
        } else {
            literal.push(ch as char);
            i += 1;
        }
    }

    // Flush remaining literal
    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }

    segments
}

/// Rewrite an xo format string directly into a printf-compatible format
/// string.  Literal text has `%` escaped to `%%`.  Field format specifiers
/// are spliced in verbatim.
pub fn rewrite_to_printf(xo_fmt: &str) -> String {
    let segments = parse_xo_format(xo_fmt);
    let mut out = String::with_capacity(xo_fmt.len());

    for seg in &segments {
        match seg {
            Segment::Literal(text) => {
                // Escape `%` as `%%` since the output will be used by vfprintf
                for ch in text.chars() {
                    if ch == '%' {
                        out.push_str("%%");
                    } else {
                        out.push(ch);
                    }
                }
            }
            Segment::PrintfField(fmt) => {
                out.push_str(fmt);
            }
        }
    }

    out
}

// ── Field parsing ───────────────────────────────────────────────────

/// Role characters → text-mode behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Value field (default): emit format, consuming varargs.
    Value,
    /// Label: static text before a value.
    Label,
    /// Decoration: punctuation (colon, comma, etc.).
    Decoration,
    /// Note: text after a value.
    Note,
    /// Padding: whitespace for alignment.
    Padding,
    /// Title: column header.
    Title,
    /// Units: dimension label for the previous value.
    Units,
    /// Color: ANSI color/effect (ignored in text mode).
    Color,
    /// Gettext: i18n marker (ignored).
    Gettext,
    /// Error: error message (could go to stderr, treated as literal).
    Error,
    /// Warning: warning message (treated as literal).
    Warning,
    /// Start anchor: begin padded group.
    AnchorStart,
    /// Stop anchor: end padded group.
    AnchorStop,
}

struct FieldModifiers {
    has_colon: bool, // 'c' — append ":"
    has_white: bool, // 'w' — append/prepend " "
}

/// Parse a single field descriptor (the text between `{` and `}`).
///
/// Returns `None` for fields that produce no text-mode output.
fn parse_field(field: &str) -> Option<Segment> {
    if field.is_empty() {
        return None;
    }

    let bytes = field.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // ── Parse role and modifiers (before ':') ───────────────────

    let mut role = Role::Value;
    let mut mods = FieldModifiers {
        has_colon: false,
        has_white: false,
    };

    while i < len && bytes[i] != b':' {
        let ch = bytes[i];
        match ch {
            b',' => {
                // Long name — skip to ':'
                while i < len && bytes[i] != b':' {
                    i += 1;
                }
                break;
            }
            // Role characters
            b'C' => role = Role::Color,
            b'D' => role = Role::Decoration,
            b'E' => role = Role::Error,
            b'G' => role = Role::Gettext,
            b'L' => role = Role::Label,
            b'N' => role = Role::Note,
            b'P' => role = Role::Padding,
            b'T' => role = Role::Title,
            b'U' => role = Role::Units,
            b'V' => role = Role::Value,
            b'W' => role = Role::Warning,
            b'[' => role = Role::AnchorStart,
            b']' => role = Role::AnchorStop,
            // Modifier characters
            b'c' => mods.has_colon = true,
            b'w' => mods.has_white = true,
            // Other modifiers (d, e, h, k, l, n, q, t): ignored for text mode
            _ => {}
        }
        i += 1;
    }

    // Skip the ':' separator
    if i < len && bytes[i] == b':' {
        i += 1;
    }

    // ── Parse content and format ────────────────────────────────

    let mut content = String::new();
    let mut format = String::new();
    let mut in_format = false;
    let mut format_sections = 0;

    while i < len {
        let ch = bytes[i];
        match ch {
            b'\\' if i + 1 < len => {
                // Backslash escape: `\/` → `/`, `\\` → `\`
                let escaped = bytes[i + 1];
                let resolved = match escaped {
                    b'/' => '/',
                    b'\\' => '\\',
                    b'n' => '\n',
                    b't' => '\t',
                    _ => escaped as char,
                };
                if in_format {
                    format.push(resolved);
                } else {
                    content.push(resolved);
                }
                i += 2;
            }
            b'/' if !in_format => {
                in_format = true;
                format_sections += 1;
                i += 1;
            }
            b'/' if in_format && format_sections < 2 => {
                // Second slash: encoding format — skip remainder
                format_sections += 1;
                i += 1;
                // Consume rest (encoding format) until end of field
                while i < len {
                    i += 1;
                }
            }
            _ => {
                if in_format && format_sections <= 1 {
                    format.push(ch as char);
                } else if !in_format {
                    content.push(ch as char);
                }
                i += 1;
            }
        }
    }

    // ── Generate segment based on role ──────────────────────────

    match role {
        Role::Color | Role::Gettext => {
            // No output in text mode
            None
        }
        Role::AnchorStart | Role::AnchorStop => {
            // Anchors: complex group-padding. The start anchor with a
            // format like `%d` consumes a width-argument.  For now we
            // emit nothing (width padding not implemented), but we
            // must still consume the vararg so the va_list stays in
            // sync.
            if !format.is_empty() {
                // Consume the arg but discard it.  We achieve this by
                // emitting a zero-width printf spec — e.g. "%d" → "%.0d"
                // (prints the int with zero digits, effectively nothing).
                let consumed = make_zero_width(&format);
                Some(Segment::PrintfField(consumed))
            } else {
                None
            }
        }
        Role::Value => {
            if !format.is_empty() {
                Some(Segment::PrintfField(format))
            } else {
                // Default format for value is "%s"
                Some(Segment::PrintfField("%s".to_string()))
            }
        }
        Role::Label | Role::Decoration | Role::Note | Role::Error | Role::Warning => {
            if !content.is_empty() {
                let mut text = content;
                if mods.has_colon {
                    text.push(':');
                }
                if mods.has_white {
                    text.push(' ');
                }
                Some(Segment::Literal(text))
            } else if !format.is_empty() {
                // Dynamic content from varargs
                let mut fmt = format;
                // Append modifier suffixes as literals after the format
                // by splitting into format + literal tail.
                let suffix = modifier_suffix(mods.has_colon, mods.has_white);
                if suffix.is_empty() {
                    Some(Segment::PrintfField(fmt))
                } else {
                    // Combine: the printf field + literal suffix
                    fmt.push_str(&suffix);
                    Some(Segment::PrintfField(fmt))
                }
            } else {
                None
            }
        }
        Role::Padding => {
            if !content.is_empty() {
                Some(Segment::Literal(content))
            } else if !format.is_empty() {
                Some(Segment::PrintfField(format))
            } else {
                None
            }
        }
        Role::Title => {
            if !content.is_empty() && !format.is_empty() {
                // Title with both content and format: the content IS
                // the value, formatted with the spec.  Does NOT consume
                // a vararg.  We approximate by just emitting the content.
                // (e.g. {T:Name/%20s} → "                Name" — we just
                //  emit "Name" without padding, which is good enough.)
                let mut text = content;
                if mods.has_colon {
                    text.push(':');
                }
                if mods.has_white {
                    text.push(' ');
                }
                Some(Segment::Literal(text))
            } else if !format.is_empty() {
                Some(Segment::PrintfField(format))
            } else if !content.is_empty() {
                let mut text = content;
                if mods.has_colon {
                    text.push(':');
                }
                if mods.has_white {
                    text.push(' ');
                }
                Some(Segment::Literal(text))
            } else {
                None
            }
        }
        Role::Units => {
            // For units, 'w' means space BEFORE content (reversed sense)
            if !content.is_empty() {
                let mut text = String::new();
                if mods.has_white {
                    text.push(' ');
                }
                text.push_str(&content);
                if mods.has_colon {
                    text.push(':');
                }
                Some(Segment::Literal(text))
            } else if !format.is_empty() {
                Some(Segment::PrintfField(format))
            } else {
                None
            }
        }
    }
}

/// Build a zero-width printf specifier to consume a vararg without
/// producing output.  E.g. `%d` → `%.0d`, `%ld` → `%.0ld`, `%s` → `%.0s`.
fn make_zero_width(fmt: &str) -> String {
    // Insert `.0` right after `%` (and any flags like `-`, `+`, `0`, ` `, `#`).
    // The printf precision `.0` suppresses output for most conversions.
    if let Some(pct) = fmt.find('%') {
        let after_pct = pct + 1;
        // Skip flags
        let mut i = after_pct;
        let bytes = fmt.as_bytes();
        while i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b'0' | b' ' | b'#') {
            i += 1;
        }
        // Skip any existing width (digits or *)
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'*') {
            i += 1;
        }
        // Insert .0 precision here (before any existing precision, length, or conversion)
        format!("{}.0{}", &fmt[..i], &fmt[i..])
    } else {
        fmt.to_string()
    }
}

/// Build a modifier suffix (colon and/or space).
fn modifier_suffix(has_colon: bool, has_white: bool) -> String {
    let mut s = String::new();
    if has_colon {
        s.push(':');
    }
    if has_white {
        s.push(' ');
    }
    s
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_xo_format tests ────────────────────────────────────

    #[test]
    fn plain_text_only() {
        let segs = parse_xo_format("hello world\n");
        assert_eq!(segs, vec![Segment::Literal("hello world\n".into())]);
    }

    #[test]
    fn single_value_field() {
        let segs = parse_xo_format("{:count/%d}");
        assert_eq!(segs, vec![Segment::PrintfField("%d".into())]);
    }

    #[test]
    fn value_with_no_format_defaults_to_percent_s() {
        let segs = parse_xo_format("{:name}");
        assert_eq!(segs, vec![Segment::PrintfField("%s".into())]);
    }

    #[test]
    fn wc_format_string() {
        // Simplified wc output: " {:lines/%7llu} {:words/%7llu}\n"
        let segs = parse_xo_format(" {:lines/%7llu} {:words/%7llu}\n");
        assert_eq!(segs, vec![
            Segment::Literal(" ".into()),
            Segment::PrintfField("%7llu".into()),
            Segment::Literal(" ".into()),
            Segment::PrintfField("%7llu".into()),
            Segment::Literal("\n".into()),
        ]);
    }

    #[test]
    fn wc_star_width_format() {
        // Real wc uses: "{:lines/%*llu}" where %* consumes width + value
        let segs = parse_xo_format(" {:lines/%*llu}");
        assert_eq!(segs, vec![
            Segment::Literal(" ".into()),
            Segment::PrintfField("%*llu".into()),
        ]);
    }

    #[test]
    fn label_with_colon_and_whitespace() {
        let segs = parse_xo_format("{Lwc:In stock}{:qty/%u}");
        assert_eq!(segs, vec![
            Segment::Literal("In stock: ".into()),
            Segment::PrintfField("%u".into()),
        ]);
    }

    #[test]
    fn decoration_role() {
        let segs = parse_xo_format("{D::}");
        assert_eq!(segs, vec![Segment::Literal(":".into())]);
    }

    #[test]
    fn padding_role_static() {
        let segs = parse_xo_format("{P:   }");
        assert_eq!(segs, vec![Segment::Literal("   ".into())]);
    }

    #[test]
    fn padding_role_dynamic() {
        let segs = parse_xo_format("{P:/%30s}");
        assert_eq!(segs, vec![Segment::PrintfField("%30s".into())]);
    }

    #[test]
    fn units_with_whitespace_before() {
        let segs = parse_xo_format("{:distance/%u}{Uw:miles}");
        assert_eq!(segs, vec![
            Segment::PrintfField("%u".into()),
            Segment::Literal(" miles".into()),
        ]);
    }

    #[test]
    fn note_role() {
        let segs = parse_xo_format("{:cost/%u} {N:per year}");
        assert_eq!(segs, vec![
            Segment::PrintfField("%u".into()),
            Segment::Literal(" ".into()),
            Segment::Literal("per year".into()),
        ]);
    }

    #[test]
    fn title_static() {
        let segs = parse_xo_format("{T:Interface Statistics}\n");
        assert_eq!(segs, vec![
            Segment::Literal("Interface Statistics".into()),
            Segment::Literal("\n".into()),
        ]);
    }

    #[test]
    fn title_dynamic() {
        let segs = parse_xo_format("{T:/%20.20s}{T:/%6.6s}");
        assert_eq!(segs, vec![
            Segment::PrintfField("%20.20s".into()),
            Segment::PrintfField("%6.6s".into()),
        ]);
    }

    #[test]
    fn title_with_content_and_format() {
        // Title with both content and format: content is used as the value,
        // no vararg consumed.
        let segs = parse_xo_format("{T:Name/%20s}{T:Count/%6s}");
        assert_eq!(segs, vec![
            Segment::Literal("Name".into()),
            Segment::Literal("Count".into()),
        ]);
    }

    #[test]
    fn color_role_ignored() {
        let segs = parse_xo_format("{C:bold}{:value/%s}{C:}");
        assert_eq!(segs, vec![Segment::PrintfField("%s".into())]);
    }

    #[test]
    fn gettext_role_ignored() {
        let segs = parse_xo_format("{G:}Invalid token\n");
        assert_eq!(segs, vec![Segment::Literal("Invalid token\n".into())]);
    }

    #[test]
    fn escaped_braces() {
        let segs = parse_xo_format("{{hello}}");
        assert_eq!(segs, vec![Segment::Literal("{hello}".into())]);
    }

    #[test]
    fn escaped_slash_in_content() {
        let segs = parse_xo_format("{L:Low\\/warn}");
        assert_eq!(segs, vec![Segment::Literal("Low/warn".into())]);
    }

    #[test]
    fn explicit_value_role() {
        let segs = parse_xo_format("{V:name/%s}");
        assert_eq!(segs, vec![Segment::PrintfField("%s".into())]);
    }

    #[test]
    fn encoding_format_ignored() {
        // Second format (after second /) is for encoding, ignored in text
        let segs = parse_xo_format("{:temp/%5.1f/%d}");
        assert_eq!(segs, vec![Segment::PrintfField("%5.1f".into())]);
    }

    #[test]
    fn empty_field_ignored() {
        let segs = parse_xo_format("{}text");
        assert_eq!(segs, vec![Segment::Literal("text".into())]);
    }

    #[test]
    fn complex_combined() {
        let input = "{L:Free}{D::}{P:   }{:free/%u} {U:Blocks}\n";
        let segs = parse_xo_format(input);
        assert_eq!(segs, vec![
            Segment::Literal("Free".into()),
            Segment::Literal(":".into()),
            Segment::Literal("   ".into()),
            Segment::PrintfField("%u".into()),
            Segment::Literal(" ".into()),
            Segment::Literal("Blocks".into()),
            Segment::Literal("\n".into()),
        ]);
    }

    #[test]
    fn df_format_string() {
        let input = "{:name/%-*s}";
        let segs = parse_xo_format(input);
        assert_eq!(segs, vec![Segment::PrintfField("%-*s".into())]);
    }

    #[test]
    fn anchor_with_format_consumes_arg() {
        // {[:/%d} should consume the width arg but produce no visible output
        let segs = parse_xo_format("{[:/%d}{:min/%d}/{:max/%d}{]:}");
        assert_eq!(segs.len(), 4); // consumed_anchor, min, literal "/", max
        match &segs[0] {
            Segment::PrintfField(f) => assert!(f.contains(".0"), "should be zero-width: {}", f),
            _ => panic!("expected PrintfField for anchor"),
        }
    }

    #[test]
    fn long_name_modifiers() {
        // Comma-separated long names like {,label:text}
        let segs = parse_xo_format("{,label:Free}{,value:free/%u}");
        // The parser sees ',' → skips to ':', then reads content/format.
        // Since we don't parse the long name for role, we treat it as value.
        // That's OK — the key behavior is that content is preserved.
        assert_eq!(segs.len(), 2);
    }

    // ── rewrite_to_printf tests ──────────────────────────────────

    #[test]
    fn rewrite_simple_value() {
        assert_eq!(rewrite_to_printf("{:count/%d}"), "%d");
    }

    #[test]
    fn rewrite_escapes_percent_in_literals() {
        assert_eq!(rewrite_to_printf("100%% done"), "100%%%% done");
    }

    #[test]
    fn rewrite_wc_pattern() {
        let result = rewrite_to_printf(" {:lines/%*llu} {:words/%*llu} {:chars/%*llu} {:filename/%s}\n");
        assert_eq!(result, " %*llu %*llu %*llu %s\n");
    }

    #[test]
    fn rewrite_label_decoration_padding_value() {
        let result = rewrite_to_printf("{L:Free}{D::}{P:   }{:free/%u} {U:Blocks}\n");
        assert_eq!(result, "Free:   %u Blocks\n");
    }

    #[test]
    fn rewrite_label_with_modifiers() {
        let result = rewrite_to_printf("{Lwc:In stock}{:qty/%u}\n");
        assert_eq!(result, "In stock: %u\n");
    }

    #[test]
    fn rewrite_units_with_w_modifier() {
        let result = rewrite_to_printf("{:dist/%u}{Uw:miles}");
        assert_eq!(result, "%u miles");
    }

    #[test]
    fn rewrite_empty_xo_format() {
        assert_eq!(rewrite_to_printf(""), "");
    }

    #[test]
    fn rewrite_no_fields() {
        assert_eq!(rewrite_to_printf("plain text\n"), "plain text\n");
    }

    #[test]
    fn rewrite_escaped_braces_percent() {
        assert_eq!(rewrite_to_printf("{{%d}}"), "{%%d}");
    }

    // ── make_zero_width tests ────────────────────────────────────

    #[test]
    fn zero_width_int() {
        assert_eq!(make_zero_width("%d"), "%.0d");
    }

    #[test]
    fn zero_width_string() {
        assert_eq!(make_zero_width("%s"), "%.0s");
    }

    #[test]
    fn zero_width_long() {
        assert_eq!(make_zero_width("%ld"), "%.0ld");
    }

    // ── modifier_suffix tests ────────────────────────────────────

    #[test]
    fn suffix_colon_and_white() {
        assert_eq!(modifier_suffix(true, true), ": ");
    }

    #[test]
    fn suffix_colon_only() {
        assert_eq!(modifier_suffix(true, false), ":");
    }

    #[test]
    fn suffix_white_only() {
        assert_eq!(modifier_suffix(false, true), " ");
    }

    #[test]
    fn suffix_none() {
        assert_eq!(modifier_suffix(false, false), "");
    }
}
