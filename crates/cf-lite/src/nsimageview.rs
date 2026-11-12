//! NSImageView shim: image display control for AppKit.
//!
//! Object layout (inherits NSControl at offsets 0-119):
//!
//!   offset   0: NSControl fields
//!   offset 120: image_ptr  (u64, NSImage*)
//!   offset 128: flags      (u64, bit 0: allows_cut_copy_paste,
//!                                bit 1: animates,
//!                                bit 2: is_editable)
//!
//! Total: 136 bytes.

const IMAGE_OFF: usize = crate::nscontrol::CONTROL_SIZE;
const FLAGS_OFF: usize = IMAGE_OFF + 8;
pub(crate) const IMAGE_VIEW_SIZE: usize = FLAGS_OFF + 8;

const FLAG_ALLOWS_CUT_COPY_PASTE: u64 = 1 << 0;
const FLAG_ANIMATES: u64 = 1 << 1;
const FLAG_IS_EDITABLE: u64 = 1 << 2;

const IMAGE_ALIGNMENT_CENTER: u64 = 0;
const IMAGE_SCALING_PROPORTIONALLY_DOWN: u64 = 1;

fn read_u64(obj: u64, offset: usize) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + offset) as *const u64) }
}

fn write_u64(obj: u64, offset: usize, value: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + offset) as *mut u64) = value;
    }
}

fn read_flags(obj: u64) -> u64 {
    read_u64(obj, FLAGS_OFF)
}

fn write_flags(obj: u64, flags: u64) {
    write_u64(obj, FLAGS_OFF, flags);
}

fn set_flag(obj: u64, flag: u64, val: bool) {
    if obj == 0 {
        return;
    }
    let mut flags = read_flags(obj);
    if val {
        flags |= flag;
    } else {
        flags &= !flag;
    }
    write_flags(obj, flags);
}

fn get_flag(obj: u64, flag: u64) -> u8 {
    if obj == 0 {
        return 0;
    }
    u8::from(read_flags(obj) & flag != 0)
}

fn resolve_archived_image(candidate: u64) -> u64 {
    if candidate == 0 {
        return 0;
    }
    if !candidate.is_multiple_of(8) {
        return 0;
    }

    if unsafe { *(candidate as *const u64) } == crate::nsobj::nsimage_class_addr() {
        return candidate;
    }

    if crate::cfstring::extract_cfstring_cstr(candidate).is_some() {
        return crate::nsimage::shim_nsimage_image_named(0, 0, candidate);
    }

    0
}

/// `-[NSImageView initWithCoder:]`.
pub extern "C" fn shim_nsimageview_init_with_coder(obj: u64, _cmd: u64, coder: u64) -> u64 {
    if obj == 0 {
        return 0;
    }

    crate::nsview::shim_nsview_init_with_coder(obj, 0, coder);
    crate::nscontrol::init_control_defaults(obj);

    let archived_image = crate::nskeyedarchiver::decode_object_for_any_key(
        coder,
        &[
            "NSImage",
            "NS.image",
            "image",
            "_image",
            "NSImageName",
            "imageName",
        ],
    );
    let image = resolve_archived_image(archived_image);
    if image != 0 {
        shim_nsimageview_set_image(obj, 0, image);
    }

    if let Some(value) = crate::nskeyedarchiver::decode_bool_for_any_key(
        coder,
        &[
            "NSAllowsCutCopyPaste",
            "allowsCutCopyPaste",
            "_allowsCutCopyPaste",
        ],
    ) {
        shim_nsimageview_set_allows_cut_copy_paste(obj, 0, u8::from(value));
    }

    if let Some(value) =
        crate::nskeyedarchiver::decode_bool_for_any_key(coder, &["NSAnimates", "animates"])
    {
        shim_nsimageview_set_animates(obj, 0, u8::from(value));
    }

    if let Some(value) = crate::nskeyedarchiver::decode_bool_for_any_key(
        coder,
        &["NSEditable", "editable", "isEditable", "_editable"],
    ) {
        shim_nsimageview_set_editable(obj, 0, u8::from(value));
    }

    obj
}

/// `-[NSImageView image]`
pub extern "C" fn shim_nsimageview_image(obj: u64, _cmd: u64) -> u64 {
    read_u64(obj, IMAGE_OFF)
}

/// `-[NSImageView setImage:]`
pub extern "C" fn shim_nsimageview_set_image(obj: u64, _cmd: u64, image: u64) {
    write_u64(obj, IMAGE_OFF, image);
}

/// `-[NSImageView allowsCutCopyPaste]`
pub extern "C" fn shim_nsimageview_allows_cut_copy_paste(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_ALLOWS_CUT_COPY_PASTE)
}

/// `-[NSImageView setAllowsCutCopyPaste:]`
pub extern "C" fn shim_nsimageview_set_allows_cut_copy_paste(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_ALLOWS_CUT_COPY_PASTE, val != 0);
}

/// `-[NSImageView animates]`
pub extern "C" fn shim_nsimageview_animates(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_ANIMATES)
}

/// `-[NSImageView setAnimates:]`
pub extern "C" fn shim_nsimageview_set_animates(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_ANIMATES, val != 0);
}

/// `-[NSImageView isEditable]`
pub extern "C" fn shim_nsimageview_is_editable(obj: u64, _cmd: u64) -> u8 {
    get_flag(obj, FLAG_IS_EDITABLE)
}

/// `-[NSImageView setEditable:]`
pub extern "C" fn shim_nsimageview_set_editable(obj: u64, _cmd: u64, val: u8) {
    set_flag(obj, FLAG_IS_EDITABLE, val != 0);
}

/// `-[NSImageView imageAlignment]` — stub returns center alignment.
pub extern "C" fn shim_nsimageview_image_alignment(_obj: u64, _cmd: u64) -> u64 {
    IMAGE_ALIGNMENT_CENTER
}

/// `-[NSImageView setImageAlignment:]` — no-op stub.
pub extern "C" fn shim_nsimageview_set_image_alignment(_obj: u64, _cmd: u64, _alignment: u64) {}

/// `-[NSImageView imageScaling]` — stub returns proportional-down scaling.
pub extern "C" fn shim_nsimageview_image_scaling(_obj: u64, _cmd: u64) -> u64 {
    IMAGE_SCALING_PROPORTIONALLY_DOWN
}

/// `-[NSImageView setImageScaling:]` — no-op stub.
pub extern "C" fn shim_nsimageview_set_image_scaling(_obj: u64, _cmd: u64, _scaling: u64) {}

/// `-[NSImageView drawRect:]` — renders the image via CGContextDrawImage.
pub extern "C" fn shim_nsimageview_draw_rect(obj: u64, _cmd: u64, x: f64, y: f64, w: f64, h: f64) {
    if obj == 0 {
        return;
    }
    let cg_ctx = crate::nsgraphicscontext::current_cg_context_handle();
    if cg_ctx == 0 {
        return;
    }

    let image_ptr = read_u64(obj, IMAGE_OFF);
    if image_ptr == 0 {
        return;
    }

    // Get the CGImage handle from our NSImage.
    // NSImage stores image info in IMAGE_STORE; try to get a CG handle.
    let cg_image = crate::nsimage::image_to_cg_handle(image_ptr);
    if cg_image != 0 {
        cg_shims::bridge::draw_image_in_context(cg_ctx, cg_image, x, y, w, h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_image_view() -> u64 {
        alloc_obj_fake(IMAGE_VIEW_SIZE)
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsimageview_image(0, 0), 0);
        shim_nsimageview_set_image(0, 0, 0x1234);

        assert_eq!(shim_nsimageview_allows_cut_copy_paste(0, 0), 0);
        shim_nsimageview_set_allows_cut_copy_paste(0, 0, 1);

        assert_eq!(shim_nsimageview_animates(0, 0), 0);
        shim_nsimageview_set_animates(0, 0, 1);

        assert_eq!(shim_nsimageview_is_editable(0, 0), 0);
        shim_nsimageview_set_editable(0, 0, 1);

        assert_eq!(
            shim_nsimageview_image_alignment(0, 0),
            IMAGE_ALIGNMENT_CENTER
        );
        shim_nsimageview_set_image_alignment(0, 0, 99);
        assert_eq!(
            shim_nsimageview_image_scaling(0, 0),
            IMAGE_SCALING_PROPORTIONALLY_DOWN
        );
        shim_nsimageview_set_image_scaling(0, 0, 99);
    }

    #[test]
    fn image_round_trip() {
        let view = make_image_view();
        let image = 0xABCD_EF01_u64;

        assert_eq!(shim_nsimageview_image(view, 0), 0);
        shim_nsimageview_set_image(view, 0, image);
        assert_eq!(shim_nsimageview_image(view, 0), image);
        assert_eq!(read_field(view, IMAGE_OFF), image);

        free_obj(view);
    }

    #[test]
    fn image_can_be_cleared_back_to_nil() {
        let view = make_image_view();

        shim_nsimageview_set_image(view, 0, 0xBEEF);
        assert_eq!(shim_nsimageview_image(view, 0), 0xBEEF);
        shim_nsimageview_set_image(view, 0, 0);
        assert_eq!(shim_nsimageview_image(view, 0), 0);
        assert_eq!(read_field(view, IMAGE_OFF), 0);

        free_obj(view);
    }

    #[test]
    fn flags_default_to_disabled() {
        let view = make_image_view();

        assert_eq!(shim_nsimageview_allows_cut_copy_paste(view, 0), 0);
        assert_eq!(shim_nsimageview_animates(view, 0), 0);
        assert_eq!(shim_nsimageview_is_editable(view, 0), 0);
        assert_eq!(read_field(view, FLAGS_OFF), 0);

        free_obj(view);
    }

    #[test]
    fn flags_round_trip_independently() {
        let view = make_image_view();

        shim_nsimageview_set_allows_cut_copy_paste(view, 0, 1);
        assert_eq!(shim_nsimageview_allows_cut_copy_paste(view, 0), 1);
        assert_eq!(shim_nsimageview_animates(view, 0), 0);
        assert_eq!(shim_nsimageview_is_editable(view, 0), 0);

        shim_nsimageview_set_animates(view, 0, 1);
        assert_eq!(shim_nsimageview_allows_cut_copy_paste(view, 0), 1);
        assert_eq!(shim_nsimageview_animates(view, 0), 1);
        assert_eq!(shim_nsimageview_is_editable(view, 0), 0);

        shim_nsimageview_set_editable(view, 0, 1);
        assert_eq!(shim_nsimageview_allows_cut_copy_paste(view, 0), 1);
        assert_eq!(shim_nsimageview_animates(view, 0), 1);
        assert_eq!(shim_nsimageview_is_editable(view, 0), 1);
        assert_eq!(
            read_field(view, FLAGS_OFF),
            FLAG_ALLOWS_CUT_COPY_PASTE | FLAG_ANIMATES | FLAG_IS_EDITABLE
        );

        shim_nsimageview_set_animates(view, 0, 0);
        assert_eq!(shim_nsimageview_allows_cut_copy_paste(view, 0), 1);
        assert_eq!(shim_nsimageview_animates(view, 0), 0);
        assert_eq!(shim_nsimageview_is_editable(view, 0), 1);
        assert_eq!(
            read_field(view, FLAGS_OFF),
            FLAG_ALLOWS_CUT_COPY_PASTE | FLAG_IS_EDITABLE
        );

        free_obj(view);
    }

    #[test]
    fn flag_getters_ignore_unrelated_bits() {
        let view = make_image_view();
        write_field(view, FLAGS_OFF, (1_u64 << 63) | FLAG_ANIMATES);

        assert_eq!(shim_nsimageview_allows_cut_copy_paste(view, 0), 0);
        assert_eq!(shim_nsimageview_animates(view, 0), 1);
        assert_eq!(shim_nsimageview_is_editable(view, 0), 0);

        shim_nsimageview_set_editable(view, 0, 1);
        assert_eq!(
            read_field(view, FLAGS_OFF),
            (1_u64 << 63) | FLAG_ANIMATES | FLAG_IS_EDITABLE
        );

        free_obj(view);
    }

    #[test]
    fn alignment_and_scaling_are_constant_stubs() {
        let view = make_image_view();
        shim_nsimageview_set_image_alignment(view, 0, 7);
        shim_nsimageview_set_image_scaling(view, 0, 42);

        assert_eq!(
            shim_nsimageview_image_alignment(view, 0),
            IMAGE_ALIGNMENT_CENTER
        );
        assert_eq!(
            shim_nsimageview_image_scaling(view, 0),
            IMAGE_SCALING_PROPORTIONALLY_DOWN
        );
        assert_eq!(read_field(view, IMAGE_OFF), 0);
        assert_eq!(read_field(view, FLAGS_OFF), 0);

        free_obj(view);
    }
}
