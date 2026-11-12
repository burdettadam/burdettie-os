//! Swift runtime stubs for type metadata lookup, conformance checking,
//! and object allocation.
//!
//! These provide safe no-crash behaviour for guest Swift binaries that
//! call into the Swift runtime for type metadata operations.  Instead of
//! returning NULL (which causes immediate SIGSEGVs in generated code),
//! we return pointers to plausible static metadata structs.
//!
//! ## Swift TypeMetadata ABI layout
//!
//! ```text
//!   metadata_ptr - 8:  Value Witness Table pointer (VWT)
//!   metadata_ptr + 0:  kind  (0=class, 1=struct, 2=enum, 8=opaque, 12=tuple)
//!   metadata_ptr + 8:  description pointer (nominal type descriptor)
//! ```
//!
//! We provide a single static struct-kind metadata that satisfies the
//! minimum ABI requirements for code that inspects metadata.

use crate::table::ShimTable;

// ── Static dummy metadata ──────────────────────────────────────────

/// Minimal Value Witness Table (VWT) for an empty struct.
///
/// The VWT is a table of function pointers and size/alignment info.
/// We provide a 17-entry table (136 bytes on 64-bit) matching the
/// stable ABI layout.  All function pointers are 0 (unused) except
/// for size/stride/alignment (fields 8-10).
///
/// Layout indices (from Swift ABI docs):
///   [0]  initializeBufferWithCopyOfBuffer
///   [1]  destroy
///   [2]  initializeWithCopy
///   [3]  assignWithCopy
///   [4]  initializeWithTake
///   [5]  assignWithTake
///   [6]  getEnumTagSinglePayload
///   [7]  storeEnumTagSinglePayload
///   [8]  size           → 0 (empty struct)
///   [9]  stride         → 1 (minimum stride)
///   [10] flags          → 0x00010007 (alignment mask=7, POD, bitwiseTakable, inline)
///   [11..16] extra witnesses → 0
#[repr(C)]
struct DummyVwt {
    entries: [u64; 17],
}

static DUMMY_VWT: DummyVwt = DummyVwt {
    entries: [
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,           // function pointers (all null — no ops needed)
        0,           // size
        1,           // stride (must be ≥1)
        0x0001_0007, // flags: alignment mask=7, POD, bitwiseTakable, inline storage
        0,
        0,
        0,
        0,
        0,
        0, // extra witnesses
    ],
};

/// Dummy nominal type descriptor (minimal).
///
/// Swift code that reads the descriptor will find a plausible header.
/// Fields: kind (struct=1), name (null), access_fn (null), fieldDescriptor (null).
#[repr(C)]
struct DummyNominalDescriptor {
    flags: u32,            // kind = struct (1 << 0)
    parent: i32,           // relative offset to parent descriptor (0 = none)
    name_offset: i32,      // relative offset to name string (0 = null)
    access_fn_offset: i32, // relative offset to metadata access fn (0)
    field_descriptor: i32, // relative offset to field descriptor (0)
}

static DUMMY_DESCRIPTOR: DummyNominalDescriptor = DummyNominalDescriptor {
    flags: 0x0000_0001, // StructDescriptor kind
    parent: 0,
    name_offset: 0,
    access_fn_offset: 0,
    field_descriptor: 0,
};

/// The metadata struct itself: VWT lives at [-1] relative to the
/// "metadata pointer" that Swift code receives.  We lay it out as:
///
///   [0]: VWT pointer     (metadata_ptr - 8 from caller's perspective)
///   [1]: kind            (metadata_ptr + 0 → this is what callers get)
///   [2]: descriptor ptr  (metadata_ptr + 8)
///
/// Callers receive a pointer to `entries[1]`.
#[repr(C)]
struct DummyMetadata {
    vwt_ptr: u64,        // points to DUMMY_VWT
    kind: u64,           // 1 = struct
    descriptor_ptr: u64, // points to DUMMY_DESCRIPTOR
}

// We need a const-initializable metadata that points to our statics.
// Since we can't take &raw const in a static initializer (stable),
// we use a lazy OnceLock to set the pointers once.
static METADATA_STORAGE: std::sync::OnceLock<MetadataStorage> = std::sync::OnceLock::new();

struct MetadataStorage {
    /// 3-word storage: [vwt_ptr, kind=1, descriptor_ptr]
    words: [u64; 3],
}

// SAFETY: MetadataStorage is read-only after init and only contains u64 values.
unsafe impl Send for MetadataStorage {}
unsafe impl Sync for MetadataStorage {}

/// Returns the metadata pointer (points to word[1], the `kind` field).
fn dummy_metadata_ptr() -> u64 {
    let storage = METADATA_STORAGE.get_or_init(|| {
        MetadataStorage {
            words: [
                &DUMMY_VWT as *const DummyVwt as u64,
                1, // kind = struct
                &DUMMY_DESCRIPTOR as *const DummyNominalDescriptor as u64,
            ],
        }
    });
    // Callers expect a pointer to the `kind` field (word[1])
    &storage.words[1] as *const u64 as u64
}

// ── Shim functions ─────────────────────────────────────────────────

/// `swift_getTypeByMangledNameInContext` → returns static struct metadata.
///
/// Signature: `(mangledName: *const u8, nameLength: u32,
///              genericContext: *const void, genericArgs: *const *const void) -> *const Metadata`
extern "C" fn shim_swift_get_type_by_mangled_name(
    _name: u64,
    _len: u32,
    _ctx: u64,
    _args: u64,
) -> u64 {
    dummy_metadata_ptr()
}

/// `swift_getTypeByMangledNameInEnvironment` → returns static struct metadata.
extern "C" fn shim_swift_get_type_by_mangled_name_env(
    _name: u64,
    _len: u32,
    _env: u64,
    _args: u64,
) -> u64 {
    dummy_metadata_ptr()
}

/// `swift_conformsToProtocol` → returns NULL (no conformance).
///
/// Returning NULL means "type does not conform to protocol" which is
/// a valid, handleable result for Swift code that checks via `is` or `as?`.
extern "C" fn shim_swift_conforms_to_protocol(_metadata: u64, _protocol: u64) -> u64 {
    0
}

/// `swift_getExistentialTypeMetadata` → returns static metadata.
extern "C" fn shim_swift_get_existential_type_metadata(
    _kind: u64,
    _count: u64,
    _protocols: u64,
    _superclass: u64,
) -> u64 {
    dummy_metadata_ptr()
}

/// `swift_getTupleTypeMetadata` → returns static metadata.
extern "C" fn shim_swift_get_tuple_type_metadata(
    _request: u64,
    _flags: u64,
    _elements: u64,
    _labels: u64,
    _proposal: u64,
) -> u64 {
    dummy_metadata_ptr()
}

/// `swift_getOpaqueTypeMetadata` → returns static metadata.
extern "C" fn shim_swift_get_opaque_type_metadata(
    _request: u64,
    _arguments: u64,
    _descriptor: u64,
    _index: u64,
) -> u64 {
    dummy_metadata_ptr()
}

/// `swift_checkMetadataState` → returns (metadata, 0=complete).
///
/// Swift passes `MetadataRequest` and expects `MetadataResponse`.
/// We return the metadata as-is with state = 0 (Complete).
extern "C" fn shim_swift_check_metadata_state(_request: u64, metadata: u64) -> u64 {
    if metadata == 0 {
        return dummy_metadata_ptr();
    }
    metadata
}

/// `swift_getTypeContextDescriptor` → returns our dummy descriptor.
extern "C" fn shim_swift_get_type_context_descriptor(_metadata: u64) -> u64 {
    &DUMMY_DESCRIPTOR as *const DummyNominalDescriptor as u64
}

// ── Registration ───────────────────────────────────────────────────

pub struct SwiftTypeMetadataStubs;

impl SwiftTypeMetadataStubs {
    pub fn install(table: &mut ShimTable) {
        // Force-init the metadata storage so the pointer is stable
        let _ = dummy_metadata_ptr();

        table.register(
            "_swift_getTypeByMangledNameInContext",
            shim_swift_get_type_by_mangled_name as *const () as u64,
            "swift-runtime: getTypeByMangledNameInContext → static struct metadata",
        );
        table.register(
            "_swift_getTypeByMangledNameInContext2",
            shim_swift_get_type_by_mangled_name as *const () as u64,
            "swift-runtime: getTypeByMangledNameInContext2 → static struct metadata",
        );
        table.register(
            "_swift_getTypeByMangledNameInEnvironment",
            shim_swift_get_type_by_mangled_name_env as *const () as u64,
            "swift-runtime: getTypeByMangledNameInEnvironment → static struct metadata",
        );
        table.register(
            "_swift_conformsToProtocol",
            shim_swift_conforms_to_protocol as *const () as u64,
            "swift-runtime: conformsToProtocol → NULL (no conformance)",
        );
        table.register(
            "_swift_conformsToProtocolCommon",
            shim_swift_conforms_to_protocol as *const () as u64,
            "swift-runtime: conformsToProtocolCommon → NULL (no conformance)",
        );
        table.register(
            "_swift_getExistentialTypeMetadata",
            shim_swift_get_existential_type_metadata as *const () as u64,
            "swift-runtime: getExistentialTypeMetadata → static metadata",
        );
        table.register(
            "_swift_getTupleTypeMetadata",
            shim_swift_get_tuple_type_metadata as *const () as u64,
            "swift-runtime: getTupleTypeMetadata → static metadata",
        );
        table.register(
            "_swift_getOpaqueTypeMetadata",
            shim_swift_get_opaque_type_metadata as *const () as u64,
            "swift-runtime: getOpaqueTypeMetadata → static metadata",
        );
        table.register(
            "_swift_checkMetadataState",
            shim_swift_check_metadata_state as *const () as u64,
            "swift-runtime: checkMetadataState → complete",
        );
        table.register(
            "_swift_getTypeContextDescriptor",
            shim_swift_get_type_context_descriptor as *const () as u64,
            "swift-runtime: getTypeContextDescriptor → dummy descriptor",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_metadata_has_valid_kind() {
        let ptr = dummy_metadata_ptr();
        assert_ne!(ptr, 0);
        // The kind field should be 1 (struct)
        let kind = unsafe { *(ptr as *const u64) };
        assert_eq!(kind, 1);
    }

    #[test]
    fn dummy_metadata_has_vwt_at_minus_one() {
        let ptr = dummy_metadata_ptr();
        // VWT pointer lives at metadata_ptr - 8
        let vwt_ptr = unsafe { *((ptr - 8) as *const u64) };
        assert_ne!(vwt_ptr, 0);
        // The VWT's stride (index 9) should be 1
        let stride = unsafe { *((vwt_ptr as usize + 9 * 8) as *const u64) };
        assert_eq!(stride, 1);
    }

    #[test]
    fn dummy_metadata_has_descriptor() {
        let ptr = dummy_metadata_ptr();
        // Descriptor pointer at metadata_ptr + 8
        let desc_ptr = unsafe { *((ptr as usize + 8) as *const u64) };
        assert_ne!(desc_ptr, 0);
        // First word of descriptor is flags (struct kind = 1)
        let flags = unsafe { *(desc_ptr as *const u32) };
        assert_eq!(flags, 1);
    }

    #[test]
    fn get_type_by_mangled_name_returns_valid() {
        let result = shim_swift_get_type_by_mangled_name(0, 0, 0, 0);
        assert_eq!(result, dummy_metadata_ptr());
    }

    #[test]
    fn conforms_to_protocol_returns_null() {
        assert_eq!(shim_swift_conforms_to_protocol(0, 0), 0);
    }

    #[test]
    fn check_metadata_state_passthrough() {
        let md = dummy_metadata_ptr();
        assert_eq!(shim_swift_check_metadata_state(0, md), md);
        // Null metadata gets replaced with dummy
        assert_ne!(shim_swift_check_metadata_state(0, 0), 0);
    }
}
