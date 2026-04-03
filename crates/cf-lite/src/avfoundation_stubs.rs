//! AVFoundation stub classes.
//!
//! Provides minimal ObjC class shims for AVAudioPlayer, AVAudioRecorder,
//! AVAudioEngine, AVCaptureDevice, AVCaptureSession, and AVPlayer.
//! All methods return nil / 0 / NO so AVFoundation-using code can
//! instantiate without crashing on unrecognized selectors.

// ── Instance sizes ────────────────────────────────────────────────────────────
pub const AVAUDIOPLAYER_SIZE:  usize = 40;
pub const AVAUDIORECORDER_SIZE: usize = 40;
pub const AVAUDIOENGINE_SIZE:  usize = 24;
pub const AVCAPTUREDEVICE_SIZE: usize = 24;
pub const AVCAPTURESESSION_SIZE: usize = 32;
pub const AVPLAYER_SIZE:       usize = 32;

// ── AVAudioPlayer ─────────────────────────────────────────────────────────────
// Layout: [ISA:8][url:8][duration:8(f64)][volume:8(f64)][currentTime:8(f64)]

pub extern "C" fn shim_avaudioplayer_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, AVAUDIOPLAYER_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_avaudioplayer_init_with_url(obj: u64, _cmd: u64, url: u64, error_out: u64) -> u64 {
    if error_out != 0 { unsafe { *(error_out as *mut u64) = 0; } }
    if obj != 0 { unsafe { *((obj as usize + 8) as *mut u64) = url; } }
    obj
}

pub extern "C" fn shim_avaudioplayer_init_with_data(obj: u64, _cmd: u64, _data: u64, error_out: u64) -> u64 {
    if error_out != 0 { unsafe { *(error_out as *mut u64) = 0; } }
    obj
}

pub extern "C" fn shim_avaudioplayer_play(_obj: u64, _cmd: u64) -> u64 { 0 /* NO */ }
pub extern "C" fn shim_avaudioplayer_pause(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avaudioplayer_stop(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avaudioplayer_prepare_to_play(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudioplayer_is_playing(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudioplayer_duration(_obj: u64, _cmd: u64) -> f64 { 0.0 }
pub extern "C" fn shim_avaudioplayer_current_time(_obj: u64, _cmd: u64) -> f64 { 0.0 }
pub extern "C" fn shim_avaudioplayer_set_current_time(_obj: u64, _cmd: u64, _t: f64) {}
pub extern "C" fn shim_avaudioplayer_volume(_obj: u64, _cmd: u64) -> f32 { 1.0 }
pub extern "C" fn shim_avaudioplayer_set_volume(_obj: u64, _cmd: u64, _v: f32) {}
pub extern "C" fn shim_avaudioplayer_number_of_loops(_obj: u64, _cmd: u64) -> i64 { 0 }
pub extern "C" fn shim_avaudioplayer_set_number_of_loops(_obj: u64, _cmd: u64, _n: i64) {}
pub extern "C" fn shim_avaudioplayer_url(obj: u64, _cmd: u64) -> u64 {
    if obj == 0 { return 0; }
    unsafe { *((obj as usize + 8) as *const u64) }
}
pub extern "C" fn shim_avaudioplayer_delegate(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudioplayer_set_delegate(_obj: u64, _cmd: u64, _d: u64) {}

// ── AVAudioRecorder ───────────────────────────────────────────────────────────

pub extern "C" fn shim_avaudiorecorder_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, AVAUDIORECORDER_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_avaudiorecorder_init_with_url(obj: u64, _cmd: u64, _url: u64, _settings: u64, error_out: u64) -> u64 {
    if error_out != 0 { unsafe { *(error_out as *mut u64) = 0; } }
    obj
}

pub extern "C" fn shim_avaudiorecorder_record(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudiorecorder_stop(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avaudiorecorder_is_recording(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudiorecorder_delete_recording(_obj: u64, _cmd: u64) -> u64 { 0 }

// ── AVAudioEngine ─────────────────────────────────────────────────────────────

pub extern "C" fn shim_avaudioengine_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, AVAUDIOENGINE_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_avaudioengine_init(obj: u64, _cmd: u64) -> u64 { obj }
pub extern "C" fn shim_avaudioengine_new(cls: u64, cmd: u64) -> u64 {
    shim_avaudioengine_init(shim_avaudioengine_alloc(cls, cmd), cmd)
}
pub extern "C" fn shim_avaudioengine_start(_obj: u64, _cmd: u64, error_out: u64) -> u64 {
    if error_out != 0 { unsafe { *(error_out as *mut u64) = 0; } }
    0
}
pub extern "C" fn shim_avaudioengine_stop(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avaudioengine_is_running(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudioengine_main_mixer_node(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudioengine_output_node(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avaudioengine_input_node(_obj: u64, _cmd: u64) -> u64 { 0 }

// ── AVCaptureDevice ───────────────────────────────────────────────────────────

pub extern "C" fn shim_avcapturedevice_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, AVCAPTUREDEVICE_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

/// `+[AVCaptureDevice defaultDeviceWithMediaType:]` → nil (no camera available).
pub extern "C" fn shim_avcapturedevice_default_device_with_media_type(_cls: u64, _cmd: u64, _media_type: u64) -> u64 { 0 }
/// `+[AVCaptureDevice defaultDeviceWithDeviceType:mediaType:position:]` → nil
pub extern "C" fn shim_avcapturedevice_default_device(_cls: u64, _cmd: u64, _dev_type: u64, _media_type: u64, _pos: u64) -> u64 { 0 }
/// `+[AVCaptureDevice devicesWithMediaType:]` → empty array
pub extern "C" fn shim_avcapturedevice_devices_with_media_type(_cls: u64, _cmd: u64, _media_type: u64) -> u64 {
    crate::nsarray::make_nsarray(&[])
}
pub extern "C" fn shim_avcapturedevice_authorization_status_for_media_type(_cls: u64, _cmd: u64, _media_type: u64) -> u64 {
    0 // AVAuthorizationStatusNotDetermined
}
pub extern "C" fn shim_avcapturedevice_request_access(_cls: u64, _cmd: u64, _media_type: u64, _handler: u64) {}
pub extern "C" fn shim_avcapturedevice_local_name(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avcapturedevice_unique_id(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avcapturedevice_has_media_type(_obj: u64, _cmd: u64, _media_type: u64) -> u64 { 0 }

// ── AVCaptureSession ──────────────────────────────────────────────────────────

pub extern "C" fn shim_avcapturesession_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, AVCAPTURESESSION_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_avcapturesession_init(obj: u64, _cmd: u64) -> u64 { obj }
pub extern "C" fn shim_avcapturesession_new(cls: u64, cmd: u64) -> u64 {
    shim_avcapturesession_init(shim_avcapturesession_alloc(cls, cmd), cmd)
}
pub extern "C" fn shim_avcapturesession_add_input(_obj: u64, _cmd: u64, _input: u64) {}
pub extern "C" fn shim_avcapturesession_add_output(_obj: u64, _cmd: u64, _output: u64) {}
pub extern "C" fn shim_avcapturesession_remove_input(_obj: u64, _cmd: u64, _input: u64) {}
pub extern "C" fn shim_avcapturesession_remove_output(_obj: u64, _cmd: u64, _output: u64) {}
pub extern "C" fn shim_avcapturesession_can_add_input(_obj: u64, _cmd: u64, _input: u64) -> u64 { 0 }
pub extern "C" fn shim_avcapturesession_can_add_output(_obj: u64, _cmd: u64, _output: u64) -> u64 { 0 }
pub extern "C" fn shim_avcapturesession_start_running(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avcapturesession_stop_running(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avcapturesession_is_running(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avcapturesession_session_preset(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avcapturesession_set_session_preset(_obj: u64, _cmd: u64, _preset: u64) {}

// ── AVPlayer ──────────────────────────────────────────────────────────────────

pub extern "C" fn shim_avplayer_alloc(cls: u64, _cmd: u64) -> u64 {
    let obj = crate::pal_alloc().calloc(1, AVPLAYER_SIZE) as u64;
    if obj == 0 { return 0; }
    unsafe { *(obj as *mut u64) = cls; }
    obj
}

pub extern "C" fn shim_avplayer_player_with_url(cls: u64, cmd: u64, _url: u64) -> u64 {
    shim_avplayer_alloc(cls, cmd)
}

pub extern "C" fn shim_avplayer_player_with_player_item(cls: u64, cmd: u64, _item: u64) -> u64 {
    shim_avplayer_alloc(cls, cmd)
}

pub extern "C" fn shim_avplayer_play(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avplayer_pause(_obj: u64, _cmd: u64) {}
pub extern "C" fn shim_avplayer_rate(_obj: u64, _cmd: u64) -> f32 { 0.0 }
pub extern "C" fn shim_avplayer_set_rate(_obj: u64, _cmd: u64, _rate: f32) {}
pub extern "C" fn shim_avplayer_status(_obj: u64, _cmd: u64) -> u64 { 0 /* AVPlayerStatusUnknown */ }
pub extern "C" fn shim_avplayer_current_item(_obj: u64, _cmd: u64) -> u64 { 0 }
pub extern "C" fn shim_avplayer_volume(_obj: u64, _cmd: u64) -> f32 { 1.0 }
pub extern "C" fn shim_avplayer_set_volume(_obj: u64, _cmd: u64, _v: f32) {}
