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
mod prefix;
mod util;
mod value;

pub use blob_decoder::v8_value::V8Value;
pub use key::IdbKey;
pub use value::RecordValue;

use forensicnomicon_core::chromium_indexeddb::{
    INDEX_ID_OBJECT_STORE_DATA, OBJECT_STORE_META_TYPE, OS_META_NAME,
};
use leveldb_core::Record;
use prefix::KeyPrefix;
use std::collections::HashMap;
use std::path::Path;
use util::{decode_utf16be, read_varint};

/// Global-metadata type byte introducing a `(origin, database name) → database
/// id` index record (`kDatabaseNameTypeByte`, Chromium
/// `indexed_db_leveldb_coding.cc`). Not yet exported by forensicnomicon-core.
const DATABASE_NAME_TYPE_BYTE: u8 = 201;

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
/// records; a second pass decodes every object-store data record (index id
/// [`INDEX_ID_OBJECT_STORE_DATA`]).
#[must_use]
pub fn decode_records(records: &[Record]) -> Vec<IndexedDbRecord> {
    let names = Names::resolve(records);

    let mut out = Vec::new();
    for r in records {
        let Some(prefix) = KeyPrefix::parse(&r.key) else {
            continue;
        };
        if prefix.index_id != u64::from(INDEX_ID_OBJECT_STORE_DATA) {
            continue; // not an object-store *data* record
        }
        let tail = &r.key[prefix.len.min(r.key.len())..];
        let key = IdbKey::decode(tail);
        let value = decode_value(&r.value);
        out.push(IndexedDbRecord {
            database_id: prefix.database_id,
            object_store_id: prefix.object_store_id,
            database: names.database(prefix.database_id),
            object_store: names.object_store(prefix.database_id, prefix.object_store_id),
            key,
            value,
            seq: r.seq,
            deleted: r.deleted,
        });
    }
    out
}

/// Decode an object-store record value: strip the wrapper-version varint, then
/// deserialize the Blink `SerializedScriptValue`. On any failure the raw
/// (post-varint) bytes and the error are retained.
fn decode_value(value: &[u8]) -> RecordValue {
    // The value is a wrapper-version varint followed by the Blink SSV blob.
    let body_off = read_varint(value, 0).map_or(0, |(_, off)| off);
    let body = value.get(body_off..).unwrap_or(&[]);
    match blob_decoder::v8_value::deserialize_blink(body) {
        Ok(v) => RecordValue::V8(v),
        Err(e) => RecordValue::Undecoded {
            raw: body.to_vec(),
            error: e.to_string(),
        },
    }
}

/// Name-resolution tables built from the metadata records in one pass.
struct Names {
    databases: HashMap<u64, String>,
    object_stores: HashMap<(u64, u64), String>,
}

impl Names {
    fn resolve(records: &[Record]) -> Names {
        let mut databases = HashMap::new();
        let mut object_stores = HashMap::new();
        for r in records {
            if r.deleted {
                continue;
            }
            let Some(prefix) = KeyPrefix::parse(&r.key) else {
                continue;
            };
            let tail = &r.key[prefix.len.min(r.key.len())..];

            // Global `(origin, dbname) → db_id` index: KeyPrefix(0,0,0) +
            // DATABASE_NAME_TYPE_BYTE + origin(StringWithLength) +
            // dbname(StringWithLength); value = db_id (LE int).
            if prefix.database_id == 0
                && prefix.object_store_id == 0
                && prefix.index_id == 0
                && tail.first() == Some(&DATABASE_NAME_TYPE_BYTE)
            {
                if let Some(name) = database_name_from_index(&tail[1..]) {
                    let db_id = le_int(&r.value);
                    databases.entry(db_id).or_insert(name);
                }
                continue;
            }

            // Object-store name: KeyPrefix(db,0,0) + OBJECT_STORE_META_TYPE +
            // varint(os_id) + OS_META_NAME; value = UTF-16BE name (no length).
            if prefix.object_store_id == 0
                && prefix.index_id == 0
                && tail.first() == Some(&OBJECT_STORE_META_TYPE)
            {
                if let Some((os_id, off)) = read_varint(tail, 1) {
                    if tail.get(off) == Some(&OS_META_NAME) {
                        object_stores
                            .entry((prefix.database_id, os_id))
                            .or_insert_with(|| decode_utf16be(&r.value));
                    }
                }
            }
        }
        Names {
            databases,
            object_stores,
        }
    }

    fn database(&self, db_id: u64) -> Option<String> {
        self.databases.get(&db_id).cloned()
    }

    fn object_store(&self, db_id: u64, os_id: u64) -> Option<String> {
        self.object_stores.get(&(db_id, os_id)).cloned()
    }
}

/// Read the database name (the second `StringWithLength`) from a
/// `DatabaseNameKey` tail: `origin(StringWithLength) + dbname(StringWithLength)`.
fn database_name_from_index(data: &[u8]) -> Option<String> {
    let (origin_units, off) = read_varint(data, 0)?;
    let after_origin = off.checked_add((origin_units as usize).checked_mul(2)?)?;
    let (name_units, off) = read_varint(data, after_origin)?;
    let end = off.checked_add((name_units as usize).checked_mul(2)?)?;
    let body = data.get(off..end)?;
    Some(decode_utf16be(body))
}

/// Decode a minimal little-endian unsigned integer (IndexedDB `EncodeInt`).
fn le_int(data: &[u8]) -> u64 {
    let mut v = 0u64;
    for (i, &b) in data.iter().take(8).enumerate() {
        v |= u64::from(b) << (8 * i);
    }
    v
}

/// Read an `*.indexeddb.leveldb` directory and decode its object-store data
/// records.
pub fn read_dir(dir: &Path) -> Result<Vec<IndexedDbRecord>, leveldb_core::Error> {
    let records = leveldb_core::read_dir(dir)?;
    Ok(decode_records(&records))
}
