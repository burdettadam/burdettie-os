//! NSUnit, NSDimension, NSMeasurement, and concrete unit class registrations.

use std::collections::HashMap;
use objc_runtime::ClassInfo;
use super::me;
use crate::{nsmeasurement, nsobj};

pub fn register_nsunit_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsunit_class_addr();
    let metaclass_addr = nsobj::nsunit_metaclass_addr();
    let info = ClassInfo {
        name: "NSUnit".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("symbol", nsmeasurement::shim_nsunit_symbol as u64, "@@:"),
            me("initWithSymbol:", nsmeasurement::shim_nsunit_init_with_symbol as u64, "@@:@"),
            me("description", nsmeasurement::shim_nsunit_description as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsmeasurement::NSUNIT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSUnit");
}

pub fn register_nsdimension_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsdimension_class_addr();
    let metaclass_addr = nsobj::nsdimension_metaclass_addr();
    let info = ClassInfo {
        name: "NSDimension".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("symbol", nsmeasurement::shim_nsunit_symbol as u64, "@@:"),
            me("initWithSymbol:", nsmeasurement::shim_nsdimension_init_with_symbol as u64, "@@:@"),
            me("description", nsmeasurement::shim_nsunit_description as u64, "@@:"),
            me("baseUnit", nsmeasurement::shim_nsdimension_base_unit as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsmeasurement::NSUNIT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsunit_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSDimension");
}

pub fn register_nsmeasurement_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmeasurement_class_addr();
    let metaclass_addr = nsobj::nsmeasurement_metaclass_addr();
    let info = ClassInfo {
        name: "NSMeasurement".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("doubleValue", nsmeasurement::shim_nsmeasurement_double_value as u64, "d@:"),
            me("unit", nsmeasurement::shim_nsmeasurement_unit as u64, "@@:"),
            me("initWithDoubleValue:unit:", nsmeasurement::shim_nsmeasurement_init_with_double_value_unit as u64, "@@:d@"),
            me("measurementByConvertingToUnit:", nsmeasurement::shim_nsmeasurement_measurement_by_converting_to_unit as u64, "@@:@"),
            me("isEqualToMeasurement:", nsmeasurement::shim_nsmeasurement_is_equal_to_measurement as u64, "B@:@"),
            me("description", nsmeasurement::shim_nsmeasurement_description as u64, "@@:"),
        ],
        class_methods: vec![],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsmeasurement::NSMEASUREMENT_SIZE as u32, is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMeasurement");
}

macro_rules! register_unit_class {
    ($fn_name:ident, $class_name:literal, $class_fn:ident, $meta_fn:ident, $parent_fn:ident,
     $( ($sel:literal, $shim:expr, $enc:literal) ),* $(,)?
     ; class $( ($csel:literal, $cshim:expr, $cenc:literal) ),* $(,)? ) => {
        pub fn $fn_name(rt: &mut objc_runtime::ObjcRuntime) {
            let class_addr = nsobj::$class_fn();
            let metaclass_addr = nsobj::$meta_fn();
            let info = ClassInfo {
                name: $class_name.to_string(),
                mapped_addr: class_addr,
                methods: vec![
                    me("symbol", nsmeasurement::shim_nsunit_symbol as u64, "@@:"),
                    me("initWithSymbol:", nsmeasurement::shim_nsdimension_init_with_symbol as u64, "@@:@"),
                    me("description", nsmeasurement::shim_nsunit_description as u64, "@@:"),
                    $( me($sel, $shim, $enc), )*
                ],
                class_methods: vec![
                    $( me($csel, $cshim, $cenc), )*
                ],
                ivars: vec![], properties: vec![], protocol_names: Vec::new(),
                instance_size: nsmeasurement::NSUNIT_SIZE as u32, is_root: false, is_meta: false,
                superclass_addr: Some(nsobj::$parent_fn()),
                method_index: HashMap::new(), class_method_index: HashMap::new(),
            };
            rt.register_synthetic_class(info);
            rt.register_metaclass_addr(metaclass_addr, $class_name);
        }
    };
}

register_unit_class!(register_nsunitlength_class, "NSUnitLength",
    nsunitlength_class_addr, nsunitlength_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunitlength_base_unit as u64, "@@:"),
    ("meters", nsmeasurement::shim_nsunitlength_meters as u64, "@@:"),
    ("kilometers", nsmeasurement::shim_nsunitlength_kilometers as u64, "@@:"),
    ("centimeters", nsmeasurement::shim_nsunitlength_centimeters as u64, "@@:"),
    ("millimeters", nsmeasurement::shim_nsunitlength_millimeters as u64, "@@:"),
    ("inches", nsmeasurement::shim_nsunitlength_inches as u64, "@@:"),
    ("feet", nsmeasurement::shim_nsunitlength_feet as u64, "@@:"),
    ("yards", nsmeasurement::shim_nsunitlength_yards as u64, "@@:"),
    ("miles", nsmeasurement::shim_nsunitlength_miles as u64, "@@:"),
    ("nauticalMiles", nsmeasurement::shim_nsunitlength_nautical_miles as u64, "@@:"),
    ("lightYears", nsmeasurement::shim_nsunitlength_light_years as u64, "@@:"),
);

register_unit_class!(register_nsunitmass_class, "NSUnitMass",
    nsunitmass_class_addr, nsunitmass_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunitmass_base_unit as u64, "@@:"),
    ("kilograms", nsmeasurement::shim_nsunitmass_kilograms as u64, "@@:"),
    ("grams", nsmeasurement::shim_nsunitmass_grams as u64, "@@:"),
    ("milligrams", nsmeasurement::shim_nsunitmass_milligrams as u64, "@@:"),
    ("pounds", nsmeasurement::shim_nsunitmass_pounds as u64, "@@:"),
    ("ounces", nsmeasurement::shim_nsunitmass_ounces as u64, "@@:"),
    ("metricTons", nsmeasurement::shim_nsunitmass_metric_tons as u64, "@@:"),
    ("shortTons", nsmeasurement::shim_nsunitmass_short_tons as u64, "@@:"),
);

register_unit_class!(register_nsunittemperature_class, "NSUnitTemperature",
    nsunittemperature_class_addr, nsunittemperature_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunittemperature_base_unit as u64, "@@:"),
    ("celsius", nsmeasurement::shim_nsunittemperature_celsius as u64, "@@:"),
    ("fahrenheit", nsmeasurement::shim_nsunittemperature_fahrenheit as u64, "@@:"),
    ("kelvin", nsmeasurement::shim_nsunittemperature_kelvin as u64, "@@:"),
);

register_unit_class!(register_nsunitvolume_class, "NSUnitVolume",
    nsunitvolume_class_addr, nsunitvolume_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunitvolume_base_unit as u64, "@@:"),
    ("liters", nsmeasurement::shim_nsunitvolume_liters as u64, "@@:"),
    ("milliliters", nsmeasurement::shim_nsunitvolume_milliliters as u64, "@@:"),
    ("cubicMeters", nsmeasurement::shim_nsunitvolume_cubic_meters as u64, "@@:"),
    ("gallons", nsmeasurement::shim_nsunitvolume_gallons as u64, "@@:"),
    ("fluidOunces", nsmeasurement::shim_nsunitvolume_fluid_ounces as u64, "@@:"),
    ("cups", nsmeasurement::shim_nsunitvolume_cups as u64, "@@:"),
    ("pints", nsmeasurement::shim_nsunitvolume_pints as u64, "@@:"),
);

register_unit_class!(register_nsunitangle_class, "NSUnitAngle",
    nsunitangle_class_addr, nsunitangle_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunitangle_base_unit as u64, "@@:"),
    ("degrees", nsmeasurement::shim_nsunitangle_degrees as u64, "@@:"),
    ("radians", nsmeasurement::shim_nsunitangle_radians as u64, "@@:"),
    ("arcMinutes", nsmeasurement::shim_nsunitangle_arcminutes as u64, "@@:"),
    ("arcSeconds", nsmeasurement::shim_nsunitangle_arcseconds as u64, "@@:"),
);

register_unit_class!(register_nsunitspeed_class, "NSUnitSpeed",
    nsunitspeed_class_addr, nsunitspeed_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunitspeed_base_unit as u64, "@@:"),
    ("metersPerSecond", nsmeasurement::shim_nsunitspeed_meters_per_second as u64, "@@:"),
    ("kilometersPerHour", nsmeasurement::shim_nsunitspeed_kilometers_per_hour as u64, "@@:"),
    ("milesPerHour", nsmeasurement::shim_nsunitspeed_miles_per_hour as u64, "@@:"),
    ("knots", nsmeasurement::shim_nsunitspeed_knots as u64, "@@:"),
);

register_unit_class!(register_nsunitduration_class, "NSUnitDuration",
    nsunitduration_class_addr, nsunitduration_metaclass_addr, nsdimension_class_addr,
    ;
    class
    ("baseUnit", nsmeasurement::shim_nsunitduration_base_unit as u64, "@@:"),
    ("seconds", nsmeasurement::shim_nsunitduration_seconds as u64, "@@:"),
    ("minutes", nsmeasurement::shim_nsunitduration_minutes as u64, "@@:"),
    ("hours", nsmeasurement::shim_nsunitduration_hours as u64, "@@:"),
    ("milliseconds", nsmeasurement::shim_nsunitduration_milliseconds as u64, "@@:"),
    ("microseconds", nsmeasurement::shim_nsunitduration_microseconds as u64, "@@:"),
    ("nanoseconds", nsmeasurement::shim_nsunitduration_nanoseconds as u64, "@@:"),
    ("picoseconds", nsmeasurement::shim_nsunitduration_picoseconds as u64, "@@:"),
);

pub fn register_nsmeasurementformatter_class(rt: &mut objc_runtime::ObjcRuntime) {
    let class_addr = nsobj::nsmeasurementformatter_class_addr();
    let metaclass_addr = nsobj::nsmeasurementformatter_metaclass_addr();
    let info = ClassInfo {
        name: "NSMeasurementFormatter".to_string(),
        mapped_addr: class_addr,
        methods: vec![
            me("init", nsmeasurement::shim_nsmeasurementformatter_init as u64, "@@:"),
            me("unitStyle", nsmeasurement::shim_nsmeasurementformatter_unit_style as u64, "Q@:"),
            me("setUnitStyle:", nsmeasurement::shim_nsmeasurementformatter_set_unit_style as u64, "v@:Q"),
            me("numberFormatter", nsmeasurement::shim_nsmeasurementformatter_number_formatter as u64, "@@:"),
            me("setNumberFormatter:", nsmeasurement::shim_nsmeasurementformatter_set_number_formatter as u64, "v@:@"),
            me("stringFromMeasurement:", nsmeasurement::shim_nsmeasurementformatter_string_from_measurement as u64, "@@:@"),
        ],
        class_methods: vec![
            me("new", nsmeasurement::shim_nsmeasurementformatter_new as u64, "@@:"),
            me("alloc", nsmeasurement::shim_nsmeasurementformatter_alloc as u64, "@@:"),
        ],
        ivars: vec![], properties: vec![], protocol_names: Vec::new(),
        instance_size: nsmeasurement::NSMEASUREMENTFORMATTER_SIZE as u32,
        is_root: false, is_meta: false,
        superclass_addr: Some(nsobj::nsobject_class_addr()),
        method_index: HashMap::new(), class_method_index: HashMap::new(),
    };
    rt.register_synthetic_class(info);
    rt.register_metaclass_addr(metaclass_addr, "NSMeasurementFormatter");
}
