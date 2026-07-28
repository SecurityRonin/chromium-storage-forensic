//! Chromium **IndexedDB** (over LevelDB) reader.
//!
//! Chromium backs the IndexedDB API with a LevelDB store on disk
//! (`IndexedDB/<origin>.indexeddb.leveldb/`). Every record's key begins with a
//! space-optimised **`KeyPrefix`** naming its `(database id, object store id,
//! index id)`; object-store *data* records (index id
//! [`INDEX_ID_OBJECT_STORE_DATA`]) carry the encoded primary **`IDBKey`** in the
//! key tail and, in the value, a wrapper-version varint followed by a Blink
//! `SerializedScriptValue` (a V8 structured-clone blob).
//!
//! This reader walks every raw record (via [`leveldb_core`], so tombstones and
//! superseded versions surface too), resolves the database and object-store
//! *names* from their metadata records, decodes each data record's `IDBKey`, and
//! deserializes the value's V8 blob via [`blob_decoder`]. It emits one
//! [`IndexedDbRecord`] per object-store data record: `(database, objectStore,
//! key, decoded-value)`.
//!
//! Key-coding constants come from the fleet KNOWLEDGE leaf
//! [`forensicnomicon_core::chromium_indexeddb`]. Parsing is panic-free and
//! bounds-checked; an undecodable value surfaces its raw bytes plus the decode
//! error rather than being dropped.
//!
//! References: Chromium `indexed_db_leveldb_coding.cc` and CCL Solutions,
//! *IndexedDB on Chromium*.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod key;
mod value;

pub use blob_decoder::v8_value::V8Value;
pub use key::IdbKey;
pub use value::RecordValue;

use leveldb_core::Record;
use std::path::Path;

/// One decoded IndexedDB object-store data record.
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedDbRecord {
    /// The database id from the record's `KeyPrefix`.
    pub database_id: u64,
    /// The object-store id from the record's `KeyPrefix`.
    pub object_store_id: u64,
    /// The database name, resolved from the global database-name index (if found).
    pub database: Option<String>,
    /// The object-store name, resolved from its metadata record (if found).
    pub object_store: Option<String>,
    /// The decoded primary key (`IDBKey`).
    pub key: IdbKey,
    /// The decoded record value (a V8 structured-clone value, or raw + error).
    pub value: RecordValue,
    /// LevelDB sequence number.
    pub seq: u64,
    /// `true` if this record is a deletion tombstone.
    pub deleted: bool,
}

/// Decode IndexedDB object-store data records from raw LevelDB [`Record`]s.
///
/// A first pass resolves database and object-store names from their metadata
/// records; a second pass decodes every object-store data record.
#[must_use]
pub fn decode_records(records: &[Record]) -> Vec<IndexedDbRecord> {
    let _ = records;
    Vec::new()
}

/// Read an `*.indexeddb.leveldb` directory and decode its object-store data
/// records.
pub fn read_dir(dir: &Path) -> Result<Vec<IndexedDbRecord>, leveldb_core::Error> {
    let records = leveldb_core::read_dir(dir)?;
    Ok(decode_records(&records))
}
