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
fn errors_display_the_offending_value_and_offset() {
    // Every CacheError renders fail-loud: the offending value travels with the
    // message. Derive each variant from parse_entry (not a hand-built enum) and
    // assert on its Display, so the reader always gets the evidence.

    // TooShort: an empty buffer reports the available count and the 24-byte need.
    let too_short = parse_entry(&[]).unwrap_err().to_string();
    assert!(
        too_short.contains("too short")
            && too_short.contains("0 bytes")
            && too_short.contains("24"),
        "TooShort display was {too_short:?}"
    );

    // BadMagic: the eight bytes found are shown verbatim (hex).
    let mut wrong = vec![0u8; 32];
    wrong[..8].copy_from_slice(&[0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03, 0x04]);
    let bad_magic = parse_entry(&wrong).unwrap_err().to_string();
    assert!(
        bad_magic.contains("bad Simple Cache entry magic")
            && bad_magic.contains("de")
            && bad_magic.contains("ef"),
        "BadMagic display was {bad_magic:?}"
    );

    // KeyLengthOutOfRange: the declared length and the available byte count.
    let mut oversized = vec![0u8; 24];
    oversized[..8].copy_from_slice(&0xfcfb_6d1b_a772_5c30u64.to_le_bytes());
    oversized[12..16].copy_from_slice(&0xffff_ffffu32.to_le_bytes()); // key_length
    let out_of_range = parse_entry(&oversized).unwrap_err().to_string();
    assert!(
        out_of_range.contains("key_length 4294967295") && out_of_range.contains("exceeds"),
        "KeyLengthOutOfRange display was {out_of_range:?}"
    );
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

// ─── Hand-built entries: the stream/trailer edge shapes ──────────────────────
//
// Layout per `SimpleFileHeader` / `SimpleFileEOF` (Chromium `simple_entry_format.h`):
// [header 24B][key][stream 1 body][EOF 24B][stream 0 pickle][key SHA-256?][EOF 24B].

const INITIAL_MAGIC: u64 = 0xfcfb_6d1b_a772_5c30;
const FINAL_MAGIC: u64 = 0xf4fa_6f45_970d_41d8;
const FLAG_HAS_CRC32: u32 = 1 << 0;
const FLAG_HAS_KEY_SHA256: u32 = 1 << 1;

fn header(key: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&INITIAL_MAGIC.to_le_bytes());
    b.extend_from_slice(&5u32.to_le_bytes()); // version
    b.extend_from_slice(&(key.len() as u32).to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes()); // key_hash
    b.extend_from_slice(&0u32.to_le_bytes()); // padding to HEADER_LEN
    b.extend_from_slice(key);
    b
}

fn eof(flags: u32, data_crc32: u32, stream_size: u32) -> Vec<u8> {
    let mut t = Vec::new();
    t.extend_from_slice(&FINAL_MAGIC.to_le_bytes());
    t.extend_from_slice(&flags.to_le_bytes());
    t.extend_from_slice(&data_crc32.to_le_bytes());
    t.extend_from_slice(&stream_size.to_le_bytes());
    t.extend_from_slice(&0u32.to_le_bytes()); // padding to EOF_LEN
    t
}

#[test]
fn a_single_trailer_yields_the_body_and_crc_but_no_response_info() {
    // Stream 1 closed by its trailer, then the file ends: there is no stream-0
    // region to read, so the body and its CRC survive and nothing is invented.
    let mut buf = header(b"http://x/one.bin");
    buf.extend_from_slice(b"BODY");
    buf.extend_from_slice(&eof(FLAG_HAS_CRC32, 0x1234_5678, 4));
    let e = parse_entry(&buf).expect("single-trailer entry parses");
    assert_eq!(e.body, b"BODY");
    assert_eq!(e.body_crc32, Some(0x1234_5678));
    assert!(e.status_line.is_none());
    assert!(e.key_sha256.is_none());
    assert!(e.request_time_webkit_micros.is_none());
}

#[test]
fn a_stream0_region_without_a_pickle_leaves_the_response_info_empty() {
    // Two trailers with nothing between them: the stream-0 region is empty, so
    // there is no "HTTP/" header block and no times to read. No CRC flag and no
    // key-SHA-256 flag either — those fields must stay None, not zero.
    let mut buf = header(b"http://x/two.bin");
    buf.extend_from_slice(b"B");
    buf.extend_from_slice(&eof(0, 0, 1));
    buf.extend_from_slice(&eof(0, 0, 0));
    let e = parse_entry(&buf).expect("empty stream-0 region parses");
    assert_eq!(e.body, b"B");
    assert_eq!(e.body_crc32, None, "no CRC32 flag means no CRC32");
    assert!(e.status_line.is_none());
    assert!(e.headers.is_empty());
    assert!(e.key_sha256.is_none());
    assert!(e.response_time_webkit_micros.is_none());
}

#[test]
fn a_pickle_with_no_http_block_still_yields_the_times() {
    // The pickle is present and carries request/response times at the fixed
    // offsets, but the "HTTP/" header block is missing (truncated entry): the
    // times survive and the status line stays None.
    let mut pickle = vec![0u8; 28];
    pickle[12..20].copy_from_slice(&13_300_000_000_000_000u64.to_le_bytes());
    pickle[20..28].copy_from_slice(&13_300_000_000_000_001u64.to_le_bytes());
    let mut buf = header(b"http://x/three.bin");
    buf.extend_from_slice(b"B");
    buf.extend_from_slice(&eof(0, 0, 1));
    buf.extend_from_slice(&pickle);
    buf.extend_from_slice(&eof(0, 0, pickle.len() as u32));
    let e = parse_entry(&buf).expect("pickle-without-headers entry parses");
    assert_eq!(e.request_time_webkit_micros, Some(13_300_000_000_000_000));
    assert_eq!(e.response_time_webkit_micros, Some(13_300_000_000_000_001));
    assert!(e.status_line.is_none());
    assert!(e.headers.is_empty());
}

#[test]
fn a_key_sha256_flag_with_truncated_bytes_leaves_the_hash_none() {
    // The trailer claims a trailing key SHA-256 but the region holds only 4 of
    // the 32 bytes: the hash must be reported absent rather than part-filled.
    let pickle = b"HTTP/1.1 204 No Content\x00\x00";
    let mut buf = header(b"http://x/four.bin");
    buf.extend_from_slice(b"B");
    buf.extend_from_slice(&eof(0, 0, 1));
    buf.extend_from_slice(pickle);
    buf.extend_from_slice(&[0xAA; 4]); // a truncated SHA-256
    buf.extend_from_slice(&eof(FLAG_HAS_KEY_SHA256, 0, pickle.len() as u32));
    let e = parse_entry(&buf).expect("truncated key-SHA-256 entry parses");
    assert_eq!(e.status_line.as_deref(), Some("HTTP/1.1 204 No Content"));
    assert_eq!(e.key_sha256, None, "a partial hash is no hash");
}
