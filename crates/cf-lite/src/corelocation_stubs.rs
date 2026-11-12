//! CoreLocation and CoreBluetooth stub classes.
//!
//! CLLocationManager, CLLocation, CLGeocoder — return "not determined"
//! authorization, zero coordinates, and empty geocode results.
//!
//! CBCentralManager, CBPeripheralManager — return CBManagerStateUnknown (0)
//! and are safe to instantiate without crashing.

// ── Instance sizes ────────────────────────────────────────────────────────────
pub const CLLOCATIONMANAGER_SIZE: usize = 32;
pub const CLLOCATION_SIZE: usize = 48; // ISA + lat + lon + alt + acc + timestamp
pub const CLGEOCODER_SIZE: usize = 24;
pub const CBCENTRALMANAGER_SIZE: usize = 32;
pub const CBPERIPHERALMANAGER_SIZE: usize = 32;

// ── CLLocationManager ─────────────────────────────────────────────────────────
// Layout: [ISA:8][delegate:8][desiredAccuracy:8(f64)][distanceFilter:8(f64)]
//
// CLAuthorizationStatus values: 0=NotDetermined, 2=Denied, 3=Restricted, 4=AuthorizedAlways, 5=WhenInUse

const CL_NOT_DETERMINED: u64 = 0;

pub extern "C" fn shim_cllocationmanager_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CLLOCATIONMANAGER_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_cllocationmanager_init(obj: u64, _cmd: u64) -> u64 {
    obj
}

pub extern "C" fn shim_cllocationmanager_new(cls: u64, cmd: u64) -> u64 {
    shim_cllocationmanager_init(shim_cllocationmanager_alloc(cls, cmd), cmd)
}

/// `+[CLLocationManager authorizationStatus]` → CLAuthorizationStatusNotDetermined (0).
pub extern "C" fn shim_cllocationmanager_authorization_status(_cls: u64, _cmd: u64) -> u64 {
    CL_NOT_DETERMINED
}

/// `+[CLLocationManager locationServicesEnabled]` → NO.
pub extern "C" fn shim_cllocationmanager_location_services_enabled(_cls: u64, _cmd: u64) -> u64 {
    0
}

pub extern "C" fn shim_cllocationmanager_delegate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + 8) as *const u64) }
}

pub extern "C" fn shim_cllocationmanager_set_delegate(obj: u64, _cmd: u64, delegate: u64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + 8) as *mut u64) = delegate;
    }
}

pub extern "C" fn shim_cllocationmanager_desired_accuracy(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 16) as *const f64) }
}

pub extern "C" fn shim_cllocationmanager_set_desired_accuracy(obj: u64, _cmd: u64, accuracy: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + 16) as *mut f64) = accuracy;
    }
}

pub extern "C" fn shim_cllocationmanager_distance_filter(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 24) as *const f64) }
}

pub extern "C" fn shim_cllocationmanager_set_distance_filter(obj: u64, _cmd: u64, filter: f64) {
    if obj == 0 {
        return;
    }
    unsafe {
        *((obj as usize + 24) as *mut f64) = filter;
    }
}

pub extern "C" fn shim_cllocationmanager_request_when_in_use_authorization(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cllocationmanager_request_always_authorization(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cllocationmanager_start_updating_location(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cllocationmanager_stop_updating_location(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cllocationmanager_start_monitoring_significant_location_changes(
    _obj: u64,
    _cmd: u64,
) {
}
pub extern "C" fn shim_cllocationmanager_stop_monitoring_significant_location_changes(
    _obj: u64,
    _cmd: u64,
) {
}
pub extern "C" fn shim_cllocationmanager_location(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_cllocationmanager_request_location(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cllocationmanager_start_updating_heading(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cllocationmanager_stop_updating_heading(_obj: u64, _cmd: u64) {}

// ── CLLocation ────────────────────────────────────────────────────────────────
// Layout: [ISA:8][latitude:8(f64)][longitude:8(f64)][altitude:8(f64)]
//         [horizontalAccuracy:8(f64)][timestamp(unused):8]

pub extern "C" fn shim_cllocation_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CLLOCATION_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_cllocation_init_with_lat_lon(
    obj: u64,
    _cmd: u64,
    lat: f64,
    lon: f64,
) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + 8) as *mut f64) = lat;
            *((obj as usize + 16) as *mut f64) = lon;
        }
    }
    obj
}

pub extern "C" fn shim_cllocation_coordinate_latitude(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 8) as *const f64) }
}

pub extern "C" fn shim_cllocation_coordinate_longitude(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 16) as *const f64) }
}

pub extern "C" fn shim_cllocation_altitude(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return 0.0;
    }
    unsafe { *((obj as usize + 24) as *const f64) }
}

pub extern "C" fn shim_cllocation_horizontal_accuracy(obj: u64, _cmd: u64) -> f64 {
    if obj == 0 {
        return -1.0;
    } // -1 = invalid
    unsafe { *((obj as usize + 32) as *const f64) }
}

pub extern "C" fn shim_cllocation_vertical_accuracy(_obj: u64, _cmd: u64) -> f64 {
    -1.0
}
pub extern "C" fn shim_cllocation_speed(_obj: u64, _cmd: u64) -> f64 {
    -1.0
}
pub extern "C" fn shim_cllocation_course(_obj: u64, _cmd: u64) -> f64 {
    -1.0
}
pub extern "C" fn shim_cllocation_timestamp(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_cllocation_distance_from_location(_obj: u64, _cmd: u64, _other: u64) -> f64 {
    0.0
}

// ── CLGeocoder ────────────────────────────────────────────────────────────────

pub extern "C" fn shim_clgeocoder_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CLGEOCODER_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_clgeocoder_init(obj: u64, _cmd: u64) -> u64 {
    obj
}
pub extern "C" fn shim_clgeocoder_new(cls: u64, cmd: u64) -> u64 {
    shim_clgeocoder_init(shim_clgeocoder_alloc(cls, cmd), cmd)
}

/// `geocodeAddressString:completionHandler:` — invokes handler immediately with empty results.
/// The block is NOT invoked (would require block dispatch infrastructure).
pub extern "C" fn shim_clgeocoder_geocode_address_string(
    _obj: u64,
    _cmd: u64,
    _addr: u64,
    _handler: u64,
) {
}
pub extern "C" fn shim_clgeocoder_reverse_geocode_location(
    _obj: u64,
    _cmd: u64,
    _loc: u64,
    _handler: u64,
) {
}
pub extern "C" fn shim_clgeocoder_cancel_geocode(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_clgeocoder_is_geocoding(_obj: u64, _cmd: u64) -> u64 {
    0
}

// ── CBCentralManager ──────────────────────────────────────────────────────────
// CBManagerState: 0=Unknown, 1=Resetting, 2=Unsupported, 3=Unauthorized, 4=PoweredOff, 5=PoweredOn

pub extern "C" fn shim_cbcentralmanager_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CBCENTRALMANAGER_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_cbcentralmanager_init_with_delegate(
    obj: u64,
    _cmd: u64,
    delegate: u64,
    _queue: u64,
) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + 8) as *mut u64) = delegate;
        }
    }
    obj
}

pub extern "C" fn shim_cbcentralmanager_init_with_delegate_options(
    obj: u64,
    cmd: u64,
    delegate: u64,
    queue: u64,
    _options: u64,
) -> u64 {
    shim_cbcentralmanager_init_with_delegate(obj, cmd, delegate, queue)
}

/// `state` → CBManagerStateUnknown (0).
pub extern "C" fn shim_cbcentralmanager_state(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_cbcentralmanager_delegate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + 8) as *const u64) }
}
pub extern "C" fn shim_cbcentralmanager_scan_for_peripherals(
    _obj: u64,
    _cmd: u64,
    _services: u64,
    _options: u64,
) {
}
pub extern "C" fn shim_cbcentralmanager_stop_scan(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cbcentralmanager_connect(
    _obj: u64,
    _cmd: u64,
    _peripheral: u64,
    _options: u64,
) {
}
pub extern "C" fn shim_cbcentralmanager_cancel_connection(_obj: u64, _cmd: u64, _peripheral: u64) {}
pub extern "C" fn shim_cbcentralmanager_is_scanning(_obj: u64, _cmd: u64) -> u64 {
    0
}

/// `+[CBCentralManager authorization]` → CBManagerAuthorizationStatusNotDetermined (0).
pub extern "C" fn shim_cbcentralmanager_authorization(_cls: u64, _cmd: u64) -> u64 {
    0
}

// ── CBPeripheralManager ───────────────────────────────────────────────────────

pub extern "C" fn shim_cbperipheralmanager_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, CBPERIPHERALMANAGER_SIZE) as u64;
    if obj == 0 {
        return 0;
    }
    unsafe {
        *(obj as *mut u64) = cls;
    }
    obj
}

pub extern "C" fn shim_cbperipheralmanager_init_with_delegate(
    obj: u64,
    _cmd: u64,
    delegate: u64,
    _queue: u64,
) -> u64 {
    if obj != 0 {
        unsafe {
            *((obj as usize + 8) as *mut u64) = delegate;
        }
    }
    obj
}

pub extern "C" fn shim_cbperipheralmanager_state(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_cbperipheralmanager_delegate(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 {
        return 0;
    }
    unsafe { *((obj as usize + 8) as *const u64) }
}
pub extern "C" fn shim_cbperipheralmanager_start_advertising(_obj: u64, _cmd: u64, _data: u64) {}
pub extern "C" fn shim_cbperipheralmanager_stop_advertising(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_cbperipheralmanager_is_advertising(_obj: u64, _cmd: u64) -> u64 {
    0
}
pub extern "C" fn shim_cbperipheralmanager_add(_obj: u64, _cmd: u64, _service: u64) {}
pub extern "C" fn shim_cbperipheralmanager_remove(_obj: u64, _cmd: u64, _service: u64) {}
pub extern "C" fn shim_cbperipheralmanager_remove_all_services(_obj: u64, _cmd: u64) {}
