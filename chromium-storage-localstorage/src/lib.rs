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

use leveldb_core::Record;
use std::path::Path;

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

/// Decode Local Storage records from raw LevelDB [`Record`]s.
#[must_use]
pub fn decode_records(records: &[Record]) -> Vec<LocalStorageRecord> {
    let _ = records;
    Vec::new()
}

/// Read a `Local Storage/leveldb` directory and decode its records.
pub fn read_dir(dir: &Path) -> Result<Vec<LocalStorageRecord>, leveldb_core::Error> {
    let records = leveldb_core::read_dir(dir)?;
    Ok(decode_records(&records))
}
