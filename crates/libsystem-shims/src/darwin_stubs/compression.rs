//! Apple Compression framework shims.
//!
//! Provides `compression_encode_buffer`, `compression_decode_buffer`, and the
//! streaming API (`compression_stream_init`/`process`/`destroy`).
//!
//! Real compression backed by open-source crates:
//!   - ZLIB: `flate2` (miniz_oxide backend, pure Rust)
//!   - LZ4:  `lz4_flex` (pure Rust LZ4 block codec)
//!   - LZMA: `lzma-rs` (pure Rust LZMA/XZ decoder, encode unsupported)
//!   - LZFSE: not yet supported (returns 0)

use crate::table::ShimTable;

// ── Algorithm constants ──────────────────────────────────────────────────────

const COMPRESSION_LZ4: u32 = 0x100;
const COMPRESSION_LZ4_RAW: u32 = 0x101;
const COMPRESSION_ZLIB: u32 = 0x205;
const COMPRESSION_LZMA: u32 = 0x306;
const COMPRESSION_LZFSE: u32 = 0x801;

// ── Status / flag constants ──────────────────────────────────────────────────

const COMPRESSION_STATUS_OK: i32 = 0;
const COMPRESSION_STATUS_ERROR: i32 = -1;
const COMPRESSION_STATUS_END: i32 = 1;

const COMPRESSION_STREAM_FINALIZE: i32 = 0x0001;

const COMPRESSION_STREAM_ENCODE: i32 = 0;
const COMPRESSION_STREAM_DECODE: i32 = 1;

// ── compression_stream layout (matches Apple's C struct) ─────────────────────
//
//   [0..8)   src_ptr   (const uint8_t *)
//   [8..16)  src_size  (size_t)
//   [16..24) dst_ptr   (uint8_t *)
//   [24..32) dst_size  (size_t)
//   [32..40) state     (void *)

const STREAM_SRC_PTR: usize = 0;
const STREAM_SRC_SIZE: usize = 8;
const STREAM_DST_PTR: usize = 16;
const STREAM_DST_SIZE: usize = 24;
const STREAM_STATE: usize = 32;

// ── Shim registration ────────────────────────────────────────────────────────

pub struct CompressionStubs;

impl CompressionStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(table,
            ("_compression_encode_buffer", shim_compression_encode_buffer as *const () as u64, "compression_encode_buffer"),
            ("_compression_decode_buffer", shim_compression_decode_buffer as *const () as u64, "compression_decode_buffer"),
            ("_compression_stream_init", shim_compression_stream_init as *const () as u64, "compression_stream_init"),
            ("_compression_stream_process", shim_compression_stream_process as *const () as u64, "compression_stream_process"),
            ("_compression_stream_destroy", shim_compression_stream_destroy as *const () as u64, "compression_stream_destroy"),
            ("_compression_encode_scratch_buffer_size", shim_compression_encode_scratch_buffer_size as *const () as u64, "compression_encode_scratch_buffer_size"),
            ("_compression_decode_scratch_buffer_size", shim_compression_decode_scratch_buffer_size as *const () as u64, "compression_decode_scratch_buffer_size"),
        );
        log::debug!("CompressionStubs: registered 7 stubs");
    }
}

// ── Adler-32 ─────────────────────────────────────────────────────────────────

fn adler32(data: &[u8]) -> u32 {
    const MOD_ADLER: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }
    (b << 16) | a
}

// ── ZLIB via flate2 (real deflate compression) ───────────────────────────────

fn zlib_encode(src: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = encoder.write_all(src);
    encoder.finish().unwrap_or_default()
}

fn zlib_decode(src: &[u8]) -> Option<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(src);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).ok()?;
    Some(out)
}

// ── LZ4 via lz4_flex (real LZ4 block compression) ────────────────────────────

/// Encode using lz4_flex real LZ4 block compression.
fn lz4_encode(src: &[u8]) -> Vec<u8> {
    if src.is_empty() {
        return Vec::new();
    }
    lz4_flex::block::compress_prepend_size(src)
}

/// Decode LZ4 block format using lz4_flex.
fn lz4_decode(src: &[u8], max_output: usize) -> Option<Vec<u8>> {
    // Try prepended-size format first (lz4_flex default)
    if let Ok(decoded) = lz4_flex::block::decompress_size_prepended(src) {
        if decoded.len() <= max_output {
            return Some(decoded);
        }
    }
    // Fall back to uncompressed-size variant
    lz4_flex::block::decompress(src, max_output).ok()
}

// ── LZMA via lzma-rs (real LZMA decompression) ──────────────────────────────

fn lzma_decode(src: &[u8]) -> Option<Vec<u8>> {
    use std::io::BufReader;

    // Try LZMA standalone format first
    let mut out = Vec::new();
    if lzma_rs::lzma_decompress(&mut BufReader::new(src), &mut out).is_ok() {
        return Some(out);
    }

    // Try XZ container format
    out.clear();
    let mut reader = BufReader::new(src);
    if lzma_rs::xz_decompress(&mut reader, &mut out).is_ok() {
        return Some(out);
    }

    None
}

// ── One-shot shims ───────────────────────────────────────────────────────────

pub extern "C" fn shim_compression_encode_buffer(
    dst: u64,
    dst_size: usize,
    src: u64,
    src_size: usize,
    _scratch: u64,
    algo: u32,
) -> usize {
    let src_slice = if src_size == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(src as *const u8, src_size) }
    };

    let encoded = match algo {
        COMPRESSION_ZLIB => zlib_encode(src_slice),
        COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => lz4_encode(src_slice),
        COMPRESSION_LZMA => {
            log::debug!("compression_encode_buffer: LZMA encode not supported");
            return 0;
        }
        _ => {
            log::debug!("compression_encode_buffer: unsupported algo 0x{:x}", algo);
            return 0;
        }
    };

    if encoded.len() > dst_size {
        return 0;
    }
    if !encoded.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(encoded.as_ptr(), dst as *mut u8, encoded.len());
        }
    }
    encoded.len()
}

pub extern "C" fn shim_compression_decode_buffer(
    dst: u64,
    dst_size: usize,
    src: u64,
    src_size: usize,
    _scratch: u64,
    algo: u32,
) -> usize {
    if src_size == 0 {
        return 0;
    }
    let src_slice = unsafe { std::slice::from_raw_parts(src as *const u8, src_size) };

    let decoded = match algo {
        COMPRESSION_ZLIB => match zlib_decode(src_slice) {
            Some(d) => d,
            None => return 0,
        },
        COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => match lz4_decode(src_slice, dst_size) {
            Some(d) => d,
            None => return 0,
        },
        COMPRESSION_LZMA => match lzma_decode(src_slice) {
            Some(d) => d,
            None => return 0,
        },
        _ => {
            log::debug!("compression_decode_buffer: unsupported algo 0x{:x}", algo);
            return 0;
        }
    };

    if decoded.len() > dst_size {
        return 0;
    }
    if !decoded.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(decoded.as_ptr(), dst as *mut u8, decoded.len());
        }
    }
    decoded.len()
}

// ── Scratch buffer size shims ────────────────────────────────────────────────

pub extern "C" fn shim_compression_encode_scratch_buffer_size(_algo: u32) -> usize {
    0
}

pub extern "C" fn shim_compression_decode_scratch_buffer_size(_algo: u32) -> usize {
    0
}

// ── Streaming API shims ──────────────────────────────────────────────────────

/// Per-stream internal state, heap-allocated and referenced through the
/// stream's `state` pointer (we store a `Box` pointer there).
struct StreamState {
    operation: i32,
    algo: u32,
    buffer: Vec<u8>,
    finished: bool,
}

unsafe fn read_stream_field<T: Copy>(stream: u64, offset: usize) -> T {
    let ptr = (stream as *const u8).add(offset) as *const T;
    ptr.read_unaligned()
}

unsafe fn write_stream_field<T: Copy>(stream: u64, offset: usize, val: T) {
    let ptr = (stream as *mut u8).add(offset) as *mut T;
    ptr.write_unaligned(val);
}

pub extern "C" fn shim_compression_stream_init(
    stream: u64,
    operation: i32,
    algo: u32,
) -> i32 {
    if stream == 0 {
        return COMPRESSION_STATUS_ERROR;
    }
    match algo {
        COMPRESSION_ZLIB | COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW | COMPRESSION_LZMA => {}
        _ => {
            log::debug!("compression_stream_init: unsupported algo 0x{:x}", algo);
            return COMPRESSION_STATUS_ERROR;
        }
    }
    if operation != COMPRESSION_STREAM_ENCODE && operation != COMPRESSION_STREAM_DECODE {
        return COMPRESSION_STATUS_ERROR;
    }

    let state = Box::new(StreamState {
        operation,
        algo,
        buffer: Vec::new(),
        finished: false,
    });
    let state_ptr = Box::into_raw(state) as u64;
    unsafe {
        write_stream_field(stream, STREAM_STATE, state_ptr);
    }
    COMPRESSION_STATUS_OK
}

pub extern "C" fn shim_compression_stream_process(stream: u64, flags: i32) -> i32 {
    if stream == 0 {
        return COMPRESSION_STATUS_ERROR;
    }

    let state_ptr: u64 = unsafe { read_stream_field(stream, STREAM_STATE) };
    if state_ptr == 0 {
        return COMPRESSION_STATUS_ERROR;
    }
    let state = unsafe { &mut *(state_ptr as *mut StreamState) };

    if state.finished {
        return COMPRESSION_STATUS_END;
    }

    // Read available input from the stream and buffer it.
    let src_ptr: u64 = unsafe { read_stream_field(stream, STREAM_SRC_PTR) };
    let src_size: usize = unsafe { read_stream_field(stream, STREAM_SRC_SIZE) };

    if src_size > 0 && src_ptr != 0 {
        let input = unsafe { std::slice::from_raw_parts(src_ptr as *const u8, src_size) };
        state.buffer.extend_from_slice(input);
        // Advance src: all consumed.
        unsafe {
            write_stream_field(stream, STREAM_SRC_PTR, src_ptr + src_size as u64);
            write_stream_field(stream, STREAM_SRC_SIZE, 0usize);
        }
    }

    let finalize = (flags & COMPRESSION_STREAM_FINALIZE) != 0;
    if !finalize {
        return COMPRESSION_STATUS_OK;
    }

    // Finalize: encode or decode the buffered data in one shot.
    let dst_ptr: u64 = unsafe { read_stream_field(stream, STREAM_DST_PTR) };
    let dst_size: usize = unsafe { read_stream_field(stream, STREAM_DST_SIZE) };

    let result = if state.operation == COMPRESSION_STREAM_ENCODE {
        match state.algo {
            COMPRESSION_ZLIB => zlib_encode(&state.buffer),
            COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => lz4_encode(&state.buffer),
            COMPRESSION_LZMA => return COMPRESSION_STATUS_ERROR, // encode not supported
            _ => return COMPRESSION_STATUS_ERROR,
        }
    } else {
        match state.algo {
            COMPRESSION_ZLIB => match zlib_decode(&state.buffer) {
                Some(d) => d,
                None => return COMPRESSION_STATUS_ERROR,
            },
            COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => match lz4_decode(&state.buffer, dst_size) {
                Some(d) => d,
                None => return COMPRESSION_STATUS_ERROR,
            },
            COMPRESSION_LZMA => match lzma_decode(&state.buffer) {
                Some(d) => d,
                None => return COMPRESSION_STATUS_ERROR,
            },
            _ => return COMPRESSION_STATUS_ERROR,
        }
    };

    if result.len() > dst_size {
        return COMPRESSION_STATUS_ERROR;
    }
    if !result.is_empty() && dst_ptr != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(result.as_ptr(), dst_ptr as *mut u8, result.len());
        }
    }

    // Update dst_ptr / dst_size to reflect how much was written.
    unsafe {
        write_stream_field(stream, STREAM_DST_PTR, dst_ptr + result.len() as u64);
        write_stream_field(stream, STREAM_DST_SIZE, dst_size - result.len());
    }

    state.finished = true;
    COMPRESSION_STATUS_END
}

pub extern "C" fn shim_compression_stream_destroy(stream: u64) -> i32 {
    if stream == 0 {
        return COMPRESSION_STATUS_ERROR;
    }
    let state_ptr: u64 = unsafe { read_stream_field(stream, STREAM_STATE) };
    if state_ptr != 0 {
        // Reclaim the heap allocation.
        let _ = unsafe { Box::from_raw(state_ptr as *mut StreamState) };
        unsafe {
            write_stream_field(stream, STREAM_STATE, 0u64);
        }
    }
    COMPRESSION_STATUS_OK
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adler32_known() {
        // Known value: adler32("Wikipedia") == 0x11E60398
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn test_zlib_encode_decode_roundtrip() {
        let input = b"hello world";
        let encoded = zlib_encode(input);
        let decoded = zlib_decode(&encoded).expect("decode failed");
        assert_eq!(&decoded, input);
    }

    #[test]
    fn test_zlib_encode_large() {
        // 100 KB of patterned data
        let input: Vec<u8> = (0..100_000).map(|i| (i % 251) as u8).collect();
        let encoded = zlib_encode(&input);
        let decoded = zlib_decode(&encoded).expect("decode failed");
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_lz4_encode_decode_roundtrip() {
        let input = b"hello world";
        let encoded = lz4_encode(input);
        let decoded = lz4_decode(&encoded, 1024).expect("decode failed");
        assert_eq!(&decoded, &input[..]);
    }

    #[test]
    fn test_unsupported_algo_returns_zero() {
        let src = b"test data";
        let mut dst = vec![0u8; 256];
        let result = shim_compression_encode_buffer(
            dst.as_mut_ptr() as u64,
            dst.len(),
            src.as_ptr() as u64,
            src.len(),
            0,
            COMPRESSION_LZMA,
        );
        assert_eq!(result, 0);
    }

    #[test]
    fn test_scratch_buffer_sizes() {
        assert_eq!(shim_compression_encode_scratch_buffer_size(COMPRESSION_ZLIB), 0);
        assert_eq!(shim_compression_decode_scratch_buffer_size(COMPRESSION_LZ4), 0);
        assert_eq!(shim_compression_encode_scratch_buffer_size(COMPRESSION_LZFSE), 0);
    }

    #[test]
    fn test_empty_encode() {
        let input: &[u8] = b"";
        // ZLIB: should produce valid empty zlib stream
        let encoded = zlib_encode(input);
        assert!(encoded.len() > 2); // at least header + block header + trailer
        let decoded = zlib_decode(&encoded).expect("empty decode failed");
        assert!(decoded.is_empty());

        // LZ4: empty input → empty output
        let lz4_enc = lz4_encode(input);
        assert!(lz4_enc.is_empty());
    }

    #[test]
    fn test_stream_init_destroy() {
        // Simulate a compression_stream on the stack.
        #[repr(C)]
        struct FakeStream {
            src_ptr: u64,
            src_size: u64,
            dst_ptr: u64,
            dst_size: u64,
            state: u64,
        }
        let mut stream = FakeStream {
            src_ptr: 0,
            src_size: 0,
            dst_ptr: 0,
            dst_size: 0,
            state: 0,
        };
        let stream_addr = &mut stream as *mut FakeStream as u64;

        let ret = shim_compression_stream_init(stream_addr, COMPRESSION_STREAM_ENCODE, COMPRESSION_ZLIB);
        assert_eq!(ret, COMPRESSION_STATUS_OK);
        assert_ne!(stream.state, 0, "state should be allocated");

        let ret = shim_compression_stream_destroy(stream_addr);
        assert_eq!(ret, COMPRESSION_STATUS_OK);
        assert_eq!(stream.state, 0, "state should be cleared");
    }

    #[test]
    fn test_stream_init_unsupported() {
        #[repr(C)]
        struct FakeStream {
            src_ptr: u64,
            src_size: u64,
            dst_ptr: u64,
            dst_size: u64,
            state: u64,
        }
        let mut stream = FakeStream {
            src_ptr: 0,
            src_size: 0,
            dst_ptr: 0,
            dst_size: 0,
            state: 0,
        };
        let stream_addr = &mut stream as *mut FakeStream as u64;

        let ret = shim_compression_stream_init(stream_addr, COMPRESSION_STREAM_ENCODE, COMPRESSION_LZMA);
        // LZMA is now accepted for streaming (decode supported)
        assert_eq!(ret, COMPRESSION_STATUS_OK);
        shim_compression_stream_destroy(stream_addr);
    }

    #[test]
    fn test_oneshot_zlib_via_shims() {
        let input = b"The quick brown fox jumps over the lazy dog";
        let mut encoded = vec![0u8; 1024];
        let enc_len = shim_compression_encode_buffer(
            encoded.as_mut_ptr() as u64,
            encoded.len(),
            input.as_ptr() as u64,
            input.len(),
            0,
            COMPRESSION_ZLIB,
        );
        assert!(enc_len > 0, "encode should succeed");

        let mut decoded = vec![0u8; 1024];
        let dec_len = shim_compression_decode_buffer(
            decoded.as_mut_ptr() as u64,
            decoded.len(),
            encoded.as_ptr() as u64,
            enc_len,
            0,
            COMPRESSION_ZLIB,
        );
        assert_eq!(dec_len, input.len());
        assert_eq!(&decoded[..dec_len], &input[..]);
    }

    #[test]
    fn test_oneshot_lz4_via_shims() {
        let input = b"AAAAAABBBBCCCCDDDD";
        let mut encoded = vec![0u8; 1024];
        let enc_len = shim_compression_encode_buffer(
            encoded.as_mut_ptr() as u64,
            encoded.len(),
            input.as_ptr() as u64,
            input.len(),
            0,
            COMPRESSION_LZ4,
        );
        assert!(enc_len > 0, "lz4 encode should succeed");

        let mut decoded = vec![0u8; 1024];
        let dec_len = shim_compression_decode_buffer(
            decoded.as_mut_ptr() as u64,
            decoded.len(),
            encoded.as_ptr() as u64,
            enc_len,
            0,
            COMPRESSION_LZ4,
        );
        assert_eq!(dec_len, input.len());
        assert_eq!(&decoded[..dec_len], &input[..]);
    }

    #[test]
    fn test_dst_too_small_returns_zero() {
        let input = b"some data that needs encoding";
        let mut tiny = vec![0u8; 2]; // too small for zlib header + data
        let result = shim_compression_encode_buffer(
            tiny.as_mut_ptr() as u64,
            tiny.len(),
            input.as_ptr() as u64,
            input.len(),
            0,
            COMPRESSION_ZLIB,
        );
        assert_eq!(result, 0);
    }
}
