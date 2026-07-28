//! Decoded IndexedDB primary key (`IDBKey`).
//!
//! Encoding (Chromium `indexed_db_leveldb_coding.cc`): a one-byte type tag then a
//! type-specific body. Strings are an `EncodeVarInt` length (in 16-bit units)
//! followed by UTF-16BE; numbers/dates are a raw little-endian `f64`; binary is a
//! varint length + bytes; arrays are a varint count + that many nested keys.

use crate::util::{decode_utf16be, read_varint};

// IDBKey type tags (Chromium `indexed_db_leveldb_coding.cc`).
const KEY_NULL: u8 = 0;
const KEY_STRING: u8 = 1;
const KEY_DATE: u8 = 2;
const KEY_NUMBER: u8 = 3;
const KEY_ARRAY: u8 = 4;
const KEY_MIN: u8 = 5;
const KEY_BINARY: u8 = 6;

/// A decoded IndexedDB key. The encoding is a one-byte type tag followed by a
/// type-specific body (Chromium `indexed_db_leveldb_coding.cc`
/// `EncodeIDBKey`/`DecodeIDBKey`).
#[derive(Clone, Debug, PartialEq)]
pub enum IdbKey {
    /// `kIndexedDBKeyNumberTypeByte` — an IEEE-754 double.
    Number(f64),
    /// `kIndexedDBKeyDateTypeByte` — a date as milliseconds since the Unix epoch.
    Date(f64),
    /// `kIndexedDBKeyStringTypeByte` — a UTF-16BE string.
    String(String),
    /// `kIndexedDBKeyBinaryTypeByte` — raw bytes.
    Binary(Vec<u8>),
    /// `kIndexedDBKeyArrayTypeByte` — an array of keys.
    Array(Vec<IdbKey>),
    /// `kIndexedDBKeyNullTypeByte` — the null/none key.
    Null,
    /// `kIndexedDBKeyMinKeyTypeByte` — the synthetic minimum key.
    Min,
    /// An unrecognised type tag or truncated body — raw bytes surfaced verbatim.
    Invalid(Vec<u8>),
}

impl IdbKey {
    /// Decode the `IDBKey` occupying the whole `tail` (the key bytes after the
    /// `KeyPrefix`). A tag or body the decoder cannot follow becomes
    /// [`IdbKey::Invalid`] with the remaining bytes; never panics.
    pub(crate) fn decode(tail: &[u8]) -> IdbKey {
        match decode_one(tail, 0) {
            Some((key, _)) => key,
            None => IdbKey::Invalid(tail.to_vec()),
        }
    }
}

/// Decode one key at `pos`; returns the key and the offset just past it.
fn decode_one(data: &[u8], pos: usize) -> Option<(IdbKey, usize)> {
    let tag = *data.get(pos)?;
    let pos = pos + 1;
    match tag {
        KEY_NULL => Some((IdbKey::Null, pos)),
        KEY_MIN => Some((IdbKey::Min, pos)),
        KEY_STRING => {
            let (units, pos) = read_varint(data, pos)?;
            let byte_len = (units as usize).checked_mul(2)?;
            let end = pos.checked_add(byte_len)?;
            let body = data.get(pos..end)?;
            Some((IdbKey::String(decode_utf16be(body)), end))
        }
        KEY_BINARY => {
            let (len, pos) = read_varint(data, pos)?;
            let end = pos.checked_add(len as usize)?;
            let body = data.get(pos..end)?;
            Some((IdbKey::Binary(body.to_vec()), end))
        }
        KEY_NUMBER | KEY_DATE => {
            let end = pos.checked_add(8)?;
            let bytes = data.get(pos..end)?;
            let mut buf = [0u8; 8];
            buf.copy_from_slice(bytes);
            let d = f64::from_le_bytes(buf);
            let key = if tag == KEY_DATE {
                IdbKey::Date(d)
            } else {
                IdbKey::Number(d)
            };
            Some((key, end))
        }
        KEY_ARRAY => {
            let (count, mut pos) = read_varint(data, pos)?;
            let mut items = Vec::new();
            for _ in 0..count {
                let (item, next) = decode_one(data, pos)?;
                items.push(item);
                pos = next;
            }
            Some((IdbKey::Array(items), pos))
        }
        _ => Some((IdbKey::Invalid(data.get(pos - 1..)?.to_vec()), data.len())),
    }
}
