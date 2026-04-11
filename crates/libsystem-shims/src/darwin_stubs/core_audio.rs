//! CoreAudio / AudioToolbox / AudioUnit null backend.
//!
//! Provides stub implementations so audio-aware macOS binaries can launch
//! without crashing. All queries report "no audio devices available" and
//! all operations return success (or benign error codes).
//!
//! Coverage:
//! - AudioObjectGetPropertyData/Size (reports 0 devices)
//! - AudioComponentFindNext / AudioComponentInstanceNew / AudioComponentInstanceDispose
//! - AudioUnitInitialize / AudioUnitUninitialize / AudioUnitRender
//! - AudioUnitGetProperty / AudioUnitSetProperty
//! - AudioQueueNewOutput / AudioQueueStart / AudioQueueStop / AudioQueueDispose
//! - AudioSessionInitialize / AudioSessionSetProperty / AudioSessionGetProperty
//! - AudioServicesCreateSystemSoundID / AudioServicesPlaySystemSound

use std::sync::atomic::{AtomicU64, Ordering};
use crate::table::ShimTable;

// ── AudioObject (HAL level) ─────────────────────────────────────

/// kAudioObjectSystemObject = 1
static K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;

/// kAudioHardwarePropertyDevices selector
const K_AUDIO_HARDWARE_PROPERTY_DEVICES: u32 = 0x6465762E; // 'dev.'
/// kAudioHardwarePropertyDefaultOutputDevice
const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT: u32 = 0x646F7574; // 'dout'
/// kAudioHardwarePropertyDefaultInputDevice
const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT: u32 = 0x64696E20; // 'din '

/// OSStatus AudioObjectGetPropertyDataSize(AudioObjectID, const AudioObjectPropertyAddress*, ...)
/// Returns 0 bytes for all device queries (no devices).
extern "C" fn shim_audio_object_get_property_data_size(
    _obj_id: u32,
    _address: u64,
    _qualifier_size: u32,
    _qualifier: u64,
    out_size: u64,
) -> i32 {
    if out_size != 0 {
        unsafe { *(out_size as *mut u32) = 0; }
    }
    0 // noErr
}

/// OSStatus AudioObjectGetPropertyData(AudioObjectID, const AudioObjectPropertyAddress*, ...)
/// Returns empty data for all queries.
extern "C" fn shim_audio_object_get_property_data(
    _obj_id: u32,
    _address: u64,
    _qualifier_size: u32,
    _qualifier: u64,
    io_size: u64,
    _out_data: u64,
) -> i32 {
    if io_size != 0 {
        unsafe { *(io_size as *mut u32) = 0; }
    }
    0
}

/// OSStatus AudioObjectSetPropertyData(...)
extern "C" fn shim_audio_object_set_property_data(
    _obj_id: u32,
    _address: u64,
    _qualifier_size: u32,
    _qualifier: u64,
    _data_size: u32,
    _data: u64,
) -> i32 {
    0
}

/// Boolean AudioObjectHasProperty(...)
extern "C" fn shim_audio_object_has_property(
    _obj_id: u32,
    _address: u64,
) -> u8 {
    0 // false — no properties available
}

/// OSStatus AudioObjectIsPropertySettable(...)
extern "C" fn shim_audio_object_is_property_settable(
    _obj_id: u32,
    _address: u64,
    out_settable: u64,
) -> i32 {
    if out_settable != 0 {
        unsafe { *(out_settable as *mut u8) = 0; }
    }
    0
}

/// OSStatus AudioObjectAddPropertyListener(...)
extern "C" fn shim_audio_object_add_property_listener(
    _obj_id: u32,
    _address: u64,
    _callback: u64,
    _client_data: u64,
) -> i32 {
    0
}

/// OSStatus AudioObjectRemovePropertyListener(...)
extern "C" fn shim_audio_object_remove_property_listener(
    _obj_id: u32,
    _address: u64,
    _callback: u64,
    _client_data: u64,
) -> i32 {
    0
}

// ── AudioComponent / AudioUnit ──────────────────────────────────

static NEXT_UNIT: AtomicU64 = AtomicU64::new(0xA001_0001);

/// AudioComponent AudioComponentFindNext(AudioComponent, const AudioComponentDescription*)
/// Returns a fake component handle.
extern "C" fn shim_audio_component_find_next(
    _prev: u64,
    _desc: u64,
) -> u64 {
    NEXT_UNIT.fetch_add(1, Ordering::Relaxed)
}

/// UInt32 AudioComponentCount(const AudioComponentDescription*)
extern "C" fn shim_audio_component_count(_desc: u64) -> u32 {
    0
}

/// OSStatus AudioComponentInstanceNew(AudioComponent, AudioComponentInstance*)
extern "C" fn shim_audio_component_instance_new(
    _component: u64,
    out_instance: u64,
) -> i32 {
    if out_instance != 0 {
        let handle = NEXT_UNIT.fetch_add(1, Ordering::Relaxed);
        unsafe { *(out_instance as *mut u64) = handle; }
    }
    0
}

/// OSStatus AudioComponentInstanceDispose(AudioComponentInstance)
extern "C" fn shim_audio_component_instance_dispose(_instance: u64) -> i32 {
    0
}

/// OSStatus AudioUnitInitialize(AudioUnit)
extern "C" fn shim_audio_unit_initialize(_unit: u64) -> i32 { 0 }

/// OSStatus AudioUnitUninitialize(AudioUnit)
extern "C" fn shim_audio_unit_uninitialize(_unit: u64) -> i32 { 0 }

/// OSStatus AudioUnitRender(AudioUnit, AudioUnitRenderActionFlags*, const AudioTimeStamp*, UInt32, UInt32, AudioBufferList*)
extern "C" fn shim_audio_unit_render(
    _unit: u64,
    _flags: u64,
    _timestamp: u64,
    _bus: u32,
    _frames: u32,
    _buffer_list: u64,
) -> i32 {
    // No devices — return "hardware not running"
    -66785_i32 // kAudioUnitErr_CannotDoInCurrentContext
}

/// OSStatus AudioUnitGetProperty(AudioUnit, AudioUnitPropertyID, AudioUnitScope, AudioUnitElement, void*, UInt32*)
extern "C" fn shim_audio_unit_get_property(
    _unit: u64,
    _prop_id: u32,
    _scope: u32,
    _element: u32,
    _data: u64,
    io_size: u64,
) -> i32 {
    if io_size != 0 {
        unsafe { *(io_size as *mut u32) = 0; }
    }
    0
}

/// OSStatus AudioUnitSetProperty(AudioUnit, AudioUnitPropertyID, AudioUnitScope, AudioUnitElement, const void*, UInt32)
extern "C" fn shim_audio_unit_set_property(
    _unit: u64,
    _prop_id: u32,
    _scope: u32,
    _element: u32,
    _data: u64,
    _data_size: u32,
) -> i32 {
    0
}

// ── AudioQueue ──────────────────────────────────────────────────

static NEXT_QUEUE: AtomicU64 = AtomicU64::new(0xA002_0001);

/// OSStatus AudioQueueNewOutput(const AudioStreamBasicDescription*, AudioQueueOutputCallback, void*, CFRunLoopRef, CFStringRef, UInt32, AudioQueueRef*)
extern "C" fn shim_audio_queue_new_output(
    _format: u64,
    _callback: u64,
    _user_data: u64,
    _run_loop: u64,
    _run_loop_mode: u64,
    _flags: u32,
    out_queue: u64,
) -> i32 {
    if out_queue != 0 {
        let handle = NEXT_QUEUE.fetch_add(1, Ordering::Relaxed);
        unsafe { *(out_queue as *mut u64) = handle; }
    }
    0
}

/// OSStatus AudioQueueNewInput(...)
extern "C" fn shim_audio_queue_new_input(
    _format: u64,
    _callback: u64,
    _user_data: u64,
    _run_loop: u64,
    _run_loop_mode: u64,
    _flags: u32,
    out_queue: u64,
) -> i32 {
    if out_queue != 0 {
        let handle = NEXT_QUEUE.fetch_add(1, Ordering::Relaxed);
        unsafe { *(out_queue as *mut u64) = handle; }
    }
    0
}

/// OSStatus AudioQueueStart(AudioQueueRef, const AudioTimeStamp*)
extern "C" fn shim_audio_queue_start(_queue: u64, _timestamp: u64) -> i32 { 0 }

/// OSStatus AudioQueueStop(AudioQueueRef, Boolean)
extern "C" fn shim_audio_queue_stop(_queue: u64, _immediate: u8) -> i32 { 0 }

/// OSStatus AudioQueuePause(AudioQueueRef)
extern "C" fn shim_audio_queue_pause(_queue: u64) -> i32 { 0 }

/// OSStatus AudioQueueFlush(AudioQueueRef)
extern "C" fn shim_audio_queue_flush(_queue: u64) -> i32 { 0 }

/// OSStatus AudioQueueReset(AudioQueueRef)
extern "C" fn shim_audio_queue_reset(_queue: u64) -> i32 { 0 }

/// OSStatus AudioQueueDispose(AudioQueueRef, Boolean)
extern "C" fn shim_audio_queue_dispose(_queue: u64, _immediate: u8) -> i32 { 0 }

/// OSStatus AudioQueueAllocateBuffer(AudioQueueRef, UInt32, AudioQueueBufferRef*)
extern "C" fn shim_audio_queue_allocate_buffer(
    _queue: u64,
    _byte_size: u32,
    out_buf: u64,
) -> i32 {
    // Allocate a minimal fake buffer
    if out_buf != 0 {
        let buf = unsafe { libc::calloc(1, 64) } as u64;
        unsafe { *(out_buf as *mut u64) = buf; }
    }
    0
}

/// OSStatus AudioQueueFreeBuffer(AudioQueueRef, AudioQueueBufferRef)
extern "C" fn shim_audio_queue_free_buffer(_queue: u64, buf: u64) -> i32 {
    if buf != 0 {
        unsafe { libc::free(buf as *mut libc::c_void); }
    }
    0
}

/// OSStatus AudioQueueEnqueueBuffer(AudioQueueRef, AudioQueueBufferRef, UInt32, const AudioStreamPacketDescription*)
extern "C" fn shim_audio_queue_enqueue_buffer(
    _queue: u64,
    _buf: u64,
    _num_descs: u32,
    _descs: u64,
) -> i32 {
    0
}

/// OSStatus AudioQueueSetProperty(AudioQueueRef, AudioQueuePropertyID, const void*, UInt32)
extern "C" fn shim_audio_queue_set_property(
    _queue: u64,
    _prop_id: u32,
    _data: u64,
    _data_size: u32,
) -> i32 {
    0
}

/// OSStatus AudioQueueGetProperty(AudioQueueRef, AudioQueuePropertyID, void*, UInt32*)
extern "C" fn shim_audio_queue_get_property(
    _queue: u64,
    _prop_id: u32,
    _data: u64,
    io_size: u64,
) -> i32 {
    if io_size != 0 {
        unsafe { *(io_size as *mut u32) = 0; }
    }
    0
}

// ── AudioSession ────────────────────────────────────────────────

/// OSStatus AudioSessionInitialize(CFRunLoopRef, CFStringRef, AudioSessionInterruptionListener, void*)
extern "C" fn shim_audio_session_initialize(
    _run_loop: u64,
    _mode: u64,
    _listener: u64,
    _client_data: u64,
) -> i32 {
    0
}

/// OSStatus AudioSessionSetProperty(AudioSessionPropertyID, UInt32, const void*)
extern "C" fn shim_audio_session_set_property(
    _prop_id: u32,
    _data_size: u32,
    _data: u64,
) -> i32 {
    0
}

/// OSStatus AudioSessionGetProperty(AudioSessionPropertyID, UInt32*, void*)
extern "C" fn shim_audio_session_get_property(
    _prop_id: u32,
    io_size: u64,
    _data: u64,
) -> i32 {
    if io_size != 0 {
        unsafe { *(io_size as *mut u32) = 0; }
    }
    0
}

/// OSStatus AudioSessionSetActive(Boolean)
extern "C" fn shim_audio_session_set_active(_active: u8) -> i32 { 0 }

/// OSStatus AudioSessionAddPropertyListener(...)
extern "C" fn shim_audio_session_add_property_listener(
    _prop_id: u32,
    _callback: u64,
    _client_data: u64,
) -> i32 {
    0
}

// ── System sounds ───────────────────────────────────────────────

/// OSStatus AudioServicesCreateSystemSoundID(CFURLRef, SystemSoundID*)
extern "C" fn shim_audio_services_create_system_sound_id(
    _url: u64,
    out_id: u64,
) -> i32 {
    if out_id != 0 {
        unsafe { *(out_id as *mut u32) = 1; }
    }
    0
}

/// void AudioServicesPlaySystemSound(SystemSoundID)
extern "C" fn shim_audio_services_play_system_sound(_sound_id: u32) {
    // No-op: no audio hardware
}

/// OSStatus AudioServicesDisposeSystemSoundID(SystemSoundID)
extern "C" fn shim_audio_services_dispose_system_sound_id(_sound_id: u32) -> i32 {
    0
}

/// void AudioServicesPlayAlertSound(SystemSoundID)
extern "C" fn shim_audio_services_play_alert_sound(_sound_id: u32) {
    // No-op
}

// ── Install ─────────────────────────────────────────────────────

pub struct CoreAudioStubs;

impl CoreAudioStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            // AudioObject (HAL)
            ("_AudioObjectGetPropertyDataSize", shim_audio_object_get_property_data_size as *const () as u64, "AudioObjectGetPropertyDataSize → 0 bytes"),
            ("_AudioObjectGetPropertyData", shim_audio_object_get_property_data as *const () as u64, "AudioObjectGetPropertyData → empty"),
            ("_AudioObjectSetPropertyData", shim_audio_object_set_property_data as *const () as u64, "AudioObjectSetPropertyData → ok"),
            ("_AudioObjectHasProperty", shim_audio_object_has_property as *const () as u64, "AudioObjectHasProperty → false"),
            ("_AudioObjectIsPropertySettable", shim_audio_object_is_property_settable as *const () as u64, "AudioObjectIsPropertySettable → false"),
            ("_AudioObjectAddPropertyListener", shim_audio_object_add_property_listener as *const () as u64, "AudioObjectAddPropertyListener → ok"),
            ("_AudioObjectRemovePropertyListener", shim_audio_object_remove_property_listener as *const () as u64, "AudioObjectRemovePropertyListener → ok"),
            // AudioComponent / AudioUnit
            ("_AudioComponentFindNext", shim_audio_component_find_next as *const () as u64, "AudioComponentFindNext → fake handle"),
            ("_AudioComponentCount", shim_audio_component_count as *const () as u64, "AudioComponentCount → 0"),
            ("_AudioComponentInstanceNew", shim_audio_component_instance_new as *const () as u64, "AudioComponentInstanceNew → fake handle"),
            ("_AudioComponentInstanceDispose", shim_audio_component_instance_dispose as *const () as u64, "AudioComponentInstanceDispose → ok"),
            ("_AudioUnitInitialize", shim_audio_unit_initialize as *const () as u64, "AudioUnitInitialize → ok"),
            ("_AudioUnitUninitialize", shim_audio_unit_uninitialize as *const () as u64, "AudioUnitUninitialize → ok"),
            ("_AudioUnitRender", shim_audio_unit_render as *const () as u64, "AudioUnitRender → err (no device)"),
            ("_AudioUnitGetProperty", shim_audio_unit_get_property as *const () as u64, "AudioUnitGetProperty → empty"),
            ("_AudioUnitSetProperty", shim_audio_unit_set_property as *const () as u64, "AudioUnitSetProperty → ok"),
            // AudioQueue
            ("_AudioQueueNewOutput", shim_audio_queue_new_output as *const () as u64, "AudioQueueNewOutput → fake handle"),
            ("_AudioQueueNewInput", shim_audio_queue_new_input as *const () as u64, "AudioQueueNewInput → fake handle"),
            ("_AudioQueueStart", shim_audio_queue_start as *const () as u64, "AudioQueueStart → ok"),
            ("_AudioQueueStop", shim_audio_queue_stop as *const () as u64, "AudioQueueStop → ok"),
            ("_AudioQueuePause", shim_audio_queue_pause as *const () as u64, "AudioQueuePause → ok"),
            ("_AudioQueueFlush", shim_audio_queue_flush as *const () as u64, "AudioQueueFlush → ok"),
            ("_AudioQueueReset", shim_audio_queue_reset as *const () as u64, "AudioQueueReset → ok"),
            ("_AudioQueueDispose", shim_audio_queue_dispose as *const () as u64, "AudioQueueDispose → ok"),
            ("_AudioQueueAllocateBuffer", shim_audio_queue_allocate_buffer as *const () as u64, "AudioQueueAllocateBuffer → fake buffer"),
            ("_AudioQueueFreeBuffer", shim_audio_queue_free_buffer as *const () as u64, "AudioQueueFreeBuffer → ok"),
            ("_AudioQueueEnqueueBuffer", shim_audio_queue_enqueue_buffer as *const () as u64, "AudioQueueEnqueueBuffer → ok"),
            ("_AudioQueueSetProperty", shim_audio_queue_set_property as *const () as u64, "AudioQueueSetProperty → ok"),
            ("_AudioQueueGetProperty", shim_audio_queue_get_property as *const () as u64, "AudioQueueGetProperty → empty"),
            // AudioSession
            ("_AudioSessionInitialize", shim_audio_session_initialize as *const () as u64, "AudioSessionInitialize → ok"),
            ("_AudioSessionSetProperty", shim_audio_session_set_property as *const () as u64, "AudioSessionSetProperty → ok"),
            ("_AudioSessionGetProperty", shim_audio_session_get_property as *const () as u64, "AudioSessionGetProperty → empty"),
            ("_AudioSessionSetActive", shim_audio_session_set_active as *const () as u64, "AudioSessionSetActive → ok"),
            ("_AudioSessionAddPropertyListener", shim_audio_session_add_property_listener as *const () as u64, "AudioSessionAddPropertyListener → ok"),
            // System sounds
            ("_AudioServicesCreateSystemSoundID", shim_audio_services_create_system_sound_id as *const () as u64, "AudioServicesCreateSystemSoundID → 1"),
            ("_AudioServicesPlaySystemSound", shim_audio_services_play_system_sound as *const () as u64, "AudioServicesPlaySystemSound → no-op"),
            ("_AudioServicesPlayAlertSound", shim_audio_services_play_alert_sound as *const () as u64, "AudioServicesPlayAlertSound → no-op"),
            ("_AudioServicesDisposeSystemSoundID", shim_audio_services_dispose_system_sound_id as *const () as u64, "AudioServicesDisposeSystemSoundID → ok"),
            // Data symbol
            ("_kAudioObjectSystemObject", &K_AUDIO_OBJECT_SYSTEM_OBJECT as *const u32 as u64, "kAudioObjectSystemObject = 1")
        );

        log::debug!("CoreAudioStubs: registered 39 shims + 1 data symbol");
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_object_reports_no_devices() {
        let mut size: u32 = 999;
        let rc = shim_audio_object_get_property_data_size(
            1, // kAudioObjectSystemObject
            0,
            0,
            0,
            &mut size as *mut u32 as u64,
        );
        assert_eq!(rc, 0);
        assert_eq!(size, 0);
    }

    #[test]
    fn audio_component_find_next_returns_handle() {
        let h = shim_audio_component_find_next(0, 0);
        assert_ne!(h, 0);
    }

    #[test]
    fn audio_unit_lifecycle() {
        let mut instance: u64 = 0;
        let rc = shim_audio_component_instance_new(
            0x1234,
            &mut instance as *mut u64 as u64,
        );
        assert_eq!(rc, 0);
        assert_ne!(instance, 0);
        assert_eq!(shim_audio_unit_initialize(instance), 0);
        assert_eq!(shim_audio_unit_uninitialize(instance), 0);
        assert_eq!(shim_audio_component_instance_dispose(instance), 0);
    }

    #[test]
    fn audio_queue_lifecycle() {
        let mut queue: u64 = 0;
        let rc = shim_audio_queue_new_output(
            0, 0, 0, 0, 0, 0,
            &mut queue as *mut u64 as u64,
        );
        assert_eq!(rc, 0);
        assert_ne!(queue, 0);
        assert_eq!(shim_audio_queue_start(queue, 0), 0);
        assert_eq!(shim_audio_queue_stop(queue, 1), 0);
        assert_eq!(shim_audio_queue_dispose(queue, 1), 0);
    }

    #[test]
    fn audio_session_init_and_property() {
        assert_eq!(shim_audio_session_initialize(0, 0, 0, 0), 0);
        assert_eq!(shim_audio_session_set_active(1), 0);
        assert_eq!(shim_audio_session_set_property(0, 0, 0), 0);
    }

    #[test]
    fn system_sound_lifecycle() {
        let mut sound_id: u32 = 0;
        let rc = shim_audio_services_create_system_sound_id(
            0x1234,
            &mut sound_id as *mut u32 as u64,
        );
        assert_eq!(rc, 0);
        assert_eq!(sound_id, 1);
        shim_audio_services_play_system_sound(sound_id);
        assert_eq!(shim_audio_services_dispose_system_sound_id(sound_id), 0);
    }
}
