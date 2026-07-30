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

// ─── IDBKey decoding, tag by tag ─────────────────────────────────────────────
//
// The encoding is Chromium `indexed_db_leveldb_coding.cc` `EncodeIDBKey`: a
// one-byte type tag then a type-specific body. `KeyPrefix(1,1,1)` (descriptor
// 0x00, one byte per id, index id 1 = INDEX_ID_OBJECT_STORE_DATA) makes the
// record an object-store *data* record, so `decode_records` decodes its tail.

/// A data-record key: `KeyPrefix(1,1,1)` followed by the raw `IDBKey` bytes.
fn data_key(idb_key: &[u8]) -> Vec<u8> {
    let mut k = vec![0x00, 0x01, 0x01, 0x01];
    k.extend_from_slice(idb_key);
    k
}

/// The `IdbKey` a data record carrying `idb_key` in its tail decodes to.
fn decoded_key(idb_key: &[u8]) -> IdbKey {
    let out = decode_records(&[rec(&data_key(idb_key), &[])]);
    assert_eq!(out.len(), 1, "expected one data record for {idb_key:02x?}");
    out[0].key.clone()
}

#[test]
fn decodes_the_null_and_min_key_tags() {
    assert_eq!(decoded_key(&[0x00]), IdbKey::Null);
    assert_eq!(decoded_key(&[0x05]), IdbKey::Min);
}

#[test]
fn decodes_number_and_date_keys() {
    // Both bodies are a raw little-endian f64; only the tag distinguishes them.
    let mut number = vec![0x03];
    number.extend_from_slice(&42.5f64.to_le_bytes());
    assert_eq!(decoded_key(&number), IdbKey::Number(42.5));

    let mut date = vec![0x02];
    date.extend_from_slice(&1_700_000_000_000f64.to_le_bytes());
    assert_eq!(decoded_key(&date), IdbKey::Date(1_700_000_000_000f64));
}

#[test]
fn decodes_a_binary_key() {
    // tag 0x06, varint length 3, then the bytes.
    let key = decoded_key(&[0x06, 0x03, 0xde, 0xad, 0xbe]);
    assert_eq!(key, IdbKey::Binary(vec![0xde, 0xad, 0xbe]));
}

#[test]
fn decodes_a_nested_array_key() {
    // tag 0x04, varint count 2, then Number(1.0) and String("z").
    let mut key = vec![0x04, 0x02, 0x03];
    key.extend_from_slice(&1.0f64.to_le_bytes());
    key.extend_from_slice(&[0x01, 0x01, 0x00, b'z']); // string, 1 unit, UTF-16BE
    assert_eq!(
        decoded_key(&key),
        IdbKey::Array(vec![IdbKey::Number(1.0), IdbKey::String("z".to_owned())])
    );
}

#[test]
fn an_unrecognised_key_tag_surfaces_the_raw_bytes() {
    // 0x7f is not a defined IDBKey type byte: the tag and everything after it
    // must reach the analyst verbatim rather than be silently dropped.
    assert_eq!(
        decoded_key(&[0x7f, 0x01, 0x02]),
        IdbKey::Invalid(vec![0x7f, 0x01, 0x02])
    );
}

#[test]
fn an_empty_key_tail_is_invalid() {
    // The key is exactly the KeyPrefix — there is no IDBKey to decode.
    assert_eq!(decoded_key(&[]), IdbKey::Invalid(Vec::new()));
}

#[test]
fn a_truncated_binary_key_is_invalid_not_a_panic() {
    // Length field claims 8 bytes; only 1 is present.
    assert_eq!(
        decoded_key(&[0x06, 0x08, 0x01]),
        IdbKey::Invalid(vec![0x06, 0x08, 0x01])
    );
}

#[test]
fn a_multi_byte_varint_length_is_read_in_full() {
    // 200 body bytes → EncodeVarInt(200) = 0xC8 0x01 (a continuation group), so
    // this drives the multi-byte path a single-byte length never reaches.
    let body: Vec<u8> = (0..200u16).map(|i| i as u8).collect();
    let mut key = vec![0x06, 0xC8, 0x01];
    key.extend_from_slice(&body);
    assert_eq!(decoded_key(&key), IdbKey::Binary(body));
}

#[test]
fn a_varint_overflowing_u64_is_invalid_not_a_panic() {
    // Eleven continuation bytes push the shift past 64: the length cannot be
    // represented, so the key is surfaced raw instead of wrapping around.
    let mut key = vec![0x06];
    key.extend_from_slice(&[0x80; 11]);
    assert_eq!(decoded_key(&key), IdbKey::Invalid(key));
}

#[test]
fn object_store_metadata_other_than_the_name_is_not_read_as_the_name() {
    // Two decoys for the object-store-name resolver, both under KeyPrefix(1,0,0)
    // with OBJECT_STORE_META_TYPE (50):
    //   * a *key path* metadata record (subtype 1, not OS_META_NAME 0);
    //   * a truncated record whose os-id varint is missing entirely.
    // Neither names object store 1, so the data record's name stays unresolved.
    let key_path_meta = vec![0x00, 0x01, 0x00, 0x00, 50, 0x01, 0x01];
    let truncated_meta = vec![0x00, 0x01, 0x00, 0x00, 50];
    let out = decode_records(&[
        rec(&key_path_meta, &[0x00, b'n']),
        rec(&truncated_meta, &[0x00, b'n']),
        rec(&data_key(&[0x00]), &[]),
    ]);
    assert_eq!(out.len(), 1, "only the data record is emitted");
    assert_eq!(
        out[0].object_store, None,
        "key-path / truncated metadata must not be mistaken for the store name"
    );
}
