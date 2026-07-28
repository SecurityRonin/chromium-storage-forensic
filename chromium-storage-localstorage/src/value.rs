//! Decoded Chromium Local Storage value types.
//!
//! A **type-prefixed string** is a one-byte encoding marker followed by the
//! encoded body: marker `0x00` = UTF-16LE, `0x01` = Latin-1 (ISO-8859-1); any
//! other marker is malformed and surfaced raw with `lossy` set. Decoding never
//! fails — a lone surrogate or dangling byte becomes U+FFFD and sets `lossy`, and
//! the raw bytes are always retained.
//!
//! The marker→encoding mapping is the fleet KNOWLEDGE rule
//! [`forensicnomicon_core::chromium_local_storage::value_encoding`].

use forensicnomicon_core::chromium_local_storage::{value_encoding, ValueEncoding};

/// How a [`StorageValue`] was decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-16 little-endian (marker `0x00`).
    Utf16Le,
    /// 8-bit ISO-8859-1 / Latin-1 (marker `0x01`).
    Latin1,
    /// The value had no bytes at all.
    Empty,
    /// An unrecognised marker byte (carried verbatim).
    Unknown(u8),
}

/// A decoded storage value. The `raw` bytes are always retained; `lossy` is set
/// whenever decoding had to substitute a replacement character or the marker was
/// unrecognised, so a caller cannot mistake a lossy decode for a clean one
/// (secure-by-design).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageValue {
    /// Best-effort decoded text (U+FFFD substituted for undecodable units).
    pub text: String,
    /// The raw value bytes, verbatim.
    pub raw: Vec<u8>,
    /// Which decoder produced `text`.
    pub encoding: Encoding,
    /// `true` if decoding was lossy or the marker was unrecognised.
    pub lossy: bool,
}

impl StorageValue {
    /// The empty value (e.g. a deletion tombstone carries no value bytes).
    pub(crate) fn empty() -> Self {
        StorageValue {
            text: String::new(),
            raw: Vec::new(),
            encoding: Encoding::Empty,
            lossy: false,
        }
    }
}

/// Decode a UTF-16LE body. A trailing odd byte or a lone surrogate marks the
/// result lossy but never panics.
fn decode_utf16le(body: &[u8], raw: &[u8]) -> StorageValue {
    let mut lossy = false;
    let mut chunks = body.chunks_exact(2);
    let units: Vec<u16> = chunks
        .by_ref()
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    if !chunks.remainder().is_empty() {
        lossy = true; // dangling half code unit
    }
    let mut text = String::new();
    for unit in char::decode_utf16(units) {
        match unit {
            Ok(ch) => text.push(ch),
            Err(_) => {
                text.push('\u{FFFD}');
                lossy = true;
            }
        }
    }
    StorageValue {
        text,
        raw: raw.to_vec(),
        encoding: Encoding::Utf16Le,
        lossy,
    }
}

/// Decode a Latin-1 body. Every byte maps to U+00xx, so this is total and never
/// lossy.
fn decode_latin1(body: &[u8], raw: &[u8]) -> StorageValue {
    let text: String = body.iter().map(|&b| b as char).collect();
    StorageValue {
        text,
        raw: raw.to_vec(),
        encoding: Encoding::Latin1,
        lossy: false,
    }
}

/// Decode a type-prefixed string. The leading marker selects the encoding (per
/// [`value_encoding`]); an unrecognised marker surfaces the raw bytes with
/// `lossy = true` rather than guessing.
#[must_use]
pub(crate) fn decode_type_prefixed(raw: &[u8]) -> StorageValue {
    let Some((&marker, body)) = raw.split_first() else {
        return StorageValue::empty();
    };
    match value_encoding(marker) {
        Some(ValueEncoding::Utf16Le) => decode_utf16le(body, raw),
        Some(ValueEncoding::Latin1) => decode_latin1(body, raw),
        // `None` is an unrecognised marker; the `Some(_)` arm is a defensive
        // catch for a future `ValueEncoding` variant (the enum is
        // `#[non_exhaustive]`). Both surface the raw bytes rather than guess.
        None | Some(_) => StorageValue {
            text: String::from_utf8_lossy(raw).into_owned(),
            raw: raw.to_vec(),
            encoding: Encoding::Unknown(marker),
            lossy: true,
        },
    }
}
