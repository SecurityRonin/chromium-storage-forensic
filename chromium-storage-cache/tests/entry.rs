//! Tests for the Simple Cache entry reader.
//!
//! Tier-2 (real-artifact) oracle: `tests/data/simple-cache/ea2e47cbdc22305e_0`
//! was minted by driving headless Google Chrome to fetch a local cacheable
//! resource whose body and headers are known ground truth (see
//! `tests/data/README.md`). `d9c2c72a2ec24e84_0` is a second real entry
//! (google.com telemetry) used to confirm the layout generalises.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromium_storage_cache::{parse_entry, read_dir, CacheEntry, CacheError};
use std::path::PathBuf;

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/simple-cache"
    ))
}

fn mine() -> CacheEntry {
    let path = data_dir().join("ea2e47cbdc22305e_0");
    let bytes = std::fs::read(path).expect("read minted cache entry");
    parse_entry(&bytes).expect("parse minted cache entry")
}

// ─── Tier-2: real minted Chrome cache entry ──────────────────────────────────

#[test]
fn decodes_the_known_url_and_body() {
    let e = mine();
    assert!(
        e.url.contains("127.0.0.1:8731/cached.txt"),
        "url was {:?}",
        e.url
    );
    assert!(e.key.contains("cached.txt"), "key was {:?}", e.key);
    assert_eq!(e.version, 5);
    assert_eq!(e.body, b"CACHED-CONTENT-MARKER-9427\n");
}

#[test]
fn decodes_the_http_status_and_headers() {
    let e = mine();
    assert_eq!(e.status_line.as_deref(), Some("HTTP/1.0 200 OK"));
    let has = |name: &str, val: &str| {
        e.headers
            .iter()
            .any(|(n, v)| n.eq_ignore_ascii_case(name) && v == val)
    };
    assert!(has("Content-Length", "27"), "headers: {:?}", e.headers);
    assert!(
        has("Cache-Control", "public, max-age=999999"),
        "headers: {:?}",
        e.headers
    );
    assert!(
        has("Content-Type", "text/plain"),
        "headers: {:?}",
        e.headers
    );
}

#[test]
fn decodes_request_and_response_times() {
    let e = mine();
    let req = e.request_time_webkit_micros.expect("request time");
    let resp = e.response_time_webkit_micros.expect("response time");
    // 2026 in WebKit µs (since 1601) is > 1.3e16; request precedes response.
    assert!(req > 13_000_000_000_000_000, "request time {req}");
    assert!(resp >= req, "response {resp} should be >= request {req}");
}

#[test]
fn carries_the_key_sha256() {
    assert!(mine().key_sha256.is_some());
}

#[test]
fn read_dir_returns_both_real_entries() {
    let entries = read_dir(&data_dir()).expect("read simple-cache dir");
    let ok: Vec<_> = entries
        .iter()
        .filter_map(|(_, r)| r.as_ref().ok())
        .collect();
    assert_eq!(ok.len(), 2, "expected 2 parseable entries");
    assert!(
        ok.iter().any(|e| e.url.contains("cached.txt")),
        "expected the minted cached.txt entry among {:?}",
        ok.iter().map(|e| &e.url).collect::<Vec<_>>()
    );
}

// ─── Robustness (panic-free on malformed input) ──────────────────────────────

#[test]
fn empty_buffer_is_too_short_not_a_panic() {
    assert!(matches!(parse_entry(&[]), Err(CacheError::TooShort { .. })));
}

#[test]
fn wrong_magic_is_reported_with_the_bytes() {
    let mut buf = vec![0u8; 32];
    buf[..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    match parse_entry(&buf) {
        Err(CacheError::BadMagic { found }) => assert_eq!(found, [1, 2, 3, 4, 5, 6, 7, 8]),
        other => panic!("expected BadMagic, got {other:?}"),
    }
}

#[test]
fn oversized_key_length_is_rejected() {
    // Valid magic, but key_length claims far more than the buffer holds.
    let mut buf = vec![0u8; 24];
    buf[..8].copy_from_slice(&0xfcfb_6d1b_a772_5c30u64.to_le_bytes());
    buf[12..16].copy_from_slice(&0xffff_ffffu32.to_le_bytes()); // key_length
    assert!(matches!(
        parse_entry(&buf),
        Err(CacheError::KeyLengthOutOfRange { .. })
    ));
}

#[test]
fn valid_header_without_trailers_still_yields_the_url() {
    // Header + key, but no EOF trailers (truncated entry): URL survives, body empty.
    let key = b"1/0/_dk_http://x http://x http://x/asset.js";
    let mut buf = Vec::new();
    buf.extend_from_slice(&0xfcfb_6d1b_a772_5c30u64.to_le_bytes());
    buf.extend_from_slice(&5u32.to_le_bytes()); // version
    buf.extend_from_slice(&(key.len() as u32).to_le_bytes()); // key_length
    buf.extend_from_slice(&0u32.to_le_bytes()); // key_hash
    buf.extend_from_slice(&0u32.to_le_bytes()); // padding
    buf.extend_from_slice(key);
    let e = parse_entry(&buf).expect("header-only entry parses");
    assert!(e.url.ends_with("/asset.js"), "url {:?}", e.url);
    assert!(e.body.is_empty());
    assert!(e.status_line.is_none());
}
