//! Chromium **Local Storage** (DOM Storage on LevelDB) reader.
//!
//! Since Chromium 61 the DOM `localStorage` API is backed by a per-profile
//! LevelDB store (`Local Storage/leveldb/`). This reader walks every raw record
//! (via [`leveldb_core`], so tombstones and superseded versions surface too) and
//! classifies each into a [`LocalStorageRecord`]:
//!
//! * a per-origin `META:` record — a protobuf carrying the store's size and a
//!   WebKit-microsecond last-modified timestamp;
//! * a data record — `"_" + origin + NUL + script-key → value`, where the
//!   script-key and value are type-prefixed strings (`0x00` UTF-16LE / `0x01`
//!   Latin-1);
//! * anything else (e.g. the absence-of-`VERSION` bookkeeping keys), surfaced
//!   raw.
//!
//! The key schema and encoding-marker constants come from the fleet KNOWLEDGE
//! leaf [`forensicnomicon_core::chromium_local_storage`]. Decoding never panics:
//! a lossy transcode becomes U+FFFD and sets [`StorageValue::lossy`], and the raw
//! bytes are always retained.
//!
//! Reference: CCL Solutions, *Chromium Session Storage and Local Storage*.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod value;

pub use value::{Encoding, StorageValue};

use forensicnomicon_core::chromium_local_storage::{DATA_KEY_PREFIX, KEY_SEPARATOR, META_PREFIX};
use leveldb_core::Record;
use std::path::Path;
use value::decode_type_prefixed;

/// One decoded Local Storage record. Deletion tombstones and superseded versions
/// surface too (each carries its `seq` and `deleted` flag).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalStorageRecord {
    /// Origin-level metadata from a `META:` key.
    Meta {
        /// The storage key (origin) this metadata describes.
        origin: String,
        /// Last-modified time, WebKit microseconds (µs since 1601-01-01 UTC).
        timestamp_webkit_micros: u64,
        /// Declared size in bytes, if the protobuf carried the size field.
        size: Option<u64>,
        /// LevelDB sequence number.
        seq: u64,
        /// `true` if this is a deletion tombstone.
        deleted: bool,
    },
    /// An actual stored key/value pair.
    Data {
        /// The storage key (origin), decoded as Latin-1.
        origin: String,
        /// The script-visible key (a type-prefixed string).
        script_key: StorageValue,
        /// The stored value (a type-prefixed string; empty for a tombstone).
        value: StorageValue,
        /// LevelDB sequence number.
        seq: u64,
        /// `true` if this is a deletion tombstone.
        deleted: bool,
    },
    /// A key that matched neither the Meta nor Data shape. The raw key bytes are
    /// surfaced verbatim rather than dropped.
    Other {
        /// The raw user key.
        key: Vec<u8>,
        /// LevelDB sequence number.
        seq: u64,
        /// `true` if this is a deletion tombstone.
        deleted: bool,
    },
}

/// Decode a byte string as ISO-8859-1 (Latin-1): each byte is one code point.
/// Origins are stored as Latin-1 in the key.
fn latin1(raw: &[u8]) -> String {
    raw.iter().map(|&b| b as char).collect()
}

/// Read one LEB128 protobuf varint at `off`; returns `(value, next_offset)` or
/// `None` if it is truncated or overflows `u64`. Panic-free.
fn read_varint(data: &[u8], off: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    let mut i = off;
    loop {
        let byte = *data.get(i)?;
        i += 1;
        if shift >= 64 {
            return None; // more than 10 groups → overflow
        }
        result |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some((result, i));
        }
        shift += 7;
    }
}

/// Parse the `StorageMetadata` protobuf: field 1 (tag `0x08`) is the WebKit-µs
/// last-modified timestamp varint; optional field 2 (tag `0x10`) is the declared
/// size varint. Returns `None` unless the payload opens with the field-1
/// timestamp, matching how Chrome writes it.
fn parse_meta_protobuf(data: &[u8]) -> Option<(u64, Option<u64>)> {
    // Field 1, wire type 0 (varint): tag byte 0x08.
    let (tag, off) = read_varint(data, 0)?;
    if tag != 0x08 {
        return None;
    }
    let (timestamp, off) = read_varint(data, off)?;
    // Optional field 2, wire type 0: tag byte 0x10.
    let size = match read_varint(data, off) {
        Some((0x10, off2)) => read_varint(data, off2).map(|(v, _)| v),
        _ => None,
    };
    Some((timestamp, size))
}

/// Decode Local Storage records from raw LevelDB [`Record`]s. Every record is
/// classified into [`LocalStorageRecord::Meta`], [`LocalStorageRecord::Data`], or
/// [`LocalStorageRecord::Other`]; deletion tombstones and superseded versions are
/// kept.
#[must_use]
pub fn decode_records(records: &[Record]) -> Vec<LocalStorageRecord> {
    let mut out = Vec::with_capacity(records.len());
    for r in records {
        out.push(classify(r));
    }
    out
}

fn classify(r: &Record) -> LocalStorageRecord {
    if let Some(origin_raw) = r.key.strip_prefix(META_PREFIX) {
        let origin = latin1(origin_raw);
        if r.deleted {
            // A cleared metadata entry — no protobuf to parse, but the tombstone
            // itself is forensically meaningful.
            return LocalStorageRecord::Meta {
                origin,
                timestamp_webkit_micros: 0,
                size: None,
                seq: r.seq,
                deleted: true,
            };
        }
        if let Some((timestamp_webkit_micros, size)) = parse_meta_protobuf(&r.value) {
            return LocalStorageRecord::Meta {
                origin,
                timestamp_webkit_micros,
                size,
                seq: r.seq,
                deleted: false,
            };
        }
        return other(r);
    }

    if r.key.first() == Some(&DATA_KEY_PREFIX) {
        let body = &r.key[1..];
        if let Some(nul) = body.iter().position(|&b| b == KEY_SEPARATOR) {
            let origin = latin1(&body[..nul]);
            let script_key = decode_type_prefixed(&body[nul + 1..]);
            let value = if r.deleted {
                StorageValue::empty()
            } else {
                decode_type_prefixed(&r.value)
            };
            return LocalStorageRecord::Data {
                origin,
                script_key,
                value,
                seq: r.seq,
                deleted: r.deleted,
            };
        }
    }

    other(r)
}

fn other(r: &Record) -> LocalStorageRecord {
    LocalStorageRecord::Other {
        key: r.key.clone(),
        seq: r.seq,
        deleted: r.deleted,
    }
}

/// Read a `Local Storage/leveldb` directory and decode its records.
pub fn read_dir(dir: &Path) -> Result<Vec<LocalStorageRecord>, leveldb_core::Error> {
    let records = leveldb_core::read_dir(dir)?;
    Ok(decode_records(&records))
}
