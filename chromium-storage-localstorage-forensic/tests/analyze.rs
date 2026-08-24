//! Validate the Local Storage analyzer against the real minted Chrome store.
//!
//! Tier-2: `tests/data/local-storage/leveldb` was minted by driving headless
//! Chrome to write two known keys (provenance in `tests/data/README.md`). Its
//! META timestamp is genuine and every DATA row has its META, so the analyzer
//! must find **zero** anomalies — a non-empty result would be a false positive.
//! Each mutated case below is a positive control proving a check can go red.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromium_storage_localstorage_forensic::{
    analyze, read_dir, LocalStorageRecord, LsAnomalyKind,
};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/local-storage/leveldb"
    ))
}

fn records() -> Vec<LocalStorageRecord> {
    read_dir(&data_dir()).expect("read minted Local Storage dir")
}

#[test]
fn real_store_is_clean_no_false_positives() {
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
fn an_implausible_meta_timestamp_is_caught() {
    let mut recs = records();
    let meta = recs
        .iter_mut()
        .find_map(|r| match r {
            LocalStorageRecord::Meta {
                timestamp_webkit_micros,
                deleted: false,
                ..
            } => Some(timestamp_webkit_micros),
            _ => None,
        })
        .expect("a live META record to backdate");
    *meta = 1; // WebKit µs of ~1601 — impossible for this format.
    let findings = analyze(&recs);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f.kind, LsAnomalyKind::ImplausibleMetaTimestamp { .. })),
        "a pre-2010 META timestamp must trip the timeline check"
    );
}

#[test]
fn orphaned_data_is_caught() {
    let mut recs = records();
    recs.push(LocalStorageRecord::Data {
        origin: "https://orphan.example".to_owned(),
        script_key: value("k"),
        value: value("v"),
        seq: 9999,
        deleted: false,
    });
    let findings = analyze(&recs);
    assert!(
        findings
            .iter()
            .any(|f| matches!(&f.kind, LsAnomalyKind::OrphanedData { origin } if origin == "https://orphan.example")),
        "a DATA row with no META must trip the orphan check"
    );
}

#[test]
fn housekeeping_other_keys_are_not_flagged() {
    // The reader surfaces the store's VERSION and META-access rows as `Other`;
    // they are present in every real store, so the analyzer must not grade them.
    let mut recs = records();
    recs.push(LocalStorageRecord::Other {
        key: b"VERSION".to_vec(),
        seq: 9999,
        deleted: false,
    });
    assert!(
        analyze(&recs).is_empty(),
        "housekeeping Other keys must not be treated as anomalies"
    );
}

/// Build a `StorageValue` for a Latin-1 string, for synthesizing test records.
fn value(text: &str) -> chromium_storage_localstorage_forensic::StorageValue {
    chromium_storage_localstorage_forensic::StorageValue {
        text: text.to_owned(),
        raw: text.as_bytes().to_vec(),
        encoding: chromium_storage_localstorage_forensic::Encoding::Latin1,
        lossy: false,
    }
}
