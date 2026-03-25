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

    // NSException EHTYPE — used by __objc_personality_v0 for @catch type matching.
    #[cfg(target_os = "macos")]
    {
        let symbols = crate::pal_symbols();
        let fnd_path = "/System/Library/Frameworks/Foundation.framework/Foundation";
        if let Some(handle) = symbols.open_library(fnd_path) {
            if let Some(sym) = symbols.resolve_in(handle, "OBJC_EHTYPE_$_NSException") {
                table.register(
                    "_OBJC_EHTYPE_$_NSException",
                    sym as u64,
                    "NSException EHTYPE (host Foundation, for @catch matching)",
                );
            }

            for (sym_name, desc) in [
                ("OBJC_CLASS_$_NSConstantArray", "NSConstantArray class (host, for @[] literals)"),
                ("OBJC_CLASS_$_NSConstantDictionary", "NSConstantDictionary class (host, for @{} literals)"),
                ("OBJC_CLASS_$_NSConstantIntegerNumber", "NSConstantIntegerNumber class (host, for @(int) literals)"),
            ] {
                if let Some(addr) = symbols.resolve_in(handle, sym_name) {
                    table.register(
                        &format!("_{sym_name}"),
                        addr as u64,
                        desc,
                    );
                }
            }
        }
    }
}
