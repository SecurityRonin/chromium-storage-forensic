//! Fuzz the Local Storage decoder on an arbitrary record.
//!
//! Exercises the `META:` / `_origin\x00script_key` key splitting, the
//! StorageMetadata protobuf-varint parse, and the type-prefixed UTF-16LE /
//! Latin-1 value decode. Must never panic on hostile key/value bytes (lone
//! surrogates, lying varints, dangling half code units).
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
    let _ = chromium_storage_localstorage::decode_records(&[rec]);
});
