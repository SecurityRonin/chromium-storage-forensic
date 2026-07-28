//! Fuzz the IndexedDB decoder on an arbitrary record.
//!
//! Exercises the KeyPrefix parse, the IDBKey decode (string/number/date/binary/
//! nested arrays), the metadata name resolution, and the wrapper-varint strip +
//! Blink/V8 deserialize. Must never panic on hostile key/value bytes (lying
//! prefix lengths, huge string/array counts, malformed V8 streams).
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

use leveldb_core::Record;

fuzz_target!(|input: (Vec<u8>, Vec<u8>, u64, bool)| {
    let (key, value, seq, deleted) = input;
    let rec = Record {
        key,
        value,
        seq,
        deleted,
        origin_file: PathBuf::from("fuzz"),
    };
    let _ = chromium_storage_indexeddb::decode_records(&[rec]);
});
