//! Extended Foundation class data symbol registrations.

use libsystem_shims::table::ShimTable;

use crate::nsobj;

/// Register a class + metaclass data symbol pair.
macro_rules! register_class {
    ($table:expr, $name:literal, $class_fn:expr, $meta_fn:expr) => {
        $table.register(
            concat!("_OBJC_CLASS_$_", $name),
            $class_fn,
            concat!($name, " class data symbol"),
        );
        $table.register(
            concat!("_OBJC_METACLASS_$_", $name),
            $meta_fn,
            concat!($name, " metaclass data symbol"),
        );
    };
}

/// Register extended Foundation class/metaclass data symbols.
pub(super) fn install_class_symbols(table: &mut ShimTable) {
    register_class!(table, "NSDateFormatter", nsobj::nsdateformatter_class_addr(), nsobj::nsdateformatter_metaclass_addr());
    register_class!(table, "NSCharacterSet", nsobj::nscharacterset_class_addr(), nsobj::nscharacterset_metaclass_addr());
    register_class!(table, "NSNotificationCenter", nsobj::nsnotificationcenter_class_addr(), nsobj::nsnotificationcenter_metaclass_addr());
    register_class!(table, "NSNotification", nsobj::nsnotification_class_addr(), nsobj::nsnotification_metaclass_addr());
    register_class!(table, "NSValue", nsobj::nsvalue_class_addr(), nsobj::nsvalue_metaclass_addr());
    register_class!(table, "NSUUID", nsobj::nsuuid_class_addr(), nsobj::nsuuid_metaclass_addr());
    register_class!(table, "NSLocale", nsobj::nslocale_class_addr(), nsobj::nslocale_metaclass_addr());
    register_class!(table, "NSTimeZone", nsobj::nstimezone_class_addr(), nsobj::nstimezone_metaclass_addr());
    register_class!(table, "NSCalendar", nsobj::nscalendar_class_addr(), nsobj::nscalendar_metaclass_addr());
    register_class!(table, "NSNumberFormatter", nsobj::nsnumberformatter_class_addr(), nsobj::nsnumberformatter_metaclass_addr());
    register_class!(table, "NSIndexSet", nsobj::nsindexset_class_addr(), nsobj::nsindexset_metaclass_addr());
    register_class!(table, "NSMutableIndexSet", nsobj::nsmutableindexset_class_addr(), nsobj::nsmutableindexset_metaclass_addr());
    register_class!(table, "NSSortDescriptor", nsobj::nssortdescriptor_class_addr(), nsobj::nssortdescriptor_metaclass_addr());
    register_class!(table, "NSTimer", nsobj::nstimer_class_addr(), nsobj::nstimer_metaclass_addr());
    register_class!(table, "NSMethodSignature", nsobj::nsmethodsignature_class_addr(), nsobj::nsmethodsignature_metaclass_addr());
    register_class!(table, "NSInvocation", nsobj::nsinvocation_class_addr(), nsobj::nsinvocation_metaclass_addr());
    register_class!(table, "NSRegularExpression", nsobj::nsregularexpression_class_addr(), nsobj::nsregularexpression_metaclass_addr());
    register_class!(table, "NSTextCheckingResult", nsobj::nstextcheckingresult_class_addr(), nsobj::nstextcheckingresult_metaclass_addr());
    register_class!(table, "NSPredicate", nsobj::nspredicate_class_addr(), nsobj::nspredicate_metaclass_addr());
    register_class!(table, "NSAttributedString", nsobj::nsattributedstring_class_addr(), nsobj::nsattributedstring_metaclass_addr());
    register_class!(table, "NSMutableAttributedString", nsobj::nsmutableattributedstring_class_addr(), nsobj::nsmutableattributedstring_metaclass_addr());
    register_class!(table, "NSCache", nsobj::nscache_class_addr(), nsobj::nscache_metaclass_addr());
    register_class!(table, "NSMapTable", nsobj::nsmaptable_class_addr(), nsobj::nsmaptable_metaclass_addr());
    register_class!(table, "NSHashTable", nsobj::nshashtable_class_addr(), nsobj::nshashtable_metaclass_addr());
    register_class!(table, "NSOperation", nsobj::nsoperation_class_addr(), nsobj::nsoperation_metaclass_addr());
    register_class!(table, "NSBlockOperation", nsobj::nsblockoperation_class_addr(), nsobj::nsblockoperation_metaclass_addr());
    register_class!(table, "NSOperationQueue", nsobj::nsoperationqueue_class_addr(), nsobj::nsoperationqueue_metaclass_addr());
    register_class!(table, "NSJSONSerialization", nsobj::nsjsonserialization_class_addr(), nsobj::nsjsonserialization_metaclass_addr());
    register_class!(table, "NSPropertyListSerialization", nsobj::nspropertylistserialization_class_addr(), nsobj::nspropertylistserialization_metaclass_addr());

    // ── AppKit class data symbols (Phase C5) ────────────────────────
    register_class!(table, "NSResponder", nsobj::nsresponder_class_addr(), nsobj::nsresponder_metaclass_addr());
    register_class!(table, "NSApplication", nsobj::nsapplication_class_addr(), nsobj::nsapplication_metaclass_addr());
    register_class!(table, "NSRunLoop", nsobj::nsrunloop_class_addr(), nsobj::nsrunloop_metaclass_addr());
    register_class!(table, "NSWindow", nsobj::nswindow_class_addr(), nsobj::nswindow_metaclass_addr());
    register_class!(table, "NSView", nsobj::nsview_class_addr(), nsobj::nsview_metaclass_addr());
    register_class!(table, "NSEvent", nsobj::nsevent_class_addr(), nsobj::nsevent_metaclass_addr());
    register_class!(table, "NSWorkspace", nsobj::nsworkspace_class_addr(), nsobj::nsworkspace_metaclass_addr());
    register_class!(table, "NSScreen", nsobj::nsscreen_class_addr(), nsobj::nsscreen_metaclass_addr());
    register_class!(table, "NSColor", nsobj::nscolor_class_addr(), nsobj::nscolor_metaclass_addr());
    register_class!(table, "NSFont", nsobj::nsfont_class_addr(), nsobj::nsfont_metaclass_addr());
    register_class!(table, "NSImage", nsobj::nsimage_class_addr(), nsobj::nsimage_metaclass_addr());

    // ── AppKit Phase C7 class data symbols ──────────────────────────
    register_class!(table, "NSAppearance", nsobj::nsappearance_class_addr(), nsobj::nsappearance_metaclass_addr());
    register_class!(table, "NSUserNotificationCenter", nsobj::nsusernotificationcenter_class_addr(), nsobj::nsusernotificationcenter_metaclass_addr());
    register_class!(table, "NSMenu", nsobj::nsmenu_class_addr(), nsobj::nsmenu_metaclass_addr());
    register_class!(table, "NSMenuItem", nsobj::nsmenuitem_class_addr(), nsobj::nsmenuitem_metaclass_addr());
    register_class!(table, "NSNib", nsobj::nsnib_class_addr(), nsobj::nsnib_metaclass_addr());

    // ── Synthetic _OBJC_EHTYPE_$_NSException ──────────────────────────
    // On macOS, @catch(NSException *) uses the personality function
    // __objc_personality_v0, which requires an objc_ehtype_t struct whose
    // vtable == &objc_ehtype_vtable[2].  We create a static struct with
    // the right layout and lazily initialise the vtable + class pointers.
    {
        use std::cell::UnsafeCell;
        use std::sync::Once;

        #[repr(C)]
        struct SyntheticEhtype {
            vtable: u64,
            name: u64,
            cls: u64,
        }
        struct SyncEhtype(UnsafeCell<SyntheticEhtype>);
        unsafe impl Sync for SyncEhtype {}

        static EHTYPE: SyncEhtype = SyncEhtype(UnsafeCell::new(SyntheticEhtype {
            vtable: 0,
            name: 0,
            cls: 0,
        }));
        static EHTYPE_INIT: Once = Once::new();

        EHTYPE_INIT.call_once(|| {
            static NSEXCEPTION_NAME: &[u8] = b"NSException\0";
            let vtable: u64;

            #[cfg(target_os = "macos")]
            {
                let sym = unsafe {
                    libc::dlsym(
                        libc::RTLD_DEFAULT as *mut std::ffi::c_void,
                        c"objc_ehtype_vtable".as_ptr(),
                    )
                };
                vtable = if sym.is_null() {
                    0
                } else {
                    unsafe { (sym as *const *const u8).add(2) as u64 }
                };
            }
            #[cfg(not(target_os = "macos"))]
            {
                vtable = 0;
            }

            unsafe {
                let p = EHTYPE.0.get();
                (*p).vtable = vtable;
                (*p).name = NSEXCEPTION_NAME.as_ptr() as u64;
                (*p).cls = nsobj::nsexception_class_addr();
            }
        });

        table.register(
            "_OBJC_EHTYPE_$_NSException",
            EHTYPE.0.get() as u64,
            "NSException EHTYPE (synthetic, for @catch matching)",
        );
    }

    // ── NSConstant* class symbols (self-hosted) ────────────────────
    // The compiler embeds constant objects (@[], @{}, @(int)) whose ISA
    // points to these symbols.  We give NSConstantDictionary and
    // NSConstantIntegerNumber their own class addresses because their
    // memory layout differs from our dynamic NSDictionary/NSNumber.
    // NSConstantArray happens to share the same layout as our NSArray.
    table.register(
        "_OBJC_CLASS_$_NSConstantArray",
        nsobj::nsarray_class_addr(),
        "NSConstantArray class (same layout as NSArray)",
    );
    table.register(
        "_OBJC_CLASS_$_NSConstantDictionary",
        nsobj::nsconstantdictionary_class_addr(),
        "NSConstantDictionary class (40-byte constant layout)",
    );
    table.register(
        "_OBJC_CLASS_$_NSConstantIntegerNumber",
        nsobj::nsconstantintegernumber_class_addr(),
        "NSConstantIntegerNumber class (offset 16 = value)",
    );
}
