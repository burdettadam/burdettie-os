//! Real word/line break iteration backed by icu_segmenter.

use icu_segmenter::options::WordBreakInvariantOptions;
use icu_segmenter::{WordSegmenter, WordSegmenterBorrowed};
use libsystem_shims::register_shims;
use libsystem_shims::table::ShimTable;

/// Create a word segmenter from compiled data.
/// With compiled_data, creation is essentially pointer setup to baked-in statics.
fn word_segmenter() -> WordSegmenterBorrowed<'static> {
    WordSegmenter::new_auto(WordBreakInvariantOptions::default())
}

pub fn install(table: &mut ShimTable) {
    register_shims!(
        table,
        (
            "_ubrk_open",
            shim_ubrk_open as *const () as u64,
            "ubrk_open (icu4x → real segmenter)"
        ),
        (
            "_ubrk_close",
            shim_ubrk_close as *const () as u64,
            "ubrk_close (icu4x)"
        ),
        (
            "_ubrk_setText",
            shim_ubrk_set_text as *const () as u64,
            "ubrk_setText (icu4x)"
        ),
        (
            "_ubrk_first",
            shim_ubrk_first as *const () as u64,
            "ubrk_first (icu4x → 0)"
        ),
        (
            "_ubrk_next",
            shim_ubrk_next as *const () as u64,
            "ubrk_next (icu4x → real breaks)"
        )
    );
}

/// State for an active break iterator.
/// Stores the pre-computed break positions for the current text.
struct BreakIterState {
    positions: Vec<i32>,
    cursor: usize,
}

/// We store break iterator state in a leaked Box (since guest code
/// holds the pointer and calls ubrk_close to free it).
/// The ubrk_open/ubrk_close pattern means each iterator is independent.

/// UBRK_DONE sentinel value
const UBRK_DONE: i32 = -1;

/// ubrk_open(type, locale, text, textLength, status) → UBreakIterator*
pub extern "C" fn shim_ubrk_open(
    _type: i32,
    _locale: u64,
    text: *const u16,
    text_len: i32,
    status: *mut i32,
) -> u64 {
    if !status.is_null() {
        unsafe {
            *status = 0;
        }
    }

    let segmenter = word_segmenter();

    let positions = if !text.is_null() && text_len != 0 {
        let slice = if text_len < 0 {
            unsafe { read_u16_until_null(text) }
        } else {
            unsafe { std::slice::from_raw_parts(text, text_len as usize) }
        };
        segmenter
            .segment_utf16(slice)
            .collect::<Vec<usize>>()
            .iter()
            .map(|&p| p as i32)
            .collect()
    } else {
        vec![]
    };

    let state = Box::new(BreakIterState {
        positions,
        cursor: 0,
    });
    Box::into_raw(state) as u64
}

/// ubrk_close(bi)
pub extern "C" fn shim_ubrk_close(bi: u64) {
    if bi != 0 {
        unsafe {
            drop(Box::from_raw(bi as *mut BreakIterState));
        }
    }
}

/// ubrk_setText(bi, text, textLength, status)
pub extern "C" fn shim_ubrk_set_text(bi: u64, text: *const u16, text_len: i32, status: *mut i32) {
    if !status.is_null() {
        unsafe {
            *status = 0;
        }
    }
    if bi == 0 {
        return;
    }

    let segmenter = word_segmenter();
    let state = unsafe { &mut *(bi as *mut BreakIterState) };

    if !text.is_null() && text_len != 0 {
        let slice = if text_len < 0 {
            unsafe { read_u16_until_null(text) }
        } else {
            unsafe { std::slice::from_raw_parts(text, text_len as usize) }
        };
        state.positions = segmenter
            .segment_utf16(slice)
            .collect::<Vec<usize>>()
            .iter()
            .map(|&p| p as i32)
            .collect();
    } else {
        state.positions.clear();
    }
    state.cursor = 0;
}

/// ubrk_first(bi) → int32_t (always 0, the start position)
pub extern "C" fn shim_ubrk_first(bi: u64) -> i32 {
    if bi != 0 {
        let state = unsafe { &mut *(bi as *mut BreakIterState) };
        state.cursor = 0;
    }
    0
}

/// ubrk_next(bi) → int32_t (next break position or UBRK_DONE)
pub extern "C" fn shim_ubrk_next(bi: u64) -> i32 {
    if bi == 0 {
        return UBRK_DONE;
    }

    let state = unsafe { &mut *(bi as *mut BreakIterState) };
    if state.cursor < state.positions.len() {
        let pos = state.positions[state.cursor];
        state.cursor += 1;
        pos
    } else {
        UBRK_DONE
    }
}

unsafe fn read_u16_until_null<'a>(ptr: *const u16) -> &'a [u16] {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
        if len > 1_000_000 {
            break;
        }
    }
    std::slice::from_raw_parts(ptr, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_segmenter_initializes() {
        let _ = word_segmenter();
    }

    #[test]
    fn ubrk_segments_hello_world() {
        let text: Vec<u16> = "Hello World".encode_utf16().collect();
        let mut status = 0i32;
        let bi = shim_ubrk_open(1, 0, text.as_ptr(), text.len() as i32, &mut status);
        assert_ne!(bi, 0);
        assert_eq!(status, 0);

        let first = shim_ubrk_first(bi);
        assert_eq!(first, 0);

        // Collect all break positions
        let mut breaks = vec![];
        loop {
            let pos = shim_ubrk_next(bi);
            if pos == UBRK_DONE {
                break;
            }
            breaks.push(pos);
        }
        // Word segmenter should find breaks (at least at word boundaries)
        assert!(!breaks.is_empty());
        // The last break should be at or near the end of the string
        assert!(breaks.last().copied().unwrap_or(0) <= text.len() as i32);

        shim_ubrk_close(bi);
    }

    #[test]
    fn ubrk_done_on_empty() {
        let mut status = 0i32;
        let bi = shim_ubrk_open(1, 0, std::ptr::null(), 0, &mut status);
        assert_ne!(bi, 0);
        assert_eq!(shim_ubrk_next(bi), UBRK_DONE);
        shim_ubrk_close(bi);
    }
}
