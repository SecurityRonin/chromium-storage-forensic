//! Tests for the IndexedDB reader.
//!
//! Tier-2 (real-artifact) oracle: `tests/data/indexeddb/…indexeddb.leveldb` was
//! minted by driving headless Google Chrome to open database `mintdb`, create
//! object store `notes`, and `put({title:'first note', n:42, tags:['a','b']},
//! 'note-1')`, then copying the LevelDB directory (see `tests/data/README.md`).
//! Those known writes are the ground truth.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromium_storage_indexeddb::V8Value;
use chromium_storage_indexeddb::{decode_records, read_dir, IdbKey, IndexedDbRecord, RecordValue};
use leveldb_core::Record;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn note_record() -> IndexedDbRecord {
    let recs = read_dir(&data_dir()).expect("read minted IndexedDB dir");
    recs.into_iter()
        .find(|r| r.key == IdbKey::String("note-1".to_owned()))
        .expect("record with primary key \"note-1\"")
}

fn obj_get<'a>(v: &'a V8Value, key: &str) -> Option<&'a V8Value> {
    match v {
        V8Value::Object(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, val)| val),
        _ => None,
    }
}

// ─── Tier-2: real minted Chrome IndexedDB ────────────────────────────────────

#[test]
fn resolves_database_and_object_store_names() {
    let r = note_record();
    assert_eq!(r.database.as_deref(), Some("mintdb"));
    assert_eq!(r.object_store.as_deref(), Some("notes"));
}

#[test]
fn decodes_the_primary_key() {
    assert_eq!(note_record().key, IdbKey::String("note-1".to_owned()));
}

#[test]
fn decodes_the_blink_v8_value() {
    let r = note_record();
    let v = match &r.value {
        RecordValue::V8(v) => v.clone(),
        other @ RecordValue::Undecoded { .. } => {
            panic!("expected a decoded V8 value, got {other:?}")
        }
    };
    assert_eq!(
        obj_get(&v, "title"),
        Some(&V8Value::String("first note".into()))
    );
    assert_eq!(obj_get(&v, "n"), Some(&V8Value::Int(42)));
    match obj_get(&v, "tags") {
        Some(V8Value::Array(items)) => {
            let strs: Vec<_> = items
                .iter()
                .filter_map(|i| match i {
                    V8Value::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(strs, vec!["a", "b"]);
        }
        other => panic!("expected tags array, got {other:?}"),
    }
}

#[test]
fn emits_exactly_the_one_live_data_record() {
    // Only one object-store data record was written (note-1).
    let live: Vec<_> = read_dir(&data_dir())
        .unwrap()
        .into_iter()
        .filter(|r| !r.deleted)
        .collect();
    assert_eq!(live.len(), 1, "expected 1 live data record, got {live:?}");
    assert_eq!(live[0].database_id, 1);
    assert_eq!(live[0].object_store_id, 1);
}

// ─── Robustness (panic-free) ─────────────────────────────────────────────────

fn rec(key: &[u8], value: &[u8]) -> Record {
    Record {
        key: key.to_vec(),
        value: value.to_vec(),
        seq: 1,
        deleted: false,
        origin_file: PathBuf::from("t.log"),
    }
}

#[test]
fn non_data_records_are_ignored() {
    // A database-metadata record (index_id 0) is not an object-store data record.
    let out = decode_records(&[rec(&[0x00, 0x01, 0x00, 0x00, 0x04], &[0x00])]);
    assert!(out.is_empty());
}

#[test]
fn truncated_value_surfaces_raw_not_panic() {
    // Valid data-record KeyPrefix(1,1,1) + string key "x", but a garbage value.
    let mut key = vec![0x00, 0x01, 0x01, 0x01];
    key.extend_from_slice(&[0x01, 0x01, 0x00, b'x']); // IDBKey: string len 1 "x"
    let out = decode_records(&[rec(&key, &[0x05, 0xff, 0xff])]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].key, IdbKey::String("x".to_owned()));
    assert!(matches!(out[0].value, RecordValue::Undecoded { .. }));
}

#[test]
fn empty_and_garbage_records_never_panic() {
    let _ = decode_records(&[
        rec(&[], &[]),
        rec(&[0x00], &[]),
        rec(&[0xff; 3], &[0xff; 3]),
    ]);
}
