//! Checked unsigned LEB128 decoding for length-delimited metadata.

pub(crate) fn read_uleb128(data: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *data.get(*pos)?;
        *pos += 1;
        // Only one payload bit remains in the tenth byte of a u64.
        if shift == 63 && byte > 1 {
            return None;
        }
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
    }
    None
}
