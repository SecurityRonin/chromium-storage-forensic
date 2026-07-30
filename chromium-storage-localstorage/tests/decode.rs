//! Tests for the Local Storage reader.
//!
//! Tier-2 (real-artifact) oracle: `tests/data/local-storage/leveldb` was minted
//! by driving headless Google Chrome against a local page that wrote two known
//! `localStorage` keys, then copying the profile's `Local Storage/leveldb`
//! directory (see `tests/data/README.md`). The known writes are the ground truth.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromium_storage_localstorage::{decode_records, read_dir, Encoding, LocalStorageRecord};
use leveldb_core::Record;
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/local-storage/leveldb"
    ))
}

fn rec(key: &[u8], value: &[u8]) -> Record {
    Record {
        key: key.to_vec(),
        value: value.to_vec(),
        seq: 1,
        deleted: false,
        origin_file: PathBuf::from("t.log"),
    }
}

// ─── Tier-2: real minted Chrome data ─────────────────────────────────────────

#[test]
fn real_chrome_local_storage_decodes_the_known_writes() {
    let records = read_dir(&data_dir()).expect("read minted Local Storage dir");
    assert!(
        !records.is_empty(),
        "expected records from the minted store"
    );

    // The two known localStorage writes: greeting=hello, mint_ls_key=mint_ls_value_δ.
    let data: Vec<_> = records
        .iter()
        .filter_map(|r| match r {
            LocalStorageRecord::Data {
                origin,
                script_key,
                value,
                ..
            } => Some((origin.clone(), script_key.text.clone(), value.text.clone())),
            _ => None,
        })
        .collect();

    assert!(
        data.iter()
            .any(|(o, k, v)| o.contains("127.0.0.1:8731") && k == "greeting" && v == "hello"),
        "missing greeting=hello; got {data:?}"
    );
    assert!(
        data.iter().any(|(o, k, v)| o.contains("127.0.0.1:8731")
            && k == "mint_ls_key"
            && v == "mint_ls_value_\u{03b4}"),
        "missing mint_ls_key=mint_ls_value_δ (UTF-16 path); got {data:?}"
    );
}

#[test]
fn real_chrome_local_storage_has_origin_meta_record() {
    let records = read_dir(&data_dir()).expect("read minted Local Storage dir");
    let meta: Vec<_> = records
        .iter()
        .filter_map(|r| match r {
            LocalStorageRecord::Meta {
                origin,
                timestamp_webkit_micros,
                ..
            } => Some((origin.clone(), *timestamp_webkit_micros)),
            _ => None,
        })
        .collect();
    assert!(
        meta.iter()
            .any(|(o, ts)| o.contains("127.0.0.1:8731") && *ts > 0),
        "expected a META record with a non-zero WebKit timestamp; got {meta:?}"
    );
}

// ─── Decoder unit coverage (hand-built records) ──────────────────────────────

#[test]
fn data_record_utf16le_value() {
    // key: "_" + origin + NUL + <0x00 utf16le "k">, value: <0x00 utf16le "hi">
    let mut key = b"_http://ex.com\x00".to_vec();
    key.push(0x00); // UTF-16LE marker for the script key
    key.extend_from_slice(&[b'k', 0]);
    let value = [0x00u8, b'h', 0, b'i', 0].to_vec();
    let out = decode_records(&[rec(&key, &value)]);
    match &out[0] {
        LocalStorageRecord::Data {
            origin,
            script_key,
            value,
            ..
        } => {
            assert_eq!(origin, "http://ex.com");
            assert_eq!(script_key.text, "k");
            assert_eq!(value.text, "hi");
            assert_eq!(value.encoding, Encoding::Utf16Le);
            assert!(!value.lossy);
        }
        other => panic!("expected Data, got {other:?}"),
    }
}

#[test]
fn data_record_latin1_value() {
    let mut key = b"_http://ex.com\x00".to_vec();
    key.push(0x01);
    key.extend_from_slice(b"key");
    let value = [0x01u8, b'v', b'a', b'l'].to_vec();
    let out = decode_records(&[rec(&key, &value)]);
    match &out[0] {
        LocalStorageRecord::Data {
            script_key, value, ..
        } => {
            assert_eq!(script_key.text, "key");
            assert_eq!(value.text, "val");
            assert_eq!(value.encoding, Encoding::Latin1);
        }
        other => panic!("expected Data, got {other:?}"),
    }
}

#[test]
fn deletion_tombstone_keeps_empty_value() {
    let mut key = b"_http://ex.com\x00".to_vec();
    key.push(0x01);
    key.extend_from_slice(b"gone");
    let mut r = rec(&key, b"");
    r.deleted = true;
    let out = decode_records(&[r]);
    match &out[0] {
        LocalStorageRecord::Data { value, deleted, .. } => {
            assert!(*deleted);
            assert_eq!(value.encoding, Encoding::Empty);
        }
        other => panic!("expected Data tombstone, got {other:?}"),
    }
}

#[test]
fn meta_record_parses_timestamp_and_size() {
    // StorageMetadata protobuf: field 1 (tag 0x08) = ts varint, field 2 (0x10) = size.
    // ts = 300 -> varint AC 02 ; size = 5.
    let value = [0x08u8, 0xAC, 0x02, 0x10, 0x05].to_vec();
    let out = decode_records(&[rec(b"META:http://ex.com", &value)]);
    match &out[0] {
        LocalStorageRecord::Meta {
            origin,
            timestamp_webkit_micros,
            size,
            ..
        } => {
            assert_eq!(origin, "http://ex.com");
            assert_eq!(*timestamp_webkit_micros, 300);
            assert_eq!(*size, Some(5));
        }
        other => panic!("expected Meta, got {other:?}"),
    }
}

#[test]
fn version_key_is_other() {
    let out = decode_records(&[rec(b"VERSION", b"1")]);
    assert!(matches!(out[0], LocalStorageRecord::Other { .. }));
}

#[test]
fn meta_record_without_the_size_field_reports_no_size() {
    // Field 1 only (tag 0x08, ts = 300): the optional field-2 size is absent, so
    // `size` must be None rather than a fabricated zero.
    let out = decode_records(&[rec(b"META:http://ex.com", &[0x08, 0xAC, 0x02])]);
    match &out[0] {
        LocalStorageRecord::Meta {
            timestamp_webkit_micros,
            size,
            ..
        } => {
            assert_eq!(*timestamp_webkit_micros, 300);
            assert_eq!(*size, None);
        }
        other => panic!("expected Meta, got {other:?}"),
    }
}

#[test]
fn deleted_meta_record_surfaces_as_a_meta_tombstone() {
    // A cleared origin: there is no protobuf to parse, but the tombstone itself
    // is forensically meaningful (the origin's storage was wiped at this seq).
    let mut r = rec(b"META:http://ex.com", b"");
    r.deleted = true;
    r.seq = 77;
    let out = decode_records(&[r]);
    match &out[0] {
        LocalStorageRecord::Meta {
            origin,
            timestamp_webkit_micros,
            size,
            seq,
            deleted,
        } => {
            assert_eq!(origin, "http://ex.com");
            assert_eq!(*timestamp_webkit_micros, 0);
            assert_eq!(*size, None);
            assert_eq!(*seq, 77);
            assert!(*deleted);
        }
        other => panic!("expected a Meta tombstone, got {other:?}"),
    }
}

#[test]
fn meta_record_with_an_unparseable_protobuf_falls_back_to_other() {
    // Three ways the StorageMetadata payload can fail to parse; each must keep
    // the raw key rather than invent a timestamp.
    let wrong_field = rec(b"META:http://ex.com", &[0x09, 0x01]); // field 1 wire type 1
    let overflowing = rec(b"META:http://ex.com", &[0x80; 11]); // varint > u64
    let empty = rec(b"META:http://ex.com", b"");
    let out = decode_records(&[wrong_field, overflowing, empty]);
    for (i, r) in out.iter().enumerate() {
        match r {
            LocalStorageRecord::Other { key, .. } => {
                assert_eq!(key, b"META:http://ex.com", "record {i}");
            }
            other => panic!("record {i}: expected Other, got {other:?}"),
        }
    }
}

#[test]
fn a_data_prefixed_key_without_the_nul_separator_is_other() {
    // "_" prefix but no NUL: the origin/script-key split is undefined, so the
    // raw key is surfaced instead of being guessed at.
    let out = decode_records(&[rec(b"_http://ex.com", b"\x01v")]);
    match &out[0] {
        LocalStorageRecord::Other { key, .. } => assert_eq!(key, b"_http://ex.com"),
        other => panic!("expected Other, got {other:?}"),
    }
}

#[test]
fn an_empty_script_key_decodes_as_the_empty_value() {
    // The NUL is the last byte, so the script-key body has no marker at all.
    let out = decode_records(&[rec(b"_http://ex.com\x00", &[0x01, b'v'])]);
    match &out[0] {
        LocalStorageRecord::Data {
            script_key, value, ..
        } => {
            assert_eq!(script_key.text, "");
            assert_eq!(script_key.encoding, Encoding::Empty);
            assert!(!script_key.lossy);
            assert_eq!(value.text, "v");
        }
        other => panic!("expected Data, got {other:?}"),
    }
}

#[test]
fn a_dangling_utf16_byte_is_lossy_but_keeps_what_decoded() {
    // Marker 0x00 (UTF-16LE) with an odd body: 'h' decodes, the trailing byte is
    // half a code unit. The value must report itself lossy.
    let mut key = b"_http://ex.com\x00".to_vec();
    key.push(0x01);
    key.extend_from_slice(b"k");
    let out = decode_records(&[rec(&key, &[0x00, b'h', 0x00, b'i'])]);
    match &out[0] {
        LocalStorageRecord::Data { value, .. } => {
            assert_eq!(value.text, "h");
            assert_eq!(value.encoding, Encoding::Utf16Le);
            assert!(value.lossy, "a dangling half code unit is a lossy decode");
            assert_eq!(value.raw, vec![0x00, b'h', 0x00, b'i']);
        }
        other => panic!("expected Data, got {other:?}"),
    }
}

#[test]
fn a_lone_surrogate_becomes_u_fffd_and_marks_the_value_lossy() {
    // 0xD800 is a high surrogate with no low surrogate following it — not a
    // scalar value, so it renders U+FFFD.
    let mut key = b"_http://ex.com\x00".to_vec();
    key.push(0x01);
    key.extend_from_slice(b"k");
    let out = decode_records(&[rec(&key, &[0x00, 0x00, 0xD8, b'z', 0x00])]);
    match &out[0] {
        LocalStorageRecord::Data { value, .. } => {
            assert_eq!(value.text, "\u{FFFD}z");
            assert_eq!(value.encoding, Encoding::Utf16Le);
            assert!(value.lossy);
        }
        other => panic!("expected Data, got {other:?}"),
    }
}

#[test]
fn unknown_marker_is_lossy_but_never_panics() {
    let mut key = b"_http://ex.com\x00".to_vec();
    key.push(0x01);
    key.extend_from_slice(b"k");
    let value = [0x7Fu8, 0xDE, 0xAD].to_vec(); // marker 0x7F is not 0x00/0x01
    let out = decode_records(&[rec(&key, &value)]);
    match &out[0] {
        LocalStorageRecord::Data { value, .. } => {
            assert!(value.lossy);
            assert_eq!(value.encoding, Encoding::Unknown(0x7F));
            assert_eq!(value.raw, vec![0x7F, 0xDE, 0xAD]);
        }
        other => panic!("expected Data, got {other:?}"),
    }
}
