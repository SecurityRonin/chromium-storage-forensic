//! Validate the IndexedDB analyzer against the real minted Chrome store.
//!
//! Tier-2: `tests/data/indexeddb/…indexeddb.leveldb` was minted by driving
//! headless Chrome (provenance in `tests/data/README.md`). Its one user record
//! decodes cleanly and resolves both its database and object-store names, so the
//! analyzer must find **zero** anomalies — a non-empty result would be a false
//! positive. The corrupted-value case is the positive control.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromium_storage_indexeddb_forensic::{
    analyze, read_dir, IdbAnomalyKind, IndexedDbRecord, RecordValue,
};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn records() -> Vec<IndexedDbRecord> {
    read_dir(&data_dir()).expect("read minted IndexedDB dir")
}

#[test]
fn real_records_are_clean_no_false_positives() {
    let recs = records();
    assert!(!recs.is_empty(), "expected records from the minted store");
    let findings = analyze(&recs);
    assert!(
        findings.is_empty(),
        "expected no anomalies on a genuine store, got {:?}",
        findings.iter().map(|f| f.kind.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn an_undecodable_value_is_caught() {
    // Force a real record's value into the Undecoded state: the decode-integrity
    // check must fire. Proves the check can go red (positive control).
    let mut recs = records();
    let target = recs
        .iter_mut()
        .find(|r| matches!(r.value, RecordValue::V8(_)))
        .expect("a decoded V8 value to corrupt");
    target.value = RecordValue::Undecoded {
        raw: vec![0xDE, 0xAD],
        error: "injected: truncated structured-clone stream".to_owned(),
    };
    let findings = analyze(&recs);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f.kind, IdbAnomalyKind::ValueUndecodable { .. })),
        "an undecodable value must trip the decode-integrity check"
    );
}

#[test]
fn an_orphaned_record_is_caught() {
    // Drop a record's resolved object-store name: the orphan check must fire.
    let mut recs = records();
    recs[0].object_store = None;
    let findings = analyze(&recs);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f.kind, IdbAnomalyKind::OrphanedRecord { .. })),
        "a record with an unresolved object-store name must trip the orphan check"
    );
}
