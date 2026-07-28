//! Shared panic-free primitives: the IndexedDB `EncodeVarInt` varint and the
//! UTF-16BE string decoder used by keys and metadata values.

/// Read one IndexedDB `EncodeVarInt` LEB128 varint at `off`; returns
/// `(value, next_offset)` or `None` if truncated or overflowing `u64`.
pub(crate) fn read_varint(data: &[u8], off: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut i = off;
    loop {
        let byte = *data.get(i)?;
        i += 1;
        if shift >= 64 {
            return None;
        }
        result |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i));
        }
        shift += 7;
    }
}

/// Decode a UTF-16 **big-endian** byte string (IndexedDB stores strings BE). A
/// trailing odd byte or lone surrogate is rendered U+FFFD; never panics.
pub(crate) fn decode_utf16be(body: &[u8]) -> String {
    let units: Vec<u16> = body
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    char::decode_utf16(units)
        .map(|u| u.unwrap_or('\u{FFFD}'))
        .collect()
}
