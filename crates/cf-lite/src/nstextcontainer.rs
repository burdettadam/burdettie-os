//! NSTextContainer stub — defines the region where text is laid out.
//!
//! Object layout: 32 bytes
//!   [0..8):   ISA pointer
//!   [8..16):  layout_manager_ptr
//!   [16..24): width  (f64, container size width)
//!   [24..32): height (f64, container size height)

const SIZE: usize = 32;
const LAYOUT_MANAGER_OFF: usize = 8;
const WIDTH_OFF: usize = 16;
const HEIGHT_OFF: usize = 24;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as *const u8).add(offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, val: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as *mut u8).add(offset) as *mut u64) = val;
    }
}

/// `+[NSTextContainer alloc]`
pub extern "C" fn shim_nstextcontainer_alloc(_cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nstextcontainer_class_addr();
    }
    obj
}

/// `-[NSTextContainer init]`
pub extern "C" fn shim_nstextcontainer_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    // Default: very large container.
    write_u64(obj, WIDTH_OFF, f64::to_bits(1.0e7));
    write_u64(obj, HEIGHT_OFF, f64::to_bits(1.0e7));
    obj
}

/// `-[NSTextContainer initWithSize:]` (w, h as separate f64 args)
pub extern "C" fn shim_nstextcontainer_init_with_size(obj: u64, _cmd: u64, w: f64, h: f64) -> u64 {
    if obj == 0 {
        return 0;
    }
    write_u64(obj, WIDTH_OFF, f64::to_bits(w));
    write_u64(obj, HEIGHT_OFF, f64::to_bits(h));
    obj
}

/// `-[NSTextContainer size]` → returns container width (simplified to f64 return)
pub extern "C" fn shim_nstextcontainer_size_width(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    f64::from_bits(read_u64(obj, WIDTH_OFF))
}

/// `-[NSTextContainer setSize:]` (w, h)
pub extern "C" fn shim_nstextcontainer_set_size(obj: u64, _cmd: u64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, WIDTH_OFF, f64::to_bits(w));
    write_u64(obj, HEIGHT_OFF, f64::to_bits(h));
}

/// `-[NSTextContainer layoutManager]`
pub extern "C" fn shim_nstextcontainer_layout_manager(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    read_u64(obj, LAYOUT_MANAGER_OFF)
}

/// `-[NSTextContainer setLayoutManager:]`
pub extern "C" fn shim_nstextcontainer_set_layout_manager(obj: u64, _cmd: u64, lm: u64) {
    if obj == 0 {
        return;
    }
    write_u64(obj, LAYOUT_MANAGER_OFF, lm);
}

/// `-[NSTextContainer lineFragmentPadding]` → 5.0 (Apple default)
pub extern "C" fn shim_nstextcontainer_line_fragment_padding(_obj: u64, _cmd: u64) -> f64 {
    5.0
}

/// `-[NSTextContainer setLineFragmentPadding:]` → no-op
pub extern "C" fn shim_nstextcontainer_set_line_fragment_padding(
    _obj: u64,
    _cmd: u64,
    _padding: f64,
) {
}

/// `-[NSTextContainer widthTracksTextView]` → 1 (YES)
pub extern "C" fn shim_nstextcontainer_width_tracks_text_view(_obj: u64, _cmd: u64) -> u64 {
    1
}

/// `-[NSTextContainer setWidthTracksTextView:]` → no-op
pub extern "C" fn shim_nstextcontainer_set_width_tracks(_obj: u64, _cmd: u64, _flag: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    #[test]
    fn alloc_init_defaults() {
        let tc = shim_nstextcontainer_alloc(0, 0);
        let tc = shim_nstextcontainer_init(tc, 0);
        assert_ne!(tc, 0);
        assert_eq!(read_u64(tc, 0), crate::nsobj::nstextcontainer_class_addr());
        // Default container is very large.
        assert!(shim_nstextcontainer_size_width(tc, 0) > 1000.0);
        free_obj(tc);
    }

    #[test]
    fn init_with_size() {
        let tc = shim_nstextcontainer_alloc(0, 0);
        let tc = shim_nstextcontainer_init_with_size(tc, 0, 300.0, 200.0);
        assert!((shim_nstextcontainer_size_width(tc, 0) - 300.0).abs() < 0.001);
        free_obj(tc);
    }

    #[test]
    fn layout_manager_round_trip() {
        let tc = shim_nstextcontainer_alloc(0, 0);
        let tc = shim_nstextcontainer_init(tc, 0);
        assert_eq!(shim_nstextcontainer_layout_manager(tc, 0), 0);
        shim_nstextcontainer_set_layout_manager(tc, 0, 0xCAFE);
        assert_eq!(shim_nstextcontainer_layout_manager(tc, 0), 0xCAFE);
        free_obj(tc);
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nstextcontainer_init(0, 0), 0);
        assert_eq!(shim_nstextcontainer_size_width(0, 0), 0.0);
        assert_eq!(shim_nstextcontainer_layout_manager(0, 0), 0);
    }
}
