//! Tests for VmProt, Section flags, SectionType variants, TlvDescriptor,
//! and zerofill detection.
#![allow(clippy::identity_op)]

use macho_types::segment::*;

// ── VmProt (from mach/vm_prot.h) ────────────────────────────────

#[test]
fn vmprot_read_write_execute_values() {
    assert_eq!(VmProt::READ.bits(), 0x01); // VM_PROT_READ
    assert_eq!(VmProt::WRITE.bits(), 0x02); // VM_PROT_WRITE
    assert_eq!(VmProt::EXECUTE.bits(), 0x04); // VM_PROT_EXECUTE
}

#[test]
fn vmprot_text_segment() {
    // __TEXT is typically r-x
    let prot = VmProt::READ | VmProt::EXECUTE;
    assert!(prot.contains(VmProt::READ));
    assert!(!prot.contains(VmProt::WRITE));
    assert!(prot.contains(VmProt::EXECUTE));
    assert_eq!(format!("{prot}"), "r-x");
}

#[test]
fn vmprot_data_segment() {
    // __DATA is typically rw-
    let prot = VmProt::READ | VmProt::WRITE;
    assert_eq!(format!("{prot}"), "rw-");
}

#[test]
fn vmprot_no_access() {
    // __PAGEZERO has no protections
    let prot = VmProt::empty();
    assert_eq!(format!("{prot}"), "---");
}

#[test]
fn vmprot_linkedit_segment() {
    // __LINKEDIT is typically r--
    let prot = VmProt::READ;
    assert_eq!(format!("{prot}"), "r--");
}

#[test]
fn vmprot_serde_roundtrip() {
    let original = VmProt::READ | VmProt::EXECUTE;
    let json = serde_json::to_string(&original).unwrap();
    let restored: VmProt = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

// ── Section flags ───────────────────────────────────────────────

#[test]
fn section_type_extraction() {
    let s = Section {
        sectname: "__bss".into(),
        segname: "__DATA".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x01, // S_ZEROFILL
        reserved1: 0,
        reserved2: 0,
    };
    assert_eq!(s.section_type(), 0x01);
    assert!(s.is_zerofill());
}

#[test]
fn section_regular_not_zerofill() {
    let s = Section {
        sectname: "__text".into(),
        segname: "__TEXT".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x00, // S_REGULAR
        reserved1: 0,
        reserved2: 0,
    };
    assert_eq!(s.section_type(), 0x00);
    assert!(!s.is_zerofill());
}

#[test]
fn section_gb_zerofill() {
    let s = Section {
        sectname: "__huge".into(),
        segname: "__DATA".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x0C, // S_GB_ZEROFILL
        reserved1: 0,
        reserved2: 0,
    };
    assert!(s.is_zerofill());
}

// ── SectionType tests ───────────────────────────────────────────

#[test]
fn section_type_regular() {
    assert_eq!(SectionType::from(0x00), SectionType::Regular);
}

#[test]
fn section_type_zerofill() {
    assert_eq!(SectionType::from(0x01), SectionType::ZeroFill);
}

#[test]
fn section_type_mod_init_func() {
    assert_eq!(SectionType::from(0x09), SectionType::ModInitFuncPointers);
}

#[test]
fn section_type_thread_local_variables() {
    assert_eq!(SectionType::from(0x13), SectionType::ThreadLocalVariables);
}

#[test]
fn section_type_init_offsets() {
    assert_eq!(SectionType::from(0x16), SectionType::InitOffsets);
}

#[test]
fn section_type_unknown() {
    assert_eq!(SectionType::from(0xFF), SectionType::Unknown(0xFF));
}

#[test]
fn section_type_name_on_section() {
    let section = Section {
        sectname: "__text".into(),
        segname: "__TEXT".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x00, // S_REGULAR
        reserved1: 0,
        reserved2: 0,
    };
    assert_eq!(section.section_type_name(), SectionType::Regular);
}

// ── Section reserved fields ─────────────────────────────────────

#[test]
fn section_has_reserved_fields() {
    let s = Section {
        sectname: "__stubs".into(),
        segname: "__TEXT".into(),
        addr: 0x1000,
        size: 24,
        offset: 0x1000,
        align: 2,
        reloff: 0,
        nreloc: 0,
        flags: 0x80000408, // S_SYMBOL_STUBS | S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS
        reserved1: 5,      // indirect sym table index
        reserved2: 12,     // stub size
    };
    assert_eq!(s.reserved1, 5);
    assert_eq!(s.reserved2, 12);
}

#[test]
fn section_reserved_defaults_zero() {
    let s = Section {
        sectname: "__text".into(),
        segname: "__TEXT".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0,
        reserved1: 0,
        reserved2: 0,
    };
    assert_eq!(s.reserved1, 0);
    assert_eq!(s.reserved2, 0);
}

// ── TlvDescriptor struct ────────────────────────────────────────

#[test]
fn tlv_descriptor_default_values() {
    let d = macho_types::TlvDescriptor {
        address: 0x1000,
        thunk: 0x2000,
        key: 0,
        offset: 48,
    };
    assert_eq!(d.address, 0x1000);
    assert_eq!(d.thunk, 0x2000);
    assert_eq!(d.key, 0);
    assert_eq!(d.offset, 48);
}

#[test]
fn tlv_descriptor_serializes_to_json() {
    let d = macho_types::TlvDescriptor {
        address: 0x100,
        thunk: 0x200,
        key: 1,
        offset: 8,
    };
    let json = serde_json::to_string(&d).unwrap();
    assert!(json.contains("\"address\""));
    assert!(json.contains("\"thunk\""));
    assert!(json.contains("\"key\""));
    assert!(json.contains("\"offset\""));
}

// ── SectionType TLV variants ────────────────────────────────────

#[test]
fn section_type_thread_local_regular() {
    assert_eq!(SectionType::from(0x11), SectionType::ThreadLocalRegular);
}

#[test]
fn section_type_thread_local_zerofill() {
    assert_eq!(SectionType::from(0x12), SectionType::ThreadLocalZeroFill);
}

#[test]
fn section_type_thread_local_variables_from_u8() {
    assert_eq!(SectionType::from(0x13), SectionType::ThreadLocalVariables);
}

#[test]
fn section_type_thread_local_variable_pointers() {
    assert_eq!(
        SectionType::from(0x14),
        SectionType::ThreadLocalVariablePointers
    );
}

#[test]
fn section_type_thread_local_init_function_pointers() {
    assert_eq!(
        SectionType::from(0x15),
        SectionType::ThreadLocalInitFunctionPointers
    );
}

#[test]
fn section_flags_detect_tlv_type() {
    let s = Section {
        sectname: "__thread_vars".into(),
        segname: "__DATA".into(),
        addr: 0,
        size: 48,
        offset: 0,
        align: 3,
        reloff: 0,
        nreloc: 0,
        flags: 0x13, // S_THREAD_LOCAL_VARIABLES
        reserved1: 0,
        reserved2: 0,
    };
    assert_eq!(s.section_type_name(), SectionType::ThreadLocalVariables);
}

// ── is_zerofill includes ThreadLocalZeroFill ────────────────────

#[test]
fn is_zerofill_regular_zerofill() {
    let s = Section {
        sectname: "".into(),
        segname: "".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x01, // S_ZEROFILL
        reserved1: 0,
        reserved2: 0,
    };
    assert!(s.is_zerofill());
}

#[test]
fn is_zerofill_gb_zerofill() {
    let s = Section {
        sectname: "".into(),
        segname: "".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x0C, // S_GB_ZEROFILL
        reserved1: 0,
        reserved2: 0,
    };
    assert!(s.is_zerofill());
}

#[test]
fn is_zerofill_thread_local_zerofill() {
    let s = Section {
        sectname: "__thread_bss".into(),
        segname: "__DATA".into(),
        addr: 0,
        size: 4,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x12, // S_THREAD_LOCAL_ZEROFILL
        reserved1: 0,
        reserved2: 0,
    };
    assert!(
        s.is_zerofill(),
        "ThreadLocalZeroFill should be treated as zerofill"
    );
}

#[test]
fn is_zerofill_false_for_regular() {
    let s = Section {
        sectname: "".into(),
        segname: "".into(),
        addr: 0,
        size: 0,
        offset: 0,
        align: 0,
        reloff: 0,
        nreloc: 0,
        flags: 0x00, // S_REGULAR
        reserved1: 0,
        reserved2: 0,
    };
    assert!(!s.is_zerofill());
}
