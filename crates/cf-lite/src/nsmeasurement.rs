//! NSUnit and NSMeasurement shims.
//!
//! NSUnit is the abstract base class for units of measurement.
//! NSDimension extends NSUnit with a linear converter (coefficient + constant).
//! NSMeasurement wraps a double value + unit pair.
//!
//! Object layout — NSUnit / NSDimension (32 bytes):
//!   [0..8):   ISA pointer
//!   [8..16):  symbol: u64     — NSString pointer
//!   [16..24): coefficient: f64 — linear factor to base unit (1.0 = base)
//!   [24..32): constant: f64   — additive offset to base (0.0 for most units)
//!
//! Conversion formula (Apple NSUnitConverterLinear):
//!   base_value = value × coefficient + constant
//!   value      = (base_value − constant) / coefficient
//!
//! Object layout — NSMeasurement (24 bytes):
//!   [0..8):  ISA pointer
//!   [8..16): value: f64   — the numeric measurement value
//!   [16..24): unit: u64   — NSUnit* pointer

pub const NSUNIT_SIZE: usize = 32; // Extended to include coefficient + constant
pub const NSMEASUREMENT_SIZE: usize = 24;

const UNIT_SYMBOL_OFF: usize = 8;
const UNIT_COEFF_OFF: usize = 16;
const UNIT_CONST_OFF: usize = 24;
const MEAS_VALUE_OFF: usize = 8;
const MEAS_UNIT_OFF: usize = 16;

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn alloc_unit_linear(isa: u64, symbol: &str, coeff: f64, constant: f64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSUNIT_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = isa;
        *((obj as usize + UNIT_COEFF_OFF) as *mut f64) = coeff;
        *((obj as usize + UNIT_CONST_OFF) as *mut f64) = constant;
    }
    let sym = make_nsstring(symbol);
    unsafe { *((obj as usize + UNIT_SYMBOL_OFF) as *mut u64) = sym; }
    obj
}

fn alloc_unit(isa: u64, symbol: &str) -> u64 {
    alloc_unit_linear(isa, symbol, 1.0, 0.0)
}

// ── NSUnit ────────────────────────────────────────────────────────────────────

/// `−[NSUnit symbol]` — returns the unit symbol as NSString.
pub extern "C" fn shim_nsunit_symbol(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + UNIT_SYMBOL_OFF) as *const u64) }
}

/// `-[NSUnit initWithSymbol:]`
pub extern "C" fn shim_nsunit_init_with_symbol(obj: u64, _cmd: u64, symbol: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsunit_class_addr();
        *((obj as usize + UNIT_SYMBOL_OFF) as *mut u64) = symbol;
    }
    obj
}

/// `-[NSUnit description]`
pub extern "C" fn shim_nsunit_description(obj: u64, _cmd: u64) -> u64 {
    shim_nsunit_symbol(obj, _cmd)
}

// ── NSDimension (stub — no real converter, identity only) ─────────────────────

/// `-[NSDimension initWithSymbol:]` (same as NSUnit)
pub extern "C" fn shim_nsdimension_init_with_symbol(obj: u64, _cmd: u64, symbol: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsdimension_class_addr();
        *((obj as usize + UNIT_SYMBOL_OFF) as *mut u64) = symbol;
    }
    obj
}

/// `-[NSDimension baseUnit]` — stub: returns self.
pub extern "C" fn shim_nsdimension_base_unit(obj: u64, _cmd: u64) -> u64 { obj }

// ── NSMeasurement ─────────────────────────────────────────────────────────────

fn alloc_measurement(value: f64, unit: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMEASUREMENT_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmeasurement_class_addr();
        *((obj as usize + MEAS_VALUE_OFF) as *mut f64) = value;
        *((obj as usize + MEAS_UNIT_OFF) as *mut u64) = unit;
    }
    obj
}

/// `-[NSMeasurement initWithDoubleValue:unit:]`
pub extern "C" fn shim_nsmeasurement_init_with_double_value_unit(
    obj: u64, _cmd: u64, value: f64, unit: u64,
) -> u64 {
    if obj == 0 { return alloc_measurement(value, unit); }
    unsafe {
        *(obj as *mut u64) = crate::nsobj::nsmeasurement_class_addr();
        *((obj as usize + MEAS_VALUE_OFF) as *mut f64) = value;
        *((obj as usize + MEAS_UNIT_OFF) as *mut u64) = unit;
    }
    obj
}

/// `-[NSMeasurement doubleValue]`
pub extern "C" fn shim_nsmeasurement_double_value(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 { return 0.0; }
    unsafe { *((obj as usize + MEAS_VALUE_OFF) as *const f64) }
}

/// `-[NSMeasurement unit]`
pub extern "C" fn shim_nsmeasurement_unit(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + MEAS_UNIT_OFF) as *const u64) }
}

/// `-[NSMeasurement measurementByConvertingToUnit:]`
/// Performs linear unit conversion: converts via base unit using coefficient/constant.
pub extern "C" fn shim_nsmeasurement_measurement_by_converting_to_unit(
    obj: u64, _cmd: u64, to_unit: u64,
) -> u64 {
    if obj == 0 { return 0; }
    let value = unsafe { *((obj as usize + MEAS_VALUE_OFF) as *const f64) };
    let from_unit = unsafe { *((obj as usize + MEAS_UNIT_OFF) as *const u64) };

    // Read linear converter parameters from both units.
    let (from_coeff, from_const) = if from_unit != 0 {
        let c = unsafe { *((from_unit as usize + UNIT_COEFF_OFF) as *const f64) };
        let k = unsafe { *((from_unit as usize + UNIT_CONST_OFF) as *const f64) };
        if c == 0.0 { (1.0f64, 0.0f64) } else { (c, k) }
    } else { (1.0, 0.0) };

    let (to_coeff, to_const) = if to_unit != 0 {
        let c = unsafe { *((to_unit as usize + UNIT_COEFF_OFF) as *const f64) };
        let k = unsafe { *((to_unit as usize + UNIT_CONST_OFF) as *const f64) };
        if c == 0.0 { (1.0f64, 0.0f64) } else { (c, k) }
    } else { (1.0, 0.0) };

    // Convert: value → base → target
    let base = value * from_coeff + from_const;
    let converted = (base - to_const) / to_coeff;

    alloc_measurement(converted, to_unit)
}

/// `-[NSMeasurement isEqualToMeasurement:]`
pub extern "C" fn shim_nsmeasurement_is_equal_to_measurement(
    obj: u64, _cmd: u64, other: u64,
) -> u8 {
    if obj == 0 || other == 0 { return 0; }
    let v1 = unsafe { *((obj as usize + MEAS_VALUE_OFF) as *const f64) };
    let v2 = unsafe { *((other as usize + MEAS_VALUE_OFF) as *const f64) };
    let u1 = unsafe { *((obj as usize + MEAS_UNIT_OFF) as *const u64) };
    let u2 = unsafe { *((other as usize + MEAS_UNIT_OFF) as *const u64) };
    if (v1 - v2).abs() < f64::EPSILON && u1 == u2 { 1 } else { 0 }
}

/// `-[NSMeasurement description]` — "value symbol" format.
pub extern "C" fn shim_nsmeasurement_description(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    let value = unsafe { *((obj as usize + MEAS_VALUE_OFF) as *const f64) };
    let unit = unsafe { *((obj as usize + MEAS_UNIT_OFF) as *const u64) };
    let sym = if unit != 0 {
        crate::cfstring::extract_cfstring_cstr(
            unsafe { *((unit as usize + UNIT_SYMBOL_OFF) as *const u64) }
        ).unwrap_or("?")
    } else { "" };
    let s = format!("{value} {sym}");
    make_nsstring(&s)
}

// ── Concrete unit singletons ──────────────────────────────────────────────────

use std::sync::OnceLock;

/// Linear-converter singleton (base = value × coeff + const).
macro_rules! unit_singleton_linear {
    ($static:ident, $fn_name:ident, $isa_fn:ident, $symbol:expr, $coeff:expr, $const:expr) => {
        static $static: OnceLock<u64> = OnceLock::new();
        pub fn $fn_name() -> u64 {
            *$static.get_or_init(|| alloc_unit_linear(crate::nsobj::$isa_fn(), $symbol, $coeff, $const))
        }
    };
}

// ── NSUnitLength (base = meters) ─────────────────────────────────────────────

unit_singleton_linear!(UNIT_METERS,          nsunitlength_meters,          nsunitlength_class_addr, "m",   1.0,         0.0);
unit_singleton_linear!(UNIT_KILOMETERS,      nsunitlength_kilometers,      nsunitlength_class_addr, "km",  1_000.0,     0.0);
unit_singleton_linear!(UNIT_CENTIMETERS,     nsunitlength_centimeters,     nsunitlength_class_addr, "cm",  0.01,        0.0);
unit_singleton_linear!(UNIT_MILLIMETERS,     nsunitlength_millimeters,     nsunitlength_class_addr, "mm",  0.001,       0.0);
unit_singleton_linear!(UNIT_INCHES,          nsunitlength_inches,          nsunitlength_class_addr, "in",  0.0254,      0.0);
unit_singleton_linear!(UNIT_FEET,            nsunitlength_feet,            nsunitlength_class_addr, "ft",  0.3048,      0.0);
unit_singleton_linear!(UNIT_YARDS,           nsunitlength_yards,           nsunitlength_class_addr, "yd",  0.9144,      0.0);
unit_singleton_linear!(UNIT_MILES,           nsunitlength_miles,           nsunitlength_class_addr, "mi",  1_609.344,   0.0);
unit_singleton_linear!(UNIT_NAUTICAL_MILES,  nsunitlength_nautical_miles,  nsunitlength_class_addr, "NM",  1_852.0,     0.0);
unit_singleton_linear!(UNIT_LIGHTYEARS,      nsunitlength_light_years,     nsunitlength_class_addr, "ly",  9.4607305e15, 0.0);

pub extern "C" fn shim_nsunitlength_meters(_cls: u64, _cmd: u64)          -> u64 { nsunitlength_meters() }
pub extern "C" fn shim_nsunitlength_kilometers(_cls: u64, _cmd: u64)      -> u64 { nsunitlength_kilometers() }
pub extern "C" fn shim_nsunitlength_centimeters(_cls: u64, _cmd: u64)     -> u64 { nsunitlength_centimeters() }
pub extern "C" fn shim_nsunitlength_millimeters(_cls: u64, _cmd: u64)     -> u64 { nsunitlength_millimeters() }
pub extern "C" fn shim_nsunitlength_inches(_cls: u64, _cmd: u64)          -> u64 { nsunitlength_inches() }
pub extern "C" fn shim_nsunitlength_feet(_cls: u64, _cmd: u64)            -> u64 { nsunitlength_feet() }
pub extern "C" fn shim_nsunitlength_yards(_cls: u64, _cmd: u64)           -> u64 { nsunitlength_yards() }
pub extern "C" fn shim_nsunitlength_miles(_cls: u64, _cmd: u64)           -> u64 { nsunitlength_miles() }
pub extern "C" fn shim_nsunitlength_nautical_miles(_cls: u64, _cmd: u64)  -> u64 { nsunitlength_nautical_miles() }
pub extern "C" fn shim_nsunitlength_light_years(_cls: u64, _cmd: u64)     -> u64 { nsunitlength_light_years() }
pub extern "C" fn shim_nsunitlength_base_unit(_cls: u64, _cmd: u64)       -> u64 { nsunitlength_meters() }

// ── NSUnitMass (base = kilograms) ─────────────────────────────────────────────

unit_singleton_linear!(UNIT_KILOGRAMS,   nsunitmass_kilograms,   nsunitmass_class_addr, "kg",    1.0,          0.0);
unit_singleton_linear!(UNIT_GRAMS,       nsunitmass_grams,       nsunitmass_class_addr, "g",     0.001,        0.0);
unit_singleton_linear!(UNIT_MILLIGRAMS,  nsunitmass_milligrams,  nsunitmass_class_addr, "mg",    0.000_001,    0.0);
unit_singleton_linear!(UNIT_POUNDS,      nsunitmass_pounds,      nsunitmass_class_addr, "lb",    0.453_592,    0.0);
unit_singleton_linear!(UNIT_OUNCES,      nsunitmass_ounces,      nsunitmass_class_addr, "oz",    0.028_349_5,  0.0);
unit_singleton_linear!(UNIT_METRIC_TONS, nsunitmass_metric_tons, nsunitmass_class_addr, "t",     1_000.0,      0.0);
unit_singleton_linear!(UNIT_SHORT_TONS,  nsunitmass_short_tons,  nsunitmass_class_addr, "tn",    907.185,      0.0);

pub extern "C" fn shim_nsunitmass_kilograms(_cls: u64, _cmd: u64)   -> u64 { nsunitmass_kilograms() }
pub extern "C" fn shim_nsunitmass_grams(_cls: u64, _cmd: u64)       -> u64 { nsunitmass_grams() }
pub extern "C" fn shim_nsunitmass_milligrams(_cls: u64, _cmd: u64)  -> u64 { nsunitmass_milligrams() }
pub extern "C" fn shim_nsunitmass_pounds(_cls: u64, _cmd: u64)      -> u64 { nsunitmass_pounds() }
pub extern "C" fn shim_nsunitmass_ounces(_cls: u64, _cmd: u64)      -> u64 { nsunitmass_ounces() }
pub extern "C" fn shim_nsunitmass_metric_tons(_cls: u64, _cmd: u64) -> u64 { nsunitmass_metric_tons() }
pub extern "C" fn shim_nsunitmass_short_tons(_cls: u64, _cmd: u64)  -> u64 { nsunitmass_short_tons() }
pub extern "C" fn shim_nsunitmass_base_unit(_cls: u64, _cmd: u64)   -> u64 { nsunitmass_kilograms() }

// ── NSUnitTemperature (base = Kelvin) ─────────────────────────────────────────
//  formula: K = value × coeff + const
//  celsius:     K = C + 273.15    → coeff=1.0,       const=273.15
//  fahrenheit:  K = F × 5/9 + 255.3722  → coeff=5/9, const=255.3722

unit_singleton_linear!(UNIT_KELVIN,      nsunittemperature_kelvin,      nsunittemperature_class_addr, "K",   1.0,              0.0);
unit_singleton_linear!(UNIT_CELSIUS,     nsunittemperature_celsius,     nsunittemperature_class_addr, "°C",  1.0,              273.15);
unit_singleton_linear!(UNIT_FAHRENHEIT,  nsunittemperature_fahrenheit,  nsunittemperature_class_addr, "°F",  5.0 / 9.0,        255.372_222_222);

pub extern "C" fn shim_nsunittemperature_celsius(_cls: u64, _cmd: u64)    -> u64 { nsunittemperature_celsius() }
pub extern "C" fn shim_nsunittemperature_fahrenheit(_cls: u64, _cmd: u64) -> u64 { nsunittemperature_fahrenheit() }
pub extern "C" fn shim_nsunittemperature_kelvin(_cls: u64, _cmd: u64)     -> u64 { nsunittemperature_kelvin() }
pub extern "C" fn shim_nsunittemperature_base_unit(_cls: u64, _cmd: u64)  -> u64 { nsunittemperature_kelvin() }

// ── NSUnitVolume (base = liters) ──────────────────────────────────────────────

unit_singleton_linear!(UNIT_LITERS,        nsunitvolume_liters,        nsunitvolume_class_addr, "L",      1.0,         0.0);
unit_singleton_linear!(UNIT_MILLILITERS,   nsunitvolume_milliliters,   nsunitvolume_class_addr, "mL",     0.001,       0.0);
unit_singleton_linear!(UNIT_CUBIC_METERS,  nsunitvolume_cubic_meters,  nsunitvolume_class_addr, "m³",     1_000.0,     0.0);
unit_singleton_linear!(UNIT_GALLONS,       nsunitvolume_gallons,       nsunitvolume_class_addr, "gal",    3.785_41,    0.0);
unit_singleton_linear!(UNIT_FLUID_OUNCES,  nsunitvolume_fluid_ounces,  nsunitvolume_class_addr, "fl oz",  0.029_573_5, 0.0);
unit_singleton_linear!(UNIT_CUPS,          nsunitvolume_cups,          nsunitvolume_class_addr, "cup",    0.236_588,   0.0);
unit_singleton_linear!(UNIT_PINTS,         nsunitvolume_pints,         nsunitvolume_class_addr, "pt",     0.473_176,   0.0);

pub extern "C" fn shim_nsunitvolume_liters(_cls: u64, _cmd: u64)        -> u64 { nsunitvolume_liters() }
pub extern "C" fn shim_nsunitvolume_milliliters(_cls: u64, _cmd: u64)   -> u64 { nsunitvolume_milliliters() }
pub extern "C" fn shim_nsunitvolume_cubic_meters(_cls: u64, _cmd: u64)  -> u64 { nsunitvolume_cubic_meters() }
pub extern "C" fn shim_nsunitvolume_gallons(_cls: u64, _cmd: u64)       -> u64 { nsunitvolume_gallons() }
pub extern "C" fn shim_nsunitvolume_fluid_ounces(_cls: u64, _cmd: u64)  -> u64 { nsunitvolume_fluid_ounces() }
pub extern "C" fn shim_nsunitvolume_cups(_cls: u64, _cmd: u64)          -> u64 { nsunitvolume_cups() }
pub extern "C" fn shim_nsunitvolume_pints(_cls: u64, _cmd: u64)         -> u64 { nsunitvolume_pints() }
pub extern "C" fn shim_nsunitvolume_base_unit(_cls: u64, _cmd: u64)     -> u64 { nsunitvolume_liters() }

// ── NSUnitAngle (base = degrees) ──────────────────────────────────────────────

unit_singleton_linear!(UNIT_DEGREES,     nsunitangle_degrees,     nsunitangle_class_addr, "°",   1.0,                  0.0);
unit_singleton_linear!(UNIT_RADIANS,     nsunitangle_radians,     nsunitangle_class_addr, "rad", 180.0 / std::f64::consts::PI, 0.0);
unit_singleton_linear!(UNIT_ARCMINUTES,  nsunitangle_arcminutes,  nsunitangle_class_addr, "′",   1.0 / 60.0,           0.0);
unit_singleton_linear!(UNIT_ARCSECONDS,  nsunitangle_arcseconds,  nsunitangle_class_addr, "″",   1.0 / 3_600.0,        0.0);

pub extern "C" fn shim_nsunitangle_degrees(_cls: u64, _cmd: u64)     -> u64 { nsunitangle_degrees() }
pub extern "C" fn shim_nsunitangle_radians(_cls: u64, _cmd: u64)     -> u64 { nsunitangle_radians() }
pub extern "C" fn shim_nsunitangle_arcminutes(_cls: u64, _cmd: u64)  -> u64 { nsunitangle_arcminutes() }
pub extern "C" fn shim_nsunitangle_arcseconds(_cls: u64, _cmd: u64)  -> u64 { nsunitangle_arcseconds() }
pub extern "C" fn shim_nsunitangle_base_unit(_cls: u64, _cmd: u64)   -> u64 { nsunitangle_degrees() }

// ── NSUnitSpeed (base = meters/second) ───────────────────────────────────────

unit_singleton_linear!(UNIT_MPS,   nsunitspeed_meters_per_second,   nsunitspeed_class_addr, "m/s",  1.0,          0.0);
unit_singleton_linear!(UNIT_KMPH,  nsunitspeed_kilometers_per_hour, nsunitspeed_class_addr, "km/h", 1.0 / 3.6,    0.0);
unit_singleton_linear!(UNIT_MPH,   nsunitspeed_miles_per_hour,      nsunitspeed_class_addr, "mph",  0.447_04,     0.0);
unit_singleton_linear!(UNIT_KNOTS, nsunitspeed_knots,               nsunitspeed_class_addr, "kn",   0.514_444,    0.0);

pub extern "C" fn shim_nsunitspeed_meters_per_second(_cls: u64, _cmd: u64)   -> u64 { nsunitspeed_meters_per_second() }
pub extern "C" fn shim_nsunitspeed_kilometers_per_hour(_cls: u64, _cmd: u64) -> u64 { nsunitspeed_kilometers_per_hour() }
pub extern "C" fn shim_nsunitspeed_miles_per_hour(_cls: u64, _cmd: u64)      -> u64 { nsunitspeed_miles_per_hour() }
pub extern "C" fn shim_nsunitspeed_knots(_cls: u64, _cmd: u64)               -> u64 { nsunitspeed_knots() }
pub extern "C" fn shim_nsunitspeed_base_unit(_cls: u64, _cmd: u64)           -> u64 { nsunitspeed_meters_per_second() }

// ── NSUnitDuration (base = seconds) ──────────────────────────────────────────

unit_singleton_linear!(UNIT_SECONDS,       nsunitduration_seconds,       nsunitduration_class_addr, "s",   1.0,             0.0);
unit_singleton_linear!(UNIT_MINUTES,       nsunitduration_minutes,       nsunitduration_class_addr, "min", 60.0,            0.0);
unit_singleton_linear!(UNIT_HOURS,         nsunitduration_hours,         nsunitduration_class_addr, "hr",  3_600.0,         0.0);
unit_singleton_linear!(UNIT_MILLISECONDS,  nsunitduration_milliseconds,  nsunitduration_class_addr, "ms",  0.001,           0.0);
unit_singleton_linear!(UNIT_MICROSECONDS,  nsunitduration_microseconds,  nsunitduration_class_addr, "µs",  0.000_001,       0.0);
unit_singleton_linear!(UNIT_NANOSECONDS,   nsunitduration_nanoseconds,   nsunitduration_class_addr, "ns",  1e-9,            0.0);
unit_singleton_linear!(UNIT_PICOSECONDS,   nsunitduration_picoseconds,   nsunitduration_class_addr, "ps",  1e-12,           0.0);

pub extern "C" fn shim_nsunitduration_seconds(_cls: u64, _cmd: u64)       -> u64 { nsunitduration_seconds() }
pub extern "C" fn shim_nsunitduration_minutes(_cls: u64, _cmd: u64)       -> u64 { nsunitduration_minutes() }
pub extern "C" fn shim_nsunitduration_hours(_cls: u64, _cmd: u64)         -> u64 { nsunitduration_hours() }
pub extern "C" fn shim_nsunitduration_milliseconds(_cls: u64, _cmd: u64)  -> u64 { nsunitduration_milliseconds() }
pub extern "C" fn shim_nsunitduration_microseconds(_cls: u64, _cmd: u64)  -> u64 { nsunitduration_microseconds() }
pub extern "C" fn shim_nsunitduration_nanoseconds(_cls: u64, _cmd: u64)   -> u64 { nsunitduration_nanoseconds() }
pub extern "C" fn shim_nsunitduration_picoseconds(_cls: u64, _cmd: u64)   -> u64 { nsunitduration_picoseconds() }
pub extern "C" fn shim_nsunitduration_base_unit(_cls: u64, _cmd: u64)     -> u64 { nsunitduration_seconds() }

// ── NSMeasurementFormatter ────────────────────────────────────────────────────
//  Layout (32 bytes):
//    [0..8]   ISA
//    [8..16]  numberFormatter (u64, unused; 0 = use default formatting)
//    [16..24] unitStyle (u64: 0=short, 1=medium, 2=long)
//    [24..32] locale (unused)

pub const NSMEASUREMENTFORMATTER_SIZE: usize = 32;
const FMT_NUMFMT_OFF: usize = 8;
const FMT_STYLE_OFF:  usize = 16;

pub extern "C" fn shim_nsmeasurementformatter_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, NSMEASUREMENTFORMATTER_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_nsmeasurementformatter_init(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    // ISA already set by alloc; set unitStyle = 1 (medium)
    unsafe { *((obj as usize + FMT_STYLE_OFF) as *mut u64) = 1; }
    obj
}

pub extern "C" fn shim_nsmeasurementformatter_new(cls: u64, cmd: u64) -> u64 {
    let obj = shim_nsmeasurementformatter_alloc(cls, cmd);
    shim_nsmeasurementformatter_init(obj, cmd)
}

pub extern "C" fn shim_nsmeasurementformatter_unit_style(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 1; }
    unsafe { *((obj as usize + FMT_STYLE_OFF) as *const u64) }
}

pub extern "C" fn shim_nsmeasurementformatter_set_unit_style(obj: u64, _cmd: u64, style: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + FMT_STYLE_OFF) as *mut u64) = style; }
}

pub extern "C" fn shim_nsmeasurementformatter_number_formatter(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + FMT_NUMFMT_OFF) as *const u64) }
}

pub extern "C" fn shim_nsmeasurementformatter_set_number_formatter(obj: u64, _cmd: u64, fmt: u64) {
    if obj == 0 { return; }
    unsafe { *((obj as usize + FMT_NUMFMT_OFF) as *mut u64) = fmt; }
}

/// `-[NSMeasurementFormatter stringFromMeasurement:]` → NSString "{value} {symbol}"
pub extern "C" fn shim_nsmeasurementformatter_string_from_measurement(
    _self: u64,
    _cmd: u64,
    measurement: u64,
) -> u64 {
    if measurement == 0 { return 0; }
    let value = unsafe { *((measurement as usize + MEAS_VALUE_OFF) as *const f64) };
    let unit = unsafe { *((measurement as usize + MEAS_UNIT_OFF) as *const u64) };

    let symbol_ns = if unit != 0 { shim_nsunit_symbol(unit, 0) } else { 0 };
    let sym_str = if symbol_ns != 0 {
        crate::cfstring::extract_cfstring_cstr(symbol_ns)
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };

    let formatted = if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{:.0}", value)
    } else {
        let s = format!("{:.6}", value);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };

    let result = if sym_str.is_empty() {
        formatted
    } else {
        format!("{} {}", formatted, sym_str)
    };

    // Null-terminate and pass to alloc_nsstring_from_cstr
    let mut bytes = result.into_bytes();
    bytes.push(0);
    crate::cfstring::alloc_nsstring_from_cstr(bytes.as_ptr() as *const std::ffi::c_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_length_meters_not_null() {
        let u = nsunitlength_meters();
        assert_ne!(u, 0);
        let sym = shim_nsunit_symbol(u, 0);
        assert_ne!(sym, 0);
        let s = crate::cfstring::extract_cfstring_cstr(sym).unwrap();
        assert_eq!(s, "m");
    }

    #[test]
    fn conversion_km_to_m() {
        // 1 km should be 1000 m
        let km_unit = nsunitlength_kilometers();
        let m_unit = nsunitlength_meters();
        let meas = alloc_measurement(1.0, km_unit);
        let converted_obj = shim_nsmeasurement_measurement_by_converting_to_unit(meas, 0, m_unit);
        assert_ne!(converted_obj, 0);
        let v = unsafe { *((converted_obj as usize + MEAS_VALUE_OFF) as *const f64) };
        assert!((v - 1000.0).abs() < 0.001, "Expected 1000 m, got {v}");
    }

    #[test]
    fn conversion_celsius_to_kelvin() {
        // 0 °C = 273.15 K
        let c_unit = nsunittemperature_celsius();
        let k_unit = nsunittemperature_kelvin();
        let meas = alloc_measurement(0.0, c_unit);
        let converted = shim_nsmeasurement_measurement_by_converting_to_unit(meas, 0, k_unit);
        let v = unsafe { *((converted as usize + MEAS_VALUE_OFF) as *const f64) };
        assert!((v - 273.15).abs() < 0.001, "Expected 273.15 K, got {v}");
    }

    #[test]
    fn unit_length_singletons() {
        assert_eq!(nsunitlength_meters(), nsunitlength_meters());
        assert_ne!(nsunitlength_meters(), nsunitlength_kilometers());
    }

    #[test]
    fn measurement_init_and_read() {
        let unit = nsunitlength_meters();
        let m = alloc_measurement(42.0, unit);
        assert_ne!(m, 0);
        assert!((shim_nsmeasurement_double_value(m, 0) - 42.0).abs() < 1e-9);
        assert_eq!(shim_nsmeasurement_unit(m, 0), unit);
    }

    #[test]
    fn measurement_is_equal() {
        let unit = nsunitmass_kilograms();
        let m1 = alloc_measurement(5.0, unit);
        let m2 = alloc_measurement(5.0, unit);
        let m3 = alloc_measurement(6.0, unit);
        assert_eq!(shim_nsmeasurement_is_equal_to_measurement(m1, 0, m2), 1);
        assert_eq!(shim_nsmeasurement_is_equal_to_measurement(m1, 0, m3), 0);
    }

    #[test]
    fn temperature_units_distinct() {
        assert_ne!(nsunittemperature_celsius(), nsunittemperature_fahrenheit());
        assert_ne!(nsunittemperature_celsius(), nsunittemperature_kelvin());
    }

    #[test]
    fn nil_safety() {
        assert_eq!(shim_nsunit_symbol(0, 0), 0);
        assert_eq!(shim_nsmeasurement_double_value(0, 0), 0.0);
        assert_eq!(shim_nsmeasurement_unit(0, 0), 0);
    }
}
