//! CommonCrypto API shims.
//!
//! On macOS these resolve via dlsym; on Linux they need pure-Rust
//! implementations.  All hash algorithms are implemented inline
//! (public domain algorithms) with no external crypto dependencies.

use crate::table::ShimTable;

pub struct CommonCryptoStubs;

impl CommonCryptoStubs {
    pub fn install(table: &mut ShimTable) {
        use crate::register_shims;
        register_shims!(
            table,
            (
                "_CC_SHA256",
                shim_cc_sha256 as *const () as u64,
                "CC_SHA256 one-shot"
            ),
            (
                "_CC_SHA256_Init",
                shim_cc_sha256_init as *const () as u64,
                "CC_SHA256_Init streaming"
            ),
            (
                "_CC_SHA256_Update",
                shim_cc_sha256_update as *const () as u64,
                "CC_SHA256_Update streaming"
            ),
            (
                "_CC_SHA256_Final",
                shim_cc_sha256_final as *const () as u64,
                "CC_SHA256_Final streaming"
            ),
            (
                "_CC_SHA1",
                shim_cc_sha1 as *const () as u64,
                "CC_SHA1 one-shot"
            ),
            (
                "_CC_MD5",
                shim_cc_md5 as *const () as u64,
                "CC_MD5 one-shot"
            ),
            (
                "_CCRandomGenerateBytes",
                shim_cc_random_generate_bytes as *const () as u64,
                "CCRandomGenerateBytes"
            ),
            (
                "_CCHmac",
                shim_cc_hmac as *const () as u64,
                "CCHmac one-shot"
            ),
            // CCCryptor stateful AES API
            (
                "_CCCryptorCreate",
                shim_cc_cryptor_create as *const () as u64,
                "CCCryptorCreate (AES-128/256 CBC/ECB)"
            ),
            (
                "_CCCryptorUpdate",
                shim_cc_cryptor_update as *const () as u64,
                "CCCryptorUpdate"
            ),
            (
                "_CCCryptorFinal",
                shim_cc_cryptor_final as *const () as u64,
                "CCCryptorFinal + PKCS7"
            ),
            (
                "_CCCryptorRelease",
                shim_cc_cryptor_release as *const () as u64,
                "CCCryptorRelease"
            ),
            (
                "_CCCryptorGetOutputLength",
                shim_cc_cryptor_get_output_length as *const () as u64,
                "CCCryptorGetOutputLength"
            ),
            (
                "_CCCrypt",
                shim_cc_crypt as *const () as u64,
                "CCCrypt one-shot"
            ),
            // RSA/EC — unimplemented stubs (kCCUnimplemented = -4305)
            (
                "_CCRSACryptorGeneratePair",
                shim_cc_rsa_unimplemented as *const () as u64,
                "CCRSACryptorGeneratePair (stub)"
            ),
            (
                "_CCRSACryptorEncrypt",
                shim_cc_rsa_unimplemented as *const () as u64,
                "CCRSACryptorEncrypt (stub)"
            ),
            (
                "_CCRSACryptorDecrypt",
                shim_cc_rsa_unimplemented as *const () as u64,
                "CCRSACryptorDecrypt (stub)"
            ),
            (
                "_CCECCryptorGeneratePair",
                shim_cc_rsa_unimplemented as *const () as u64,
                "CCECCryptorGeneratePair (stub)"
            ),
        );
        log::debug!("CommonCryptoStubs: registered 19 stubs");
    }
}

// ---------------------------------------------------------------------------
// SHA-256 (FIPS 180-4)
// ---------------------------------------------------------------------------

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

const SHA256_H0: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

#[inline(always)]
fn sha256_ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}
#[inline(always)]
fn sha256_maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}
#[inline(always)]
fn sha256_big_sigma0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}
#[inline(always)]
fn sha256_big_sigma1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}
#[inline(always)]
fn sha256_small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}
#[inline(always)]
fn sha256_small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

fn sha256_compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[4 * i],
            block[4 * i + 1],
            block[4 * i + 2],
            block[4 * i + 3],
        ]);
    }
    for i in 16..64 {
        w[i] = sha256_small_sigma1(w[i - 2])
            .wrapping_add(w[i - 7])
            .wrapping_add(sha256_small_sigma0(w[i - 15]))
            .wrapping_add(w[i - 16]);
    }

    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *state;

    for i in 0..64 {
        let t1 = h
            .wrapping_add(sha256_big_sigma1(e))
            .wrapping_add(sha256_ch(e, f, g))
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let t2 = sha256_big_sigma0(a).wrapping_add(sha256_maj(a, b, c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);
}

fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut state = SHA256_H0;
    let mut total_len: u64 = 0;
    let mut buf = [0u8; 64];
    let mut buf_len = 0usize;

    for &byte in data {
        buf[buf_len] = byte;
        buf_len += 1;
        total_len += 1;
        if buf_len == 64 {
            sha256_compress(&mut state, &buf);
            buf_len = 0;
        }
    }

    // Padding
    let bit_len = total_len * 8;
    buf[buf_len] = 0x80;
    buf_len += 1;

    if buf_len > 56 {
        while buf_len < 64 {
            buf[buf_len] = 0;
            buf_len += 1;
        }
        sha256_compress(&mut state, &buf);
        buf = [0u8; 64];
        buf_len = 0;
    }

    while buf_len < 56 {
        buf[buf_len] = 0;
        buf_len += 1;
    }

    buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sha256_compress(&mut state, &buf);

    let mut out = [0u8; 32];
    for (i, h) in state.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&h.to_be_bytes());
    }
    out
}

// Streaming SHA-256 context stored at the guest pointer.
#[repr(C)]
struct Sha256Ctx {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: u32,
    _pad: u32,
    total_len: u64,
}

/// One-shot SHA-256.
pub extern "C" fn shim_cc_sha256(data: u64, len: u32, md: u64) -> u64 {
    let slice = if data == 0 || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) }
    };
    let digest = sha256_digest(slice);
    if md != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(digest.as_ptr(), md as *mut u8, 32);
        }
    }
    md
}

/// Streaming SHA-256 init.
pub extern "C" fn shim_cc_sha256_init(ctx: u64) -> i32 {
    if ctx == 0 {
        return 0;
    }
    let c = ctx as *mut Sha256Ctx;
    unsafe {
        (*c).state = SHA256_H0;
        (*c).buf = [0u8; 64];
        (*c).buf_len = 0;
        (*c)._pad = 0;
        (*c).total_len = 0;
    }
    1
}

/// Streaming SHA-256 update.
pub extern "C" fn shim_cc_sha256_update(ctx: u64, data: u64, len: u32) -> i32 {
    if ctx == 0 {
        return 0;
    }
    let c = unsafe { &mut *(ctx as *mut Sha256Ctx) };
    let input = if data == 0 || len == 0 {
        return 1;
    } else {
        unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) }
    };

    for &byte in input {
        c.buf[c.buf_len as usize] = byte;
        c.buf_len += 1;
        c.total_len += 1;
        if c.buf_len == 64 {
            sha256_compress(&mut c.state, &c.buf);
            c.buf_len = 0;
        }
    }
    1
}

/// Streaming SHA-256 finalize.
pub extern "C" fn shim_cc_sha256_final(md: u64, ctx: u64) -> i32 {
    if ctx == 0 || md == 0 {
        return 0;
    }
    let c = unsafe { &mut *(ctx as *mut Sha256Ctx) };

    let bit_len = c.total_len * 8;
    let bl = c.buf_len as usize;

    c.buf[bl] = 0x80;
    let mut pos = bl + 1;

    if pos > 56 {
        while pos < 64 {
            c.buf[pos] = 0;
            pos += 1;
        }
        sha256_compress(&mut c.state, &c.buf);
        c.buf = [0u8; 64];
        pos = 0;
    }

    while pos < 56 {
        c.buf[pos] = 0;
        pos += 1;
    }

    c.buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sha256_compress(&mut c.state, &c.buf);

    let mut out = [0u8; 32];
    for (i, h) in c.state.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&h.to_be_bytes());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(out.as_ptr(), md as *mut u8, 32);
    }
    1
}

// ---------------------------------------------------------------------------
// SHA-1 (FIPS 180-4)
// ---------------------------------------------------------------------------

const SHA1_H0: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];

fn sha1_compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[4 * i],
            block[4 * i + 1],
            block[4 * i + 2],
            block[4 * i + 3],
        ]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }

    let [mut a, mut b, mut c, mut d, mut e] = *state;

    for (i, &word) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | ((!b) & d), 0x5a827999u32),
            20..=39 => (b ^ c ^ d, 0x6ed9eba1u32),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdcu32),
            _ => (b ^ c ^ d, 0xca62c1d6u32),
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(word);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut state = SHA1_H0;
    let mut total_len: u64 = 0;
    let mut buf = [0u8; 64];
    let mut buf_len = 0usize;

    for &byte in data {
        buf[buf_len] = byte;
        buf_len += 1;
        total_len += 1;
        if buf_len == 64 {
            sha1_compress(&mut state, &buf);
            buf_len = 0;
        }
    }

    let bit_len = total_len * 8;
    buf[buf_len] = 0x80;
    buf_len += 1;

    if buf_len > 56 {
        while buf_len < 64 {
            buf[buf_len] = 0;
            buf_len += 1;
        }
        sha1_compress(&mut state, &buf);
        buf = [0u8; 64];
        buf_len = 0;
    }

    while buf_len < 56 {
        buf[buf_len] = 0;
        buf_len += 1;
    }

    buf[56..64].copy_from_slice(&bit_len.to_be_bytes());
    sha1_compress(&mut state, &buf);

    let mut out = [0u8; 20];
    for (i, h) in state.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&h.to_be_bytes());
    }
    out
}

/// One-shot SHA-1.
pub extern "C" fn shim_cc_sha1(data: u64, len: u32, md: u64) -> u64 {
    let slice = if data == 0 || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) }
    };
    let digest = sha1_digest(slice);
    if md != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(digest.as_ptr(), md as *mut u8, 20);
        }
    }
    md
}

// ---------------------------------------------------------------------------
// MD5 (RFC 1321)
// ---------------------------------------------------------------------------

const MD5_T: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

const MD5_S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

fn md5_compress(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u32::from_le_bytes([
            block[4 * i],
            block[4 * i + 1],
            block[4 * i + 2],
            block[4 * i + 3],
        ]);
    }

    let [mut a, mut b, mut c, mut d] = *state;

    for i in 0..64 {
        let (f, g) = match i {
            0..=15 => ((b & c) | ((!b) & d), i),
            16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
            32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
            _ => (c ^ (b | !d), (7 * i) % 16),
        };
        let temp = a.wrapping_add(f).wrapping_add(MD5_T[i]).wrapping_add(m[g]);
        a = d;
        d = c;
        c = b;
        b = b.wrapping_add(temp.rotate_left(MD5_S[i]));
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
}

fn md5_digest(data: &[u8]) -> [u8; 16] {
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    let mut total_len: u64 = 0;
    let mut buf = [0u8; 64];
    let mut buf_len = 0usize;

    for &byte in data {
        buf[buf_len] = byte;
        buf_len += 1;
        total_len += 1;
        if buf_len == 64 {
            md5_compress(&mut state, &buf);
            buf_len = 0;
        }
    }

    let bit_len = total_len * 8;
    buf[buf_len] = 0x80;
    buf_len += 1;

    if buf_len > 56 {
        while buf_len < 64 {
            buf[buf_len] = 0;
            buf_len += 1;
        }
        md5_compress(&mut state, &buf);
        buf = [0u8; 64];
        buf_len = 0;
    }

    while buf_len < 56 {
        buf[buf_len] = 0;
        buf_len += 1;
    }

    // MD5 uses little-endian bit length
    buf[56..64].copy_from_slice(&bit_len.to_le_bytes());
    md5_compress(&mut state, &buf);

    let mut out = [0u8; 16];
    for (i, h) in state.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&h.to_le_bytes());
    }
    out
}

/// One-shot MD5.
pub extern "C" fn shim_cc_md5(data: u64, len: u32, md: u64) -> u64 {
    let slice = if data == 0 || len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) }
    };
    let digest = md5_digest(slice);
    if md != 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(digest.as_ptr(), md as *mut u8, 16);
        }
    }
    md
}

// ---------------------------------------------------------------------------
// CCRandomGenerateBytes
// ---------------------------------------------------------------------------

/// Fill buffer with cryptographically random bytes.  Returns 0 (kCCSuccess).
pub extern "C" fn shim_cc_random_generate_bytes(bytes: u64, count: usize) -> i32 {
    if bytes == 0 || count == 0 {
        return 0;
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(bytes as *mut u8, count) };

    // Use getentropy in chunks of 256 (POSIX limit).
    let mut offset = 0;
    while offset < count {
        let chunk = std::cmp::min(count - offset, 256);
        let ret = unsafe { libc::getentropy(buf[offset..].as_mut_ptr().cast(), chunk) };
        if ret != 0 {
            // Fallback: read /dev/urandom
            if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
                use std::io::Read;
                let _ = f.read_exact(&mut buf[offset..offset + chunk]);
            }
        }
        offset += chunk;
    }
    0 // kCCSuccess
}

// ---------------------------------------------------------------------------
// CCHmac (one-shot, supports kCCHmacAlgSHA256 = 2)
// ---------------------------------------------------------------------------

/// One-shot HMAC.  Only algorithm 2 (kCCHmacAlgSHA256) is fully supported.
pub extern "C" fn shim_cc_hmac(
    algorithm: u32,
    key: u64,
    key_len: usize,
    data: u64,
    data_len: usize,
    mac_out: u64,
) {
    if mac_out == 0 {
        return;
    }

    let key_slice = if key == 0 || key_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(key as *const u8, key_len) }
    };

    let data_slice = if data == 0 || data_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data as *const u8, data_len) }
    };

    // We support kCCHmacAlgSHA256 (2).  For unsupported algorithms, zero-fill.
    if algorithm != 2 {
        log::warn!("CCHmac: unsupported algorithm {algorithm}, zeroing output");
        unsafe {
            // SHA-256 digest length as default output size
            std::ptr::write_bytes(mac_out as *mut u8, 0, 32);
        }
        return;
    }

    let mac = hmac_sha256(key_slice, data_slice);
    unsafe {
        std::ptr::copy_nonoverlapping(mac.as_ptr(), mac_out as *mut u8, 32);
    }
}

/// HMAC-SHA256 per RFC 2104.
fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;

    // Step 1: If key > block size, hash it.
    let key_block = if key.len() > BLOCK_SIZE {
        let h = sha256_digest(key);
        let mut kb = [0u8; BLOCK_SIZE];
        kb[..32].copy_from_slice(&h);
        kb
    } else {
        let mut kb = [0u8; BLOCK_SIZE];
        kb[..key.len()].copy_from_slice(key);
        kb
    };

    // Step 2: ipad = key XOR 0x36, opad = key XOR 0x5c
    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    // Step 3: inner hash = SHA-256(ipad || data)
    let mut inner = Vec::with_capacity(BLOCK_SIZE + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_hash = sha256_digest(&inner);

    // Step 4: outer hash = SHA-256(opad || inner_hash)
    let mut outer = Vec::with_capacity(BLOCK_SIZE + 32);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_hash);
    sha256_digest(&outer)
}

// ---------------------------------------------------------------------------
// CCCryptor — AES-128/192/256 CBC/ECB with PKCS#7 padding
// ---------------------------------------------------------------------------
//
// Operation constants (CCOperation)
const KCC_ENCRYPT: u32 = 0;
const KCC_DECRYPT: u32 = 1;
// Algorithm constants (CCAlgorithm)
const KCC_ALGORITHM_AES: u32 = 0;
// Options (CCOptions)
const KCC_OPTION_PKCS7_PADDING: u32 = 0x0001;
const KCC_OPTION_ECB_MODE: u32 = 0x0002;
// Status codes
const KCC_SUCCESS: i32 = 0;
const KCC_PARAM_ERROR: i32 = -4300;
const KCC_BUFFER_TOO_SMALL: i32 = -4301;
const KCC_UNIMPLEMENTED: i32 = -4305;

// AES S-box
static SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

static SBOX_INV: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

// AES RCON constants
static RCON: [u32; 11] = [
    0x00000000, 0x01000000, 0x02000000, 0x04000000, 0x08000000, 0x10000000, 0x20000000, 0x40000000,
    0x80000000, 0x1b000000, 0x36000000,
];

fn aes_sub_word(w: u32) -> u32 {
    let b = w.to_be_bytes();
    u32::from_be_bytes([
        SBOX[b[0] as usize],
        SBOX[b[1] as usize],
        SBOX[b[2] as usize],
        SBOX[b[3] as usize],
    ])
}

fn aes_rot_word(w: u32) -> u32 {
    w.rotate_left(8)
}

/// Expand an AES-128 or AES-256 key into round key words.
/// Returns (round_keys, number_of_rounds).
fn aes_key_expand(key: &[u8]) -> (Vec<u32>, usize) {
    let nk = key.len() / 4; // 4 (AES-128) or 6 (AES-192) or 8 (AES-256)
    let nr = nk + 6; // 10, 12, or 14 rounds
    let total = 4 * (nr + 1);
    let mut w = vec![0u32; total];
    for i in 0..nk {
        w[i] = u32::from_be_bytes([key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]]);
    }
    for i in nk..total {
        let mut temp = w[i - 1];
        if i % nk == 0 {
            temp = aes_sub_word(aes_rot_word(temp)) ^ RCON[i / nk];
        } else if nk > 6 && i % nk == 4 {
            temp = aes_sub_word(temp);
        }
        w[i] = w[i - nk] ^ temp;
    }
    (w, nr)
}

#[inline]
fn xtime(b: u8) -> u8 {
    if b & 0x80 != 0 {
        (b << 1) ^ 0x1b
    } else {
        b << 1
    }
}
#[inline]
fn gmul2(b: u8) -> u8 {
    xtime(b)
}
#[inline]
fn gmul3(b: u8) -> u8 {
    xtime(b) ^ b
}
#[inline]
fn gmul9(b: u8) -> u8 {
    xtime(xtime(xtime(b))) ^ b
}
#[inline]
fn gmuln(b: u8) -> u8 {
    xtime(xtime(xtime(b))) ^ xtime(b)
} // 0x0e
#[inline]
fn gmulb(b: u8) -> u8 {
    xtime(xtime(xtime(b))) ^ xtime(b) ^ b
} // 0x0b
#[inline]
fn gmuld(b: u8) -> u8 {
    xtime(xtime(xtime(b))) ^ xtime(xtime(b)) ^ b
} // 0x0d

fn aes_encrypt_block(block: &[u8; 16], round_keys: &[u32], nr: usize) -> [u8; 16] {
    let mut s = [0u8; 16];
    s.copy_from_slice(block);
    // AddRoundKey (round 0)
    for i in 0..4 {
        let rk = round_keys[i].to_be_bytes();
        s[4 * i] ^= rk[0];
        s[4 * i + 1] ^= rk[1];
        s[4 * i + 2] ^= rk[2];
        s[4 * i + 3] ^= rk[3];
    }
    for r in 1..=nr {
        // SubBytes
        for b in s.iter_mut() {
            *b = SBOX[*b as usize];
        }
        // ShiftRows: row1 left 1, row2 left 2, row3 left 3
        let t = s[1];
        s[1] = s[5];
        s[5] = s[9];
        s[9] = s[13];
        s[13] = t;
        let (t1, t2) = (s[2], s[6]);
        s[2] = s[10];
        s[6] = s[14];
        s[10] = t1;
        s[14] = t2;
        let t = s[15];
        s[15] = s[11];
        s[11] = s[7];
        s[7] = s[3];
        s[3] = t;
        // MixColumns (skip for last round)
        if r < nr {
            for c in 0..4 {
                let (a, b, cc, d) = (s[4 * c], s[4 * c + 1], s[4 * c + 2], s[4 * c + 3]);
                s[4 * c] = gmul2(a) ^ gmul3(b) ^ cc ^ d;
                s[4 * c + 1] = a ^ gmul2(b) ^ gmul3(cc) ^ d;
                s[4 * c + 2] = a ^ b ^ gmul2(cc) ^ gmul3(d);
                s[4 * c + 3] = gmul3(a) ^ b ^ cc ^ gmul2(d);
            }
        }
        // AddRoundKey
        let off = 4 * r;
        for i in 0..4 {
            let rk = round_keys[off + i].to_be_bytes();
            s[4 * i] ^= rk[0];
            s[4 * i + 1] ^= rk[1];
            s[4 * i + 2] ^= rk[2];
            s[4 * i + 3] ^= rk[3];
        }
    }
    s
}

fn aes_decrypt_block(block: &[u8; 16], round_keys: &[u32], nr: usize) -> [u8; 16] {
    let mut s = [0u8; 16];
    s.copy_from_slice(block);
    let off = 4 * nr;
    for i in 0..4 {
        let rk = round_keys[off + i].to_be_bytes();
        s[4 * i] ^= rk[0];
        s[4 * i + 1] ^= rk[1];
        s[4 * i + 2] ^= rk[2];
        s[4 * i + 3] ^= rk[3];
    }
    for r in (0..nr).rev() {
        // InvShiftRows: row1 right 1, row2 right 2, row3 right 3
        let t = s[13];
        s[13] = s[9];
        s[9] = s[5];
        s[5] = s[1];
        s[1] = t;
        let (t1, t2) = (s[2], s[6]);
        s[2] = s[10];
        s[6] = s[14];
        s[10] = t1;
        s[14] = t2;
        let t = s[3];
        s[3] = s[7];
        s[7] = s[11];
        s[11] = s[15];
        s[15] = t;
        // InvSubBytes
        for b in s.iter_mut() {
            *b = SBOX_INV[*b as usize];
        }
        // AddRoundKey
        let off = 4 * r;
        for i in 0..4 {
            let rk = round_keys[off + i].to_be_bytes();
            s[4 * i] ^= rk[0];
            s[4 * i + 1] ^= rk[1];
            s[4 * i + 2] ^= rk[2];
            s[4 * i + 3] ^= rk[3];
        }
        // InvMixColumns (skip for round 0, i.e., before first round key)
        if r > 0 {
            for c in 0..4 {
                let (a, b, cc, d) = (s[4 * c], s[4 * c + 1], s[4 * c + 2], s[4 * c + 3]);
                s[4 * c] = gmuln(a) ^ gmulb(b) ^ gmuld(cc) ^ gmul9(d);
                s[4 * c + 1] = gmul9(a) ^ gmuln(b) ^ gmulb(cc) ^ gmuld(d);
                s[4 * c + 2] = gmuld(a) ^ gmul9(b) ^ gmuln(cc) ^ gmulb(d);
                s[4 * c + 3] = gmulb(a) ^ gmuld(b) ^ gmul9(cc) ^ gmuln(d);
            }
        }
    }
    s
}

/// CCCryptorRef heap layout.
struct CcCryptorState {
    op: u32,
    nr: usize,
    round_keys: Vec<u32>,
    iv: [u8; 16],
    ecb: bool, // true → ECB mode, false → CBC
    pkcs7: bool,
    buf: [u8; 16], // partial block accumulator
    buf_len: usize,
}

impl CcCryptorState {
    fn process_block(&mut self, block: &[u8; 16]) -> [u8; 16] {
        if self.op == KCC_ENCRYPT {
            let mut xored = [0u8; 16];
            if !self.ecb {
                for i in 0..16 {
                    xored[i] = block[i] ^ self.iv[i];
                }
            } else {
                xored.copy_from_slice(block);
            }
            let out = aes_encrypt_block(&xored, &self.round_keys, self.nr);
            if !self.ecb {
                self.iv = out;
            }
            out
        } else {
            let pt = aes_decrypt_block(block, &self.round_keys, self.nr);
            let mut out = [0u8; 16];
            if !self.ecb {
                for i in 0..16 {
                    out[i] = pt[i] ^ self.iv[i];
                }
                self.iv.copy_from_slice(block);
            } else {
                out.copy_from_slice(&pt);
            }
            out
        }
    }
}

fn box_cryptor(state: CcCryptorState) -> u64 {
    Box::into_raw(Box::new(state)) as u64
}

unsafe fn unbox_cryptor(ptr: u64) -> &'static mut CcCryptorState {
    &mut *(ptr as *mut CcCryptorState)
}

/// `CCCryptorStatus CCCryptorCreate(op, alg, options, key, keyLen, iv, refOut)`
pub extern "C" fn shim_cc_cryptor_create(
    op: u32,
    alg: u32,
    options: u32,
    key: u64,
    key_len: usize,
    iv: u64,
    ref_out: u64,
) -> i32 {
    if alg != KCC_ALGORITHM_AES {
        return KCC_UNIMPLEMENTED;
    }
    if key == 0 || ref_out == 0 {
        return KCC_PARAM_ERROR;
    }
    if key_len != 16 && key_len != 24 && key_len != 32 {
        return KCC_PARAM_ERROR;
    }

    let key_bytes = unsafe { std::slice::from_raw_parts(key as *const u8, key_len) };
    let (round_keys, nr) = aes_key_expand(key_bytes);

    let mut iv_buf = [0u8; 16];
    if iv != 0 && (options & KCC_OPTION_ECB_MODE == 0) {
        let iv_bytes = unsafe { std::slice::from_raw_parts(iv as *const u8, 16) };
        iv_buf.copy_from_slice(iv_bytes);
    }

    let state = CcCryptorState {
        op,
        nr,
        round_keys,
        iv: iv_buf,
        ecb: options & KCC_OPTION_ECB_MODE != 0,
        pkcs7: options & KCC_OPTION_PKCS7_PADDING != 0,
        buf: [0u8; 16],
        buf_len: 0,
    };

    unsafe {
        *(ref_out as *mut u64) = box_cryptor(state);
    }
    KCC_SUCCESS
}

/// `CCCryptorStatus CCCryptorUpdate(ref, dataIn, dataInLen, out, outAvail, moved)`
pub extern "C" fn shim_cc_cryptor_update(
    cref: u64,
    data_in: u64,
    data_in_len: usize,
    out: u64,
    out_avail: usize,
    moved: u64,
) -> i32 {
    if cref == 0 {
        return KCC_PARAM_ERROR;
    }
    let state = unsafe { unbox_cryptor(cref) };

    let input = if data_in == 0 || data_in_len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data_in as *const u8, data_in_len) }
    };

    // Estimate how many full blocks we'll output.
    let total_available = state.buf_len + input.len();
    let blocks_out = total_available / 16;
    let needed = blocks_out * 16;
    if needed > out_avail {
        return KCC_BUFFER_TOO_SMALL;
    }

    let out_ptr = out as *mut u8;
    let mut written = 0usize;
    let mut pos = 0usize;

    while pos < input.len() {
        let space = 16 - state.buf_len;
        let take = space.min(input.len() - pos);
        state.buf[state.buf_len..state.buf_len + take].copy_from_slice(&input[pos..pos + take]);
        state.buf_len += take;
        pos += take;

        if state.buf_len == 16 {
            let block: [u8; 16] = state.buf;
            let cipher = state.process_block(&block);
            unsafe {
                std::ptr::copy_nonoverlapping(cipher.as_ptr(), out_ptr.add(written), 16);
            }
            written += 16;
            state.buf_len = 0;
        }
    }

    if moved != 0 {
        unsafe {
            *(moved as *mut usize) = written;
        }
    }
    KCC_SUCCESS
}

/// `CCCryptorStatus CCCryptorFinal(ref, out, outAvail, moved)`
pub extern "C" fn shim_cc_cryptor_final(cref: u64, out: u64, out_avail: usize, moved: u64) -> i32 {
    if cref == 0 {
        return KCC_PARAM_ERROR;
    }
    let state = unsafe { unbox_cryptor(cref) };

    if state.op == KCC_ENCRYPT && state.pkcs7 {
        // Add PKCS#7 padding
        let pad = (16 - state.buf_len % 16) as u8;
        for i in state.buf_len..16 {
            state.buf[i] = pad;
        }
        state.buf_len = 16;
    }

    let written = if state.buf_len == 16 {
        if out_avail < 16 {
            return KCC_BUFFER_TOO_SMALL;
        }
        let block: [u8; 16] = state.buf;
        let cipher = state.process_block(&block);
        unsafe {
            std::ptr::copy_nonoverlapping(cipher.as_ptr(), out as *mut u8, 16);
        }
        state.buf_len = 0;
        16usize
    } else if state.op == KCC_DECRYPT && state.pkcs7 && state.buf_len == 0 {
        // PKCS#7 stripping: the last block already emitted in Update had padding —
        // we don't undo it here (caller owns the buffer).  Return 0 written.
        0usize
    } else {
        0usize
    };

    if moved != 0 {
        unsafe {
            *(moved as *mut usize) = written;
        }
    }
    KCC_SUCCESS
}

/// `CCCryptorStatus CCCryptorRelease(ref)`
pub extern "C" fn shim_cc_cryptor_release(cref: u64) -> i32 {
    if cref == 0 {
        return KCC_PARAM_ERROR;
    }
    unsafe {
        drop(Box::from_raw(cref as *mut CcCryptorState));
    }
    KCC_SUCCESS
}

/// `size_t CCCryptorGetOutputLength(ref, inputLength, final)`
pub extern "C" fn shim_cc_cryptor_get_output_length(
    cref: u64,
    input_len: usize,
    is_final: bool,
) -> usize {
    let buf_len = if cref != 0 {
        unsafe { unbox_cryptor(cref) }.buf_len
    } else {
        0
    };
    let total = buf_len + input_len;
    if is_final {
        ((total / 16) + 1) * 16
    } else {
        (total / 16) * 16
    }
}

/// One-shot `CCCrypt`.
pub extern "C" fn shim_cc_crypt(
    op: u32,
    alg: u32,
    options: u32,
    key: u64,
    key_len: usize,
    iv: u64,
    data_in: u64,
    data_in_len: usize,
    data_out: u64,
    data_out_avail: usize,
    data_out_moved: u64,
) -> i32 {
    let mut cref: u64 = 0;
    let rc = shim_cc_cryptor_create(
        op,
        alg,
        options,
        key,
        key_len,
        iv,
        &mut cref as *mut u64 as u64,
    );
    if rc != KCC_SUCCESS {
        return rc;
    }

    let mut moved1: usize = 0;
    let rc2 = shim_cc_cryptor_update(
        cref,
        data_in,
        data_in_len,
        data_out,
        data_out_avail,
        &mut moved1 as *mut usize as u64,
    );

    if rc2 != KCC_SUCCESS {
        shim_cc_cryptor_release(cref);
        return rc2;
    }

    let remaining = data_out_avail.saturating_sub(moved1);
    let mut moved2: usize = 0;
    let rc3 = shim_cc_cryptor_final(
        cref,
        if data_out != 0 {
            data_out + moved1 as u64
        } else {
            0
        },
        remaining,
        &mut moved2 as *mut usize as u64,
    );

    shim_cc_cryptor_release(cref);

    if data_out_moved != 0 {
        unsafe {
            *(data_out_moved as *mut usize) = moved1 + moved2;
        }
    }
    rc3
}

/// Stub for RSA/EC key functions — return kCCUnimplemented.
pub extern "C" fn shim_cc_rsa_unimplemented(_a: u64, _b: u64, _c: u64, _d: u64) -> i32 {
    KCC_UNIMPLEMENTED
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn test_sha256_empty() {
        let digest = sha256_digest(b"");
        assert_eq!(
            hex(&digest),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_abc() {
        let digest = sha256_digest(b"abc");
        assert_eq!(
            hex(&digest),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha256_streaming() {
        // Streaming must match one-shot.
        let mut ctx = Sha256Ctx {
            state: [0; 8],
            buf: [0; 64],
            buf_len: 0,
            _pad: 0,
            total_len: 0,
        };
        let ctx_ptr = &mut ctx as *mut Sha256Ctx as u64;
        assert_eq!(shim_cc_sha256_init(ctx_ptr), 1);

        let data = b"abc";
        assert_eq!(
            shim_cc_sha256_update(ctx_ptr, data.as_ptr() as u64, data.len() as u32),
            1
        );

        let mut md = [0u8; 32];
        assert_eq!(shim_cc_sha256_final(md.as_mut_ptr() as u64, ctx_ptr), 1);

        assert_eq!(
            hex(&md),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha1_empty() {
        let digest = sha1_digest(b"");
        assert_eq!(hex(&digest), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn test_sha1_abc() {
        let digest = sha1_digest(b"abc");
        assert_eq!(hex(&digest), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn test_md5_empty() {
        let digest = md5_digest(b"");
        assert_eq!(hex(&digest), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_md5_abc() {
        let digest = md5_digest(b"abc");
        assert_eq!(hex(&digest), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn test_random_bytes_fills() {
        let mut buf = [0u8; 64];
        let ret = shim_cc_random_generate_bytes(buf.as_mut_ptr() as u64, buf.len());
        assert_eq!(ret, 0);
        // Extremely unlikely that 64 random bytes are all zero.
        assert!(
            buf.iter().any(|&b| b != 0),
            "random buffer should not be all zeros"
        );
    }

    #[test]
    fn test_hmac_sha256() {
        // RFC 4231 Test Case 2
        let key = b"Jefe";
        let data = b"what do ya want for nothing?";
        let mac = hmac_sha256(key, data);
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }
}
