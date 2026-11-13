//! C-string extraction from Mach-O sections.
//!
//! Extracts null-terminated strings from `__cstring`, `__oslogstring`,
//! `__objc_methnames`, and similar sections.

use crate::ParsedMachO;

/// A set of C-strings extracted from a single section.
#[derive(Debug, Clone)]
pub struct SectionStrings {
    /// Segment name (e.g. "__TEXT").
    pub segment: String,
    /// Section name (e.g. "__cstring").
    pub section: String,
    /// Extracted strings.
    pub strings: Vec<String>,
}

/// Section types that contain C-strings (S_CSTRING_LITERALS = 0x02).
const S_CSTRING_LITERALS: u32 = 0x02;

/// Well-known section names that contain C-strings regardless of section type.
const CSTRING_SECTION_NAMES: &[&str] = &[
    "__cstring",
    "__oslogstring",
    "__objc_methnames",
    "__objc_classname",
    "__objc_methtype",
];

/// Extract C-strings from all cstring-type sections in the binary.
pub fn extract_cstrings(parsed: &ParsedMachO, data: &[u8]) -> Vec<SectionStrings> {
    let mut results = Vec::new();

    for section in &parsed.sections {
        let section_type = section.flags & 0xFF;
        let is_cstring_section = section_type == S_CSTRING_LITERALS
            || CSTRING_SECTION_NAMES.contains(&section.sectname.as_str());

        if !is_cstring_section {
            continue;
        }

        let Some(section_data) = crate::bytes_at(data, u64::from(section.offset), section.size)
        else {
            continue;
        };
        let strings = extract_null_terminated(section_data);

        if !strings.is_empty() {
            results.push(SectionStrings {
                segment: section.segname.clone(),
                section: section.sectname.clone(),
                strings,
            });
        }
    }

    results
}

/// Split a byte slice on null bytes, extracting valid UTF-8 strings.
fn extract_null_terminated(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut start = 0;

    for (i, &byte) in data.iter().enumerate() {
        if byte == 0 {
            if i > start {
                if let Ok(s) = std::str::from_utf8(&data[start..i]) {
                    if !s.is_empty() {
                        strings.push(s.to_string());
                    }
                }
            }
            start = i + 1;
        }
    }

    strings
}
