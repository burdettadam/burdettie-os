//! NSXMLParser shim: SAX-style XML parsing backed by `quick-xml`.
//!
//! Object layout (40 bytes):
//!   offset  0: isa              (u64)
//!   offset  8: data_ptr         (u64 — NSData containing XML)
//!   offset 16: delegate_ptr     (u64 — id<NSXMLParserDelegate>)
//!   offset 24: error_ptr        (u64 — NSError, 0=nil)
//!   offset 32: flags            (u64 — bit 0: shouldProcessNamespaces,
//!                                       bit 1: shouldReportNamespacePrefixes)

use crate::cfstring;
use crate::nsobj;

use quick_xml::events::Event;
use quick_xml::Reader;

const DATA_OFF: usize = 8;
const DELEGATE_OFF: usize = 16;
const ERROR_OFF: usize = 24;
const FLAGS_OFF: usize = 32;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_parser() -> u64 {
    let obj = crate::pal_alloc().calloc(1, 40) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = nsobj::nsxmlparser_class_addr();
    }
    obj
}

// ── Initializers ───────────────────────────────────────────────

/// `+[NSXMLParser alloc]`
pub extern "C" fn shim_nsxmlparser_alloc(_cls: u64, _cmd: u64) -> u64 {
    alloc_parser()
}

/// `-[NSXMLParser initWithData:]`
pub extern "C" fn shim_nsxmlparser_init_with_data(obj: u64, _cmd: u64, data: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe {
        *((obj as usize + DATA_OFF) as *mut u64) = data;
    }
    obj
}

/// `-[NSXMLParser initWithContentsOfURL:]`
pub extern "C" fn shim_nsxmlparser_init_with_url(obj: u64, _cmd: u64, url: u64) -> u64 {
    if obj == 0 || url == 0 {
        return 0;
    }
    // Read URL path to NSData
    let path_str = crate::nsurl::shim_nsurl_path(url, 0);
    if path_str == 0 {
        return obj;
    }
    let Some(p) = cfstring::extract_cfstring_cstr(path_str) else {
        return obj;
    };
    let Ok(bytes) = std::fs::read(p) else {
        return obj;
    };
    let data = crate::nsdata::shim_nsdata_data_with_bytes_length(
        0,
        0,
        bytes.as_ptr() as u64,
        bytes.len() as u64,
    );
    unsafe {
        *((obj as usize + DATA_OFF) as *mut u64) = data;
    }
    obj
}

// ── Delegate ───────────────────────────────────────────────────

/// `-[NSXMLParser setDelegate:]`
pub extern "C" fn shim_nsxmlparser_set_delegate(obj: u64, _cmd: u64, delegate: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + DELEGATE_OFF) as *mut u64) = delegate;
    }
}

/// `-[NSXMLParser delegate]`
pub extern "C" fn shim_nsxmlparser_delegate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) }
}

// ── Parsing ────────────────────────────────────────────────────

/// Parsed XML token (SAX event).
struct XmlToken {
    kind: TokenKind,
    name: String,
    text: String,
    /// Attribute key-value pairs.
    attrs: Vec<(String, String)>,
}

enum TokenKind {
    StartElement,
    EndElement,
    Characters,
}

/// SAX tokenizer powered by `quick-xml`.
///
/// Handles elements, self-closing elements, attributes, CDATA, comments,
/// processing instructions, and entity resolution.
fn tokenize_xml(xml: &str) -> Vec<XmlToken> {
    let mut reader = Reader::from_str(xml);
    let mut tokens = Vec::new();
    let mut raw_text = String::new();

    loop {
        let event = reader.read_event();
        // quick-xml emits entity references separately from text. Reassemble
        // each text run before trimming and unescaping so spaces around an
        // entity and references such as &amp;lt; retain their original meaning.
        if !matches!(&event, Ok(Event::Text(_) | Event::GeneralRef(_))) && !raw_text.is_empty() {
            let text = quick_xml::escape::unescape(
                raw_text.trim_matches(|c| matches!(c, ' ' | '\t' | '\r' | '\n')),
            )
            .map(|text| text.into_owned())
            .unwrap_or_default();
            raw_text.clear();
            if !text.is_empty() {
                tokens.push(XmlToken {
                    kind: TokenKind::Characters,
                    name: String::new(),
                    text,
                    attrs: vec![],
                });
            }
        }
        match event {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = extract_attrs(e);
                tokens.push(XmlToken {
                    kind: TokenKind::StartElement,
                    name,
                    text: String::new(),
                    attrs,
                });
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                tokens.push(XmlToken {
                    kind: TokenKind::EndElement,
                    name,
                    text: String::new(),
                    attrs: vec![],
                });
            }
            Ok(Event::Empty(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = extract_attrs(e);
                tokens.push(XmlToken {
                    kind: TokenKind::StartElement,
                    name: name.clone(),
                    text: String::new(),
                    attrs,
                });
                tokens.push(XmlToken {
                    kind: TokenKind::EndElement,
                    name,
                    text: String::new(),
                    attrs: vec![],
                });
            }
            Ok(Event::Text(ref e)) => {
                raw_text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(Event::GeneralRef(ref e)) => {
                raw_text.push('&');
                raw_text.push_str(&e.decode().unwrap_or_default());
                raw_text.push(';');
            }
            Ok(Event::CData(ref e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                if !text.is_empty() {
                    tokens.push(XmlToken {
                        kind: TokenKind::Characters,
                        name: String::new(),
                        text,
                        attrs: vec![],
                    });
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {} // Skip comments, PI, doctype, decl
            Err(e) => {
                log::warn!("NSXMLParser: quick-xml error: {e}");
                break;
            }
        }
    }
    tokens
}

/// Extract attribute key-value pairs from a quick-xml element.
fn extract_attrs(e: &quick_xml::events::BytesStart<'_>) -> Vec<(String, String)> {
    e.attributes()
        .filter_map(|a| a.ok())
        .map(|a| {
            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
            let val = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .map(|v| v.to_string())
                .unwrap_or_default();
            (key, val)
        })
        .collect()
}

// ── Delegate dispatch ──────────────────────────────────────────

extern "C" {
    fn objc_msg_lookup_imp(obj: u64, sel: u64) -> u64;
    fn objc_has_method(obj: u64, sel: u64) -> u8;
}

/// Look up IMP for `sel_str` on `delegate` and return it, or 0 if not found.
fn delegate_imp(delegate: u64, sel_str: &str) -> u64 {
    let Ok(csel) = std::ffi::CString::new(sel_str) else {
        return 0;
    };
    // First check that the delegate actually has this method registered.
    // objc_msg_lookup_imp returns forwarding_trampoline (non-zero) for unknown
    // selectors on known classes, which would throw "unrecognized selector".
    // objc_has_method only returns 1 if the method is in the runtime's table.
    if unsafe { objc_has_method(delegate, csel.as_ptr() as u64) } == 0 {
        return 0;
    }
    unsafe { objc_msg_lookup_imp(delegate, csel.as_ptr() as u64) }
}

/// Build a temporary NSDictionary from attribute pairs.
fn make_attrs_dict(attrs: &[(String, String)]) -> u64 {
    if attrs.is_empty() {
        return 0;
    }
    let dict = crate::nsmutabledict::shim_nsmutabledict_dictionary(0, 0);
    if dict == 0 {
        return 0;
    }
    for (k, v) in attrs {
        let k_ns = make_nsstring(k);
        let v_ns = make_nsstring(v);
        crate::nsmutabledict::shim_nsmutabledict_set_object_for_key(dict, 0, v_ns, k_ns);
    }
    dict
}

/// `-[NSXMLParser parse]` — parses the XML and sends delegate callbacks.
///
/// Returns 1 (YES) on success, 0 (NO) on failure.
pub extern "C" fn shim_nsxmlparser_parse(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let data_ptr = unsafe { *((obj as usize + DATA_OFF) as *const u64) };
    if data_ptr == 0 {
        return 0;
    }
    let delegate = unsafe { *((obj as usize + DELEGATE_OFF) as *const u64) };

    let length = crate::nsdata::shim_nsdata_length(data_ptr, 0);
    if length == 0 {
        return 1;
    }
    let bytes_ptr = crate::nsdata::shim_nsdata_bytes(data_ptr, 0);
    let xml = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            bytes_ptr as *const u8,
            length as usize,
        ))
    };
    let xml = match xml {
        Ok(s) => s.to_owned(),
        Err(_) => return 0,
    };

    let tokens = tokenize_xml(&xml);

    if delegate != 0 {
        let imp = delegate_imp(delegate, "parserDidStartDocument:");
        if imp != 0 {
            let csel = std::ffi::CString::new("parserDidStartDocument:").unwrap();
            let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
            f(delegate, csel.as_ptr() as u64, obj);
        }
    }

    for token in &tokens {
        if delegate == 0 {
            continue;
        }
        match token.kind {
            TokenKind::StartElement => {
                let imp = delegate_imp(
                    delegate,
                    "parser:didStartElement:namespaceURI:qualifiedName:attributes:",
                );
                if imp != 0 {
                    let csel = std::ffi::CString::new(
                        "parser:didStartElement:namespaceURI:qualifiedName:attributes:",
                    )
                    .unwrap();
                    let name_ns = make_nsstring(&token.name);
                    let attrs_dict = make_attrs_dict(&token.attrs);
                    // (delegate, cmd, parser, localName, nsURI, qualName, attrs)
                    let f: extern "C" fn(u64, u64, u64, u64, u64, u64, u64) =
                        unsafe { std::mem::transmute(imp) };
                    f(
                        delegate,
                        csel.as_ptr() as u64,
                        obj,
                        name_ns,
                        0,
                        name_ns,
                        attrs_dict,
                    );
                }
            }
            TokenKind::EndElement => {
                let imp =
                    delegate_imp(delegate, "parser:didEndElement:namespaceURI:qualifiedName:");
                if imp != 0 {
                    let csel =
                        std::ffi::CString::new("parser:didEndElement:namespaceURI:qualifiedName:")
                            .unwrap();
                    let name_ns = make_nsstring(&token.name);
                    let f: extern "C" fn(u64, u64, u64, u64, u64, u64) =
                        unsafe { std::mem::transmute(imp) };
                    f(delegate, csel.as_ptr() as u64, obj, name_ns, 0, name_ns);
                }
            }
            TokenKind::Characters => {
                let imp = delegate_imp(delegate, "parser:foundCharacters:");
                if imp != 0 {
                    let csel = std::ffi::CString::new("parser:foundCharacters:").unwrap();
                    let text_ns = make_nsstring(&token.text);
                    let f: extern "C" fn(u64, u64, u64, u64) = unsafe { std::mem::transmute(imp) };
                    f(delegate, csel.as_ptr() as u64, obj, text_ns);
                }
            }
        }
    }

    if delegate != 0 {
        let imp = delegate_imp(delegate, "parserDidEndDocument:");
        if imp != 0 {
            let csel = std::ffi::CString::new("parserDidEndDocument:").unwrap();
            let f: extern "C" fn(u64, u64, u64) = unsafe { std::mem::transmute(imp) };
            f(delegate, csel.as_ptr() as u64, obj);
        }
    }

    1
}

/// `-[NSXMLParser parserError]`
pub extern "C" fn shim_nsxmlparser_parser_error(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + ERROR_OFF) as *const u64) }
}

/// `-[NSXMLParser abortParsing]`
pub extern "C" fn shim_nsxmlparser_abort_parsing(_obj: u64, _cmd: u64) {
    log::debug!("NSXMLParser abortParsing (stub)");
}

/// `-[NSXMLParser setShouldProcessNamespaces:]`
pub extern "C" fn shim_nsxmlparser_set_should_process_ns(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let flags_ptr = (obj as usize + FLAGS_OFF) as *mut u64;
    unsafe {
        let mut flags = *flags_ptr;
        if val != 0 {
            flags |= 1;
        } else {
            flags &= !1;
        }
        *flags_ptr = flags;
    }
}

/// `-[NSXMLParser setShouldReportNamespacePrefixes:]`
pub extern "C" fn shim_nsxmlparser_set_should_report_ns(obj: u64, _cmd: u64, val: u8) {
    if obj == 0 {
        return;
    }
    let flags_ptr = (obj as usize + FLAGS_OFF) as *mut u64;
    unsafe {
        let mut flags = *flags_ptr;
        if val != 0 {
            flags |= 2;
        } else {
            flags &= !2;
        }
        *flags_ptr = flags;
    }
}

/// `-[NSXMLParser shouldProcessNamespaces]`
pub extern "C" fn shim_nsxmlparser_should_process_ns(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = unsafe { *((obj as usize + FLAGS_OFF) as *const u64) };
    u8::from(flags & 1 != 0)
}

/// `-[NSXMLParser shouldReportNamespacePrefixes]`
pub extern "C" fn shim_nsxmlparser_should_report_ns(obj: u64, _cmd: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    let flags = unsafe { *((obj as usize + FLAGS_OFF) as *const u64) };
    u8::from(flags & 2 != 0)
}

pub extern "C" fn shim_nsxmlparser_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    make_nsstring("<NSXMLParser>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn tokenizer_preserves_entity_text_and_attributes() {
        let tokens = tokenize_xml(
            "<root label=\"Alice &amp; Bob\"> Alice &amp; Bob &#65; &#x1F600; &amp;lt; </root>",
        );
        assert_eq!(
            tokens[0].attrs,
            vec![("label".into(), "Alice & Bob".into())]
        );
        let text: String = tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Characters))
            .map(|token| token.text.as_str())
            .collect();
        assert_eq!(text, "Alice & Bob A 😀 &lt;");
    }

    #[test]
    fn tokenizer_preserves_xml10_attribute_normalization() {
        let tokens = tokenize_xml(
            "<root literal=\"a\tb\r\nc\" escaped=\"&#x9;&#xA;&#xD;\" nested=\"&amp;lt;\"/>",
        );
        assert_eq!(
            tokens[0].attrs,
            vec![
                ("literal".into(), "a b c".into()),
                ("escaped".into(), "\t\n\r".into()),
                ("nested".into(), "&lt;".into()),
            ]
        );
    }

    #[test]
    fn tokenizer_keeps_cdata_literal_and_element_boundaries() {
        let tokens = tokenize_xml("<root> left <child/> right <![CDATA[ &amp; literal ]]></root>");
        let text: Vec<&str> = tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Characters))
            .map(|token| token.text.as_str())
            .collect();
        assert_eq!(text, vec!["left", "right", " &amp; literal "]);
        assert_eq!(
            tokens.iter().filter(|token| token.name == "child").count(),
            2
        );
    }

    #[test]
    fn alloc_sets_correct_isa() {
        let p = shim_nsxmlparser_alloc(0, 0);
        assert_ne!(p, 0);
        assert_eq!(read_field(p, 0), crate::nsobj::nsxmlparser_class_addr());
        free_obj(p);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsxmlparser_parse(0, 0), 0);
        assert_eq!(shim_nsxmlparser_delegate(0, 0), 0);
        assert_eq!(shim_nsxmlparser_parser_error(0, 0), 0);
        assert_eq!(shim_nsxmlparser_description(0, 0), 0);
        shim_nsxmlparser_set_delegate(0, 0, 0);
    }

    #[test]
    fn init_with_nil_obj_returns_zero() {
        assert_eq!(shim_nsxmlparser_init_with_data(0, 0, 0), 0);
    }

    #[test]
    fn init_with_data_stores_data() {
        let bytes = b"<root/>";
        let data = crate::nsdata::make_nsdata(bytes);
        let p = shim_nsxmlparser_alloc(0, 0);
        let p2 = shim_nsxmlparser_init_with_data(p, 0, data);
        assert_eq!(p2, p);
        // parse succeeds with no delegate
        assert_eq!(shim_nsxmlparser_parse(p, 0), 1);
        free_obj(p);
    }

    #[test]
    fn parse_with_no_data_returns_zero() {
        let p = shim_nsxmlparser_alloc(0, 0);
        // no data stored — parse should return 0
        assert_eq!(shim_nsxmlparser_parse(p, 0), 0);
        free_obj(p);
    }

    #[test]
    fn delegate_round_trip() {
        let p = shim_nsxmlparser_alloc(0, 0);
        assert_eq!(shim_nsxmlparser_delegate(p, 0), 0);
        shim_nsxmlparser_set_delegate(p, 0, 0xBEEF);
        assert_eq!(shim_nsxmlparser_delegate(p, 0), 0xBEEF);
        free_obj(p);
    }

    #[test]
    fn description_returns_expected_string() {
        let p = shim_nsxmlparser_alloc(0, 0);
        let d = shim_nsxmlparser_description(p, 0);
        assert_ne!(d, 0);
        assert_string_eq(d, "<NSXMLParser>");
        free_obj(p);
    }

    #[test]
    fn nsxml_parser_namespace_flag_stored() {
        let p = shim_nsxmlparser_alloc(0, 0);
        // Default is NO
        assert_eq!(shim_nsxmlparser_should_process_ns(p, 0), 0);
        assert_eq!(shim_nsxmlparser_should_report_ns(p, 0), 0);
        // Set shouldProcessNamespaces
        shim_nsxmlparser_set_should_process_ns(p, 0, 1);
        assert_eq!(shim_nsxmlparser_should_process_ns(p, 0), 1);
        assert_eq!(shim_nsxmlparser_should_report_ns(p, 0), 0);
        // Set shouldReportNamespacePrefixes
        shim_nsxmlparser_set_should_report_ns(p, 0, 1);
        assert_eq!(shim_nsxmlparser_should_report_ns(p, 0), 1);
        assert_eq!(shim_nsxmlparser_should_process_ns(p, 0), 1);
        // Clear shouldProcessNamespaces
        shim_nsxmlparser_set_should_process_ns(p, 0, 0);
        assert_eq!(shim_nsxmlparser_should_process_ns(p, 0), 0);
        assert_eq!(shim_nsxmlparser_should_report_ns(p, 0), 1);
        free_obj(p);
    }

    #[test]
    fn namespace_flags_nil_safety() {
        shim_nsxmlparser_set_should_process_ns(0, 0, 1);
        shim_nsxmlparser_set_should_report_ns(0, 0, 1);
        assert_eq!(shim_nsxmlparser_should_process_ns(0, 0), 0);
        assert_eq!(shim_nsxmlparser_should_report_ns(0, 0), 0);
    }
}
