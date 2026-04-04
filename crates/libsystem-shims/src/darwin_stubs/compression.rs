//! Apple Compression framework shims.
//!
//! Provides `compression_encode_buffer`, `compression_decode_buffer`, and the
//! streaming API (`compression_stream_init`/`process`/`destroy`).
//!
//! ZLIB is implemented as stored-only (no actual deflation) — valid zlib that
//! any real decompressor accepts.  LZ4 uses literal-only encoding for output
//! and a full block-format decoder for input.  LZMA/LZFSE return 0 (unsupported).

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

// ── ZLIB stored-block encode / decode ────────────────────────────────────────

fn zlib_encode_stored(src: &[u8]) -> Vec<u8> {
    // Header: CMF=0x78 (deflate, window 32K), FLG=0x01 (no dict, check bits)
    let mut out = Vec::with_capacity(src.len() + 64);
    out.push(0x78);
    out.push(0x01);

    let chunks = if src.is_empty() { 1 } else { (src.len() + 65534) / 65535 };
    for i in 0..chunks {
        let start = i * 65535;
        let end = std::cmp::min(start + 65535, src.len());
        let chunk = &src[start..end];
        let len = chunk.len() as u16;
        let nlen = !len;
        let bfinal: u8 = if i + 1 == chunks { 1 } else { 0 };
        // BFINAL | BTYPE=00 (stored)
        out.push(bfinal);
        out.push(len as u8);
        out.push((len >> 8) as u8);
        out.push(nlen as u8);
        out.push((nlen >> 8) as u8);
        out.extend_from_slice(chunk);
    }

    let checksum = adler32(src);
    out.push((checksum >> 24) as u8);
    out.push((checksum >> 16) as u8);
    out.push((checksum >> 8) as u8);
    out.push(checksum as u8);
    out
}

fn zlib_decode_stored(src: &[u8]) -> Option<Vec<u8>> {
    if src.len() < 6 {
        return None;
    }
    // Verify zlib header — CMF byte must indicate deflate (method 8)
    if src[0] & 0x0F != 8 {
        return None;
    }
    // Check header checksum
    let cmf_flg = (src[0] as u16) * 256 + src[1] as u16;
    if cmf_flg % 31 != 0 {
        return None;
    }

    let mut pos = 2;
    let trailer_len = 4;
    if src.len() < pos + trailer_len {
        return None;
    }
    let data_end = src.len() - trailer_len;
    let mut out = Vec::new();

    loop {
        if pos >= data_end {
            break;
        }
        if pos + 5 > data_end {
            return None;
        }
        let bfinal = src[pos] & 1;
        let btype = (src[pos] >> 1) & 3;
        if btype != 0 {
            // We only handle stored blocks
            return None;
        }
        pos += 1;
        let len = src[pos] as u16 | ((src[pos + 1] as u16) << 8);
        let nlen = src[pos + 2] as u16 | ((src[pos + 3] as u16) << 8);
        if len != !nlen {
            return None;
        }
        pos += 4;
        let block_end = pos + len as usize;
        if block_end > data_end {
            return None;
        }
        out.extend_from_slice(&src[pos..block_end]);
        pos = block_end;
        if bfinal != 0 {
            break;
        }
    }

    // Verify adler-32 trailer
    let expected = ((src[data_end] as u32) << 24)
        | ((src[data_end + 1] as u32) << 16)
        | ((src[data_end + 2] as u32) << 8)
        | (src[data_end + 3] as u32);
    if adler32(&out) != expected {
        return None;
    }
    Some(out)
}

// ── LZ4 block encode / decode ────────────────────────────────────────────────

/// Encode using literal-only sequences (valid LZ4 that any decoder accepts).
fn lz4_encode_literals(src: &[u8]) -> Vec<u8> {
    if src.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(src.len() + src.len() / 255 + 16);
    let mut offset = 0;

    while offset < src.len() {
        let remaining = src.len() - offset;
        // Emit literal-only token: high nibble = literal length, low nibble = 0 (no match)
        let lit_len = remaining;
        if lit_len < 15 {
            out.push((lit_len as u8) << 4);
        } else {
            out.push(0xF0);
            let mut extra = lit_len - 15;
            while extra >= 255 {
                out.push(255);
                extra -= 255;
            }
            out.push(extra as u8);
        }
        out.extend_from_slice(&src[offset..offset + lit_len]);
        offset += lit_len;
    }
    out
}

/// Decode LZ4 block format (literals + match copies).
fn lz4_decode(src: &[u8], max_output: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;

    while pos < src.len() {
        let token = src[pos];
        pos += 1;

        // ── Literal length ──
        let mut lit_len = ((token >> 4) & 0xF) as usize;
        if lit_len == 15 {
            loop {
                if pos >= src.len() {
                    return None;
                }
                let extra = src[pos] as usize;
                pos += 1;
                lit_len += extra;
                if extra < 255 {
                    break;
                }
            }
        }

        // ── Copy literals ──
        if pos + lit_len > src.len() {
            return None;
        }
        if out.len() + lit_len > max_output {
            return None;
        }
        out.extend_from_slice(&src[pos..pos + lit_len]);
        pos += lit_len;

        // If we've consumed all input, we're done (last sequence has no match).
        if pos >= src.len() {
            break;
        }

        // ── Match offset (2 bytes LE) ──
        if pos + 2 > src.len() {
            return None;
        }
        let match_offset = src[pos] as usize | ((src[pos + 1] as usize) << 8);
        pos += 2;
        if match_offset == 0 {
            return None;
        }

        // ── Match length ──
        let mut match_len = (token & 0xF) as usize + 4; // min match = 4
        if (token & 0xF) == 15 {
            loop {
                if pos >= src.len() {
                    return None;
                }
                let extra = src[pos] as usize;
                pos += 1;
                match_len += extra;
                if extra < 255 {
                    break;
                }
            }
        }

        // ── Copy from already-decoded output ──
        if match_offset > out.len() {
            return None;
        }
        if out.len() + match_len > max_output {
            return None;
        }
        let start = out.len() - match_offset;
        for i in 0..match_len {
            let byte = out[start + i];
            out.push(byte);
        }
    }

    Some(out)
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
        COMPRESSION_ZLIB => zlib_encode_stored(src_slice),
        COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => lz4_encode_literals(src_slice),
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
        COMPRESSION_ZLIB => match zlib_decode_stored(src_slice) {
            Some(d) => d,
            None => return 0,
        },
        COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => match lz4_decode(src_slice, dst_size) {
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
        COMPRESSION_ZLIB | COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => {}
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
            COMPRESSION_ZLIB => zlib_encode_stored(&state.buffer),
            COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => lz4_encode_literals(&state.buffer),
            _ => return COMPRESSION_STATUS_ERROR,
        }
    } else {
        match state.algo {
            COMPRESSION_ZLIB => match zlib_decode_stored(&state.buffer) {
                Some(d) => d,
                None => return COMPRESSION_STATUS_ERROR,
            },
            COMPRESSION_LZ4 | COMPRESSION_LZ4_RAW => match lz4_decode(&state.buffer, dst_size) {
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
        let encoded = zlib_encode_stored(input);
        let decoded = zlib_decode_stored(&encoded).expect("decode failed");
        assert_eq!(&decoded, input);
    }

    #[test]
    fn test_zlib_encode_large() {
        // 100 KB of patterned data
        let input: Vec<u8> = (0..100_000).map(|i| (i % 251) as u8).collect();
        let encoded = zlib_encode_stored(&input);
        let decoded = zlib_decode_stored(&encoded).expect("decode failed");
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_lz4_encode_decode_roundtrip() {
        let input = b"hello world";
        let encoded = lz4_encode_literals(input);
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
        let encoded = zlib_encode_stored(input);
        assert!(encoded.len() > 2); // at least header + block header + trailer
        let decoded = zlib_decode_stored(&encoded).expect("empty decode failed");
        assert!(decoded.is_empty());

        // LZ4: empty input → empty output
        let lz4_enc = lz4_encode_literals(input);
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
        assert_eq!(ret, COMPRESSION_STATUS_ERROR);
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
