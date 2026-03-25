//! Thread-safe global diagnostic event collector.

use std::sync::Mutex;

use crate::DiagEvent;

static EVENTS: Mutex<Vec<DiagEvent>> = Mutex::new(Vec::new());

/// Record a diagnostic event.
pub fn emit(event: DiagEvent) {
    let mut events = match EVENTS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    events.push(event);
}

/// Drain all collected events, returning them and clearing the collector.
pub fn drain() -> Vec<DiagEvent> {
    let mut events = match EVENTS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    std::mem::take(&mut *events)
}

/// Return a snapshot of all collected events without clearing.
pub fn snapshot() -> Vec<DiagEvent> {
    let events = match EVENTS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    events.clone()
}

/// Check whether any events have been recorded.
pub fn has_events() -> bool {
    let events = match EVENTS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    !events.is_empty()
}

/// Clear all collected events.
pub fn clear() {
    let mut events = match EVENTS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    events.clear();
}

/// Return the number of collected events.
pub fn event_count() -> usize {
    let events = match EVENTS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    events.len()
}
