use std::collections::HashMap;

use objc_runtime::ClassInfo;

use super::me;
use crate::{nscalendar, nsdate, nsdateformatter, nslocale, nsobj, nstimezone};

/// Register `NSDate` with date creation and comparison methods.
pub fn register_nsdate_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdate_class_addr();
    let metaclass_addr = nsobj::nsdate_metaclass_addr();

    let info = ClassInfo {
        name: "NSDate".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "timeIntervalSince1970",
                nsdate::shim_nsdate_ti_since_1970 as *const () as u64,
                "d@:",
            ),
            me(
                "timeIntervalSinceReferenceDate",
                nsdate::shim_nsdate_ti_since_ref as *const () as u64,
                "d@:",
            ),
            me(
                "timeIntervalSinceNow",
                nsdate::shim_nsdate_ti_since_now as *const () as u64,
                "d@:",
            ),
            me(
                "timeIntervalSinceDate:",
                nsdate::shim_nsdate_ti_since_date as *const () as u64,
                "d@:@",
            ),
            me(
                "dateByAddingTimeInterval:",
                nsdate::shim_nsdate_date_by_adding as *const () as u64,
                "@@:d",
            ),
            me(
                "isEqualToDate:",
                nsdate::shim_nsdate_is_equal_to_date as *const () as u64,
                "B@:@",
            ),
            me("compare:", nsdate::shim_nsdate_compare as *const () as u64, "q@:@"),
            me("description", nsdate::shim_nsdate_description as *const () as u64, "@@:"),
            me(
                "earlierDate:",
                nsdate::shim_nsdate_earlier_date as *const () as u64,
                "@@:@",
            ),
            me(
                "laterDate:",
                nsdate::shim_nsdate_later_date as *const () as u64,
                "@@:@",
            ),
        ],
        class_methods: vec![
            me("date", nsdate::shim_nsdate_date as *const () as u64, "@@:"),
            me(
                "dateWithTimeIntervalSince1970:",
                nsdate::shim_nsdate_date_with_ti_since_1970 as *const () as u64,
                "@@:d",
            ),
            me(
                "dateWithTimeIntervalSinceReferenceDate:",
                nsdate::shim_nsdate_date_with_ti_since_ref as *const () as u64,
                "@@:d",
            ),
            me(
                "distantFuture",
                nsdate::shim_nsdate_distant_future as *const () as u64,
                "@@:",
            ),
            me(
                "distantPast",
                nsdate::shim_nsdate_distant_past as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDate");
}

/// Register `NSDateFormatter` with date formatting methods.
pub fn register_nsdateformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdateformatter_class_addr();
    let metaclass_addr = nsobj::nsdateformatter_metaclass_addr();

    let info = ClassInfo {
        name: "NSDateFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "init",
                nsdateformatter::shim_nsdateformatter_init as *const () as u64,
                "@@:",
            ),
            me(
                "setDateFormat:",
                nsdateformatter::shim_nsdateformatter_set_date_format as *const () as u64,
                "v@:@",
            ),
            me(
                "dateFormat",
                nsdateformatter::shim_nsdateformatter_date_format as *const () as u64,
                "@@:",
            ),
            me(
                "setDateStyle:",
                nsdateformatter::shim_nsdateformatter_set_date_style as *const () as u64,
                "v@:Q",
            ),
            me(
                "dateStyle",
                nsdateformatter::shim_nsdateformatter_date_style as *const () as u64,
                "Q@:",
            ),
            me(
                "setTimeStyle:",
                nsdateformatter::shim_nsdateformatter_set_time_style as *const () as u64,
                "v@:Q",
            ),
            me(
                "timeStyle",
                nsdateformatter::shim_nsdateformatter_time_style as *const () as u64,
                "Q@:",
            ),
            me(
                "stringFromDate:",
                nsdateformatter::shim_nsdateformatter_string_from_date as *const () as u64,
                "@@:@",
            ),
            me(
                "dateFromString:",
                nsdateformatter::shim_nsdateformatter_date_from_string as *const () as u64,
                "@@:@",
            ),
            me(
                "locale",
                nsdateformatter::shim_nsdateformatter_locale as *const () as u64,
                "@@:",
            ),
            me(
                "setLocale:",
                nsdateformatter::shim_nsdateformatter_set_locale as *const () as u64,
                "v@:@",
            ),
            me(
                "calendar",
                nsdateformatter::shim_nsdateformatter_calendar as *const () as u64,
                "@@:",
            ),
            me(
                "setCalendar:",
                nsdateformatter::shim_nsdateformatter_set_calendar as *const () as u64,
                "v@:@",
            ),
            me(
                "timeZone",
                nsdateformatter::shim_nsdateformatter_time_zone as *const () as u64,
                "@@:",
            ),
            me(
                "setTimeZone:",
                nsdateformatter::shim_nsdateformatter_set_time_zone as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 32,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDateFormatter");
}

/// Register `NSTimeZone` with timezone query methods.
pub fn register_nstimezone_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nstimezone_class_addr();
    let metaclass_addr = nsobj::nstimezone_metaclass_addr();

    let info = ClassInfo {
        name: "NSTimeZone".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("name", nstimezone::shim_nstimezone_name as *const () as u64, "@@:"),
            me(
                "abbreviation",
                nstimezone::shim_nstimezone_abbreviation as *const () as u64,
                "@@:",
            ),
            me(
                "secondsFromGMT",
                nstimezone::shim_nstimezone_seconds_from_gmt as *const () as u64,
                "q@:",
            ),
            me(
                "description",
                nstimezone::shim_nstimezone_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "systemTimeZone",
                nstimezone::shim_nstimezone_system_timezone as *const () as u64,
                "@@:",
            ),
            me(
                "localTimeZone",
                nstimezone::shim_nstimezone_local_timezone as *const () as u64,
                "@@:",
            ),
            me(
                "timeZoneWithName:",
                nstimezone::shim_nstimezone_with_name as *const () as u64,
                "@@:@",
            ),
            me(
                "timeZoneWithAbbreviation:",
                nstimezone::shim_nstimezone_with_abbreviation as *const () as u64,
                "@@:@",
            ),
            me(
                "timeZoneForSecondsFromGMT:",
                nstimezone::shim_nstimezone_for_seconds_from_gmt as *const () as u64,
                "@@:q",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSTimeZone");
}

/// Register `NSCalendar` with calendar computation methods.
pub fn register_nscalendar_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nscalendar_class_addr();
    let metaclass_addr = nsobj::nscalendar_metaclass_addr();

    let info = ClassInfo {
        name: "NSCalendar".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "calendarIdentifier",
                nscalendar::shim_nscalendar_identifier as *const () as u64,
                "@@:",
            ),
            me(
                "components:fromDate:",
                nscalendar::shim_nscalendar_components_from_date as *const () as u64,
                "@@:Q@",
            ),
            me(
                "component:fromDate:",
                nscalendar::shim_nscalendar_component_from_date as *const () as u64,
                "q@:Q@",
            ),
            me(
                "dateFromComponents:",
                nscalendar::shim_nscalendar_date_from_components as *const () as u64,
                "@@:@",
            ),
            me(
                "dateByAddingComponents:toDate:options:",
                nscalendar::shim_nscalendar_date_by_adding_components as *const () as u64,
                "@@:@@Q",
            ),
            me(
                "description",
                nscalendar::shim_nscalendar_description as *const () as u64,
                "@@:",
            ),
            me(
                "locale",
                nscalendar::shim_nscalendar_locale as *const () as u64,
                "@@:",
            ),
            me(
                "setLocale:",
                nscalendar::shim_nscalendar_set_locale as *const () as u64,
                "v@:@",
            ),
        ],
        class_methods: vec![
            me(
                "currentCalendar",
                nscalendar::shim_nscalendar_current_calendar as *const () as u64,
                "@@:",
            ),
            me(
                "calendarWithIdentifier:",
                nscalendar::shim_nscalendar_with_identifier as *const () as u64,
                "@@:@",
            ),
            me(
                "autoupdatingCurrentCalendar",
                nscalendar::shim_nscalendar_auto_updating_current_calendar as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 16,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSCalendar");
}

/// Register `NSLocale` with locale information methods.
pub fn register_nslocale_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nslocale_class_addr();
    let metaclass_addr = nsobj::nslocale_metaclass_addr();

    let info = ClassInfo {
        name: "NSLocale".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me(
                "localeIdentifier",
                nslocale::shim_nslocale_locale_identifier as *const () as u64,
                "@@:",
            ),
            me(
                "languageCode",
                nslocale::shim_nslocale_language_code as *const () as u64,
                "@@:",
            ),
            me(
                "countryCode",
                nslocale::shim_nslocale_country_code as *const () as u64,
                "@@:",
            ),
            me(
                "objectForKey:",
                nslocale::shim_nslocale_object_for_key as *const () as u64,
                "@@:@",
            ),
            me(
                "displayNameForKey:value:",
                nslocale::shim_nslocale_display_name_for_key_value as *const () as u64,
                "@@:@@",
            ),
            me(
                "description",
                nslocale::shim_nslocale_description as *const () as u64,
                "@@:",
            ),
        ],
        class_methods: vec![
            me(
                "currentLocale",
                nslocale::shim_nslocale_current_locale as *const () as u64,
                "@@:",
            ),
            me(
                "systemLocale",
                nslocale::shim_nslocale_system_locale as *const () as u64,
                "@@:",
            ),
            me(
                "preferredLanguages",
                nslocale::shim_nslocale_preferred_languages as *const () as u64,
                "@@:",
            ),
        ],
        ivars: vec![],
        properties: vec![],
        protocol_names: Vec::new(),
        instance_size: 24,
        is_root: false,
        is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(),
        class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSLocale");
}
