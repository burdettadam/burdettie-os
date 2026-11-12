use super::*;

fn synthetic_class(name: &str, addr: u64, superclass_addr: Option<u64>) -> ClassInfo {
    ClassInfo {
        name: name.to_string(),
        mapped_addr: addr,
        methods: vec![],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: vec![],
        instance_size: 8,
        is_root: superclass_addr.is_none(),
        is_meta: false,
        superclass_addr,
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    }
}

fn image_class(name: &str, addr: u64) -> ObjcMetadata {
    let mut metadata = make_metadata(name, vec![]);
    metadata.class_refs[0].class_data_addr = addr;
    metadata
}

#[test]
fn repairs_image_superclasses_after_fixups_including_null() {
    let mut record = Box::new([0_u64, 0x4000, 0, 0, 0]);
    let addr = record.as_ptr() as u64;
    let mut rt = ObjcRuntime::new();
    rt.register_image(&image_class("Guest", addr), 0);

    // SAFETY: The record is live and contains the complete class layout.
    unsafe { rt.repair_superclass_addrs() };
    assert_eq!(rt.get_class("Guest").unwrap().superclass_addr, Some(0x4000));

    record[1] = 0;
    // SAFETY: The same record remains live after its superclass fixup changes.
    unsafe { rt.repair_superclass_addrs() };
    assert_eq!(rt.get_class("Guest").unwrap().superclass_addr, None);
}

#[test]
fn repair_preserves_synthetic_hierarchies_and_opaque_handles() {
    let mut rt = ObjcRuntime::new();
    // Runtime-created classes use opaque IDs, which cannot be dereferenced.
    rt.register_synthetic_class(synthetic_class("Dynamic", 1, Some(0x4000)));
    // Even when a synthetic class has storage, its supplied hierarchy wins.
    let record = Box::new([0_u64; 5]);
    rt.register_synthetic_class(synthetic_class(
        "Synthetic",
        record.as_ptr() as u64,
        Some(0x5000),
    ));

    // SAFETY: No image-backed records are registered.
    unsafe { rt.repair_superclass_addrs() };
    assert_eq!(
        rt.get_class("Dynamic").unwrap().superclass_addr,
        Some(0x4000)
    );
    assert_eq!(
        rt.get_class("Synthetic").unwrap().superclass_addr,
        Some(0x5000)
    );
}

#[test]
fn repair_reads_unaligned_export_backed_records() {
    // An exported class can lack parsed class metadata. Its byte layout still
    // needs post-fixup repair, without assuming a Rust allocation alignment.
    let mut storage = [0_u64; 6];
    let addr = storage.as_mut_ptr() as u64 + 1;
    // SAFETY: Offsets 1..41 are within the 48-byte backing allocation.
    unsafe { (addr as *mut u64).add(1).write_unaligned(0x6000) };

    let mut rt = ObjcRuntime::new();
    rt.register_mapped_class(synthetic_class("Exported", addr, None));
    // SAFETY: The complete 40-byte record remains live and readable.
    unsafe { rt.repair_superclass_addrs() };
    assert_eq!(
        rt.get_class("Exported").unwrap().superclass_addr,
        Some(0x6000)
    );
}

#[test]
fn replacement_and_overlay_preserve_the_winning_storage_kind() {
    let record = Box::new([0_u64, 0x7000, 0, 0, 0]);
    let metadata = image_class("SharedName", record.as_ptr() as u64);
    let mut image_rt = ObjcRuntime::new();
    image_rt.register_image(&metadata, 0);
    let mut synthetic_rt = ObjcRuntime::new();
    synthetic_rt.register_synthetic_class(synthetic_class("SharedName", 1, Some(0x8000)));

    // Foundation overlays replace mapped classes with synthetic definitions.
    let mut overlay = image_rt.clone();
    overlay.overlay_from(&synthetic_rt);
    // SAFETY: The winning synthetic definition has no mapped record to read.
    unsafe { overlay.repair_superclass_addrs() };
    assert_eq!(
        overlay.get_class("SharedName").unwrap().superclass_addr,
        Some(0x8000)
    );

    // A later image overlay must restore the need for memory-based repair.
    overlay.overlay_from(&image_rt);
    // SAFETY: The winning image's record remains live.
    unsafe { overlay.repair_superclass_addrs() };
    assert_eq!(
        overlay.get_class("SharedName").unwrap().superclass_addr,
        Some(0x7000)
    );

    // Direct registration has the same replacement semantics as an overlay.
    overlay.register_synthetic_class(synthetic_class("SharedName", 1, Some(0x9000)));
    // SAFETY: The replacement is synthetic and has no mapped record to read.
    unsafe { overlay.repair_superclass_addrs() };
    assert_eq!(
        overlay.get_class("SharedName").unwrap().superclass_addr,
        Some(0x9000)
    );
    overlay.register_image(&metadata, 0);
    // SAFETY: Direct image registration restores the live record.
    unsafe { overlay.repair_superclass_addrs() };
    assert_eq!(
        overlay.get_class("SharedName").unwrap().superclass_addr,
        Some(0x7000)
    );
}

#[test]
fn removing_a_mapped_class_drops_its_repair_record() {
    let mut rt = ObjcRuntime::new();
    let addr = {
        let record = Box::new([0_u64; 5]);
        let addr = record.as_ptr() as u64;
        rt.register_image(&image_class("Temporary", addr), 0);
        assert!(rt.remove_class_by_addr(addr));
        addr
    };
    rt.register_synthetic_class(synthetic_class("Temporary", addr, Some(0xA000)));
    // SAFETY: Removal/replacement leaves no mapped records; the released image
    // address is now only a synthetic identity and must not be read.
    unsafe { rt.repair_superclass_addrs() };
    assert_eq!(
        rt.get_class("Temporary").unwrap().superclass_addr,
        Some(0xA000)
    );
}
