//! Validate the cache analyzer against real Simple Cache entries.
//!
//! Tier-2: the two committed entries in `tests/data/simple-cache/` are genuine
//! (minted by Chromium, provenance in `tests/data/README.md`). They are
//! well-formed, so every self-validating check must pass — the analyzer must
//! find **zero** anomalies. A non-empty result here would be a false positive
//! (e.g. a wrong CRC/SHA recomputation), which is exactly what this guards.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromium_storage_cache_forensic::{analyze, parse_entry, CacheAnomalyKind};

fn entry(name: &str) -> chromium_storage_cache_forensic::CacheEntry {
    let path = format!(
        "{}/../tests/data/simple-cache/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).expect("read real cache entry");
    parse_entry(&bytes).expect("parse real cache entry")
}

#[test]
fn real_entries_are_clean_no_false_positives() {
    for name in ["ea2e47cbdc22305e_0", "d9c2c72a2ec24e84_0"] {
        let e = entry(name);
        let findings = analyze(&e);
        assert!(
            findings.is_empty(),
            "{name}: expected no anomalies on a genuine entry, got {:?}",
            findings.iter().map(|f| f.kind.code()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn a_corrupted_body_is_caught() {
    // Flip the body of a real entry: its stored CRC-32 no longer matches, so the
    // integrity check must fire. Proves the check can go red (positive control).
    let mut e = entry("ea2e47cbdc22305e_0");
    if e.body_crc32.is_none() || e.body.is_empty() {
        eprintln!("skip: fixture carries no body CRC to contradict");
        return;
    }
    e.body[0] ^= 0xFF;
    let findings = analyze(&e);
    assert!(
        findings
            .iter()
            .any(|f| matches!(f.kind, CacheAnomalyKind::BodyCrc32Mismatch { .. })),
        "a body edit must trip the CRC-32 integrity check"
    );
}
