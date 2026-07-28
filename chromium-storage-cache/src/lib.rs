//! Chromium **Simple Cache** entry reader.
//!
//! The Simple Cache is Chromium's default disk-cache backend on Linux, macOS,
//! Android and ChromeOS (and the backend for `CacheStorage` everywhere). It keeps
//! roughly one `<hash>_0` file per cache entry, bracketed by a fixed
//! `SimpleFileHeader` and one `SimpleFileEOF` trailer per data stream. This reader
//! decodes one `_0` entry file into a [`CacheEntry`]: the cache key (and the
//! request URL carved from it), the HTTP response status line + headers (from the
//! stream-0 `HttpResponseInfo` pickle), the response body (stream 1), the
//! request/response times, and the trailer CRCs.
//!
//! Format constants (magics, field offsets, EOF flag bits) come from the fleet
//! KNOWLEDGE leaf [`forensicnomicon_core::chromium_simple_cache`]. All input is
//! attacker-controllable: parsing is panic-free and every integer read is
//! bounds-checked via [`safe_read`]. A valid header is required; the body,
//! headers and times are best-effort and surface as `Option`/empty when the entry
//! is truncated, so a damaged entry still yields the URL it cached.
//!
//! Authoritative source: Chromium `net/disk_cache/simple/simple_entry_format.h`.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod entry;
mod error;

pub use error::CacheError;

use std::path::{Path, PathBuf};

/// One decoded Simple Cache entry (`<hash>_0` file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheEntry {
    /// The full cache key, verbatim (a NIK prefix + the request URL, space
    /// separated), decoded lossily as UTF-8.
    pub key: String,
    /// The request URL carved from the key (the last whitespace-separated token).
    pub url: String,
    /// The header's `key_hash` (a fast non-cryptographic hash of the key).
    pub key_hash: u32,
    /// The entry format version from the header (`kSimpleEntryVersionOnDisk`).
    pub version: u32,
    /// The HTTP response status line (e.g. `HTTP/1.1 200 OK`), if the stream-0
    /// `HttpResponseInfo` pickle was present and parseable.
    pub status_line: Option<String>,
    /// The HTTP response headers as `(name, value)` pairs, in stored order.
    pub headers: Vec<(String, String)>,
    /// The response body bytes (stream 1).
    pub body: Vec<u8>,
    /// The body's stored crc32, when the stream-1 trailer asserted one.
    pub body_crc32: Option<u32>,
    /// Request time, WebKit microseconds (µs since 1601-01-01 UTC).
    pub request_time_webkit_micros: Option<u64>,
    /// Response time, WebKit microseconds (µs since 1601-01-01 UTC).
    pub response_time_webkit_micros: Option<u64>,
    /// The 32-byte SHA-256 of the key, when the stream-0 trailer carried it.
    pub key_sha256: Option<[u8; 32]>,
}

/// Parse a single Simple Cache `_0` entry file's bytes into a [`CacheEntry`].
///
/// Returns `Err` loudly (with the offending value) when the header is absent or
/// malformed; body/headers/times degrade to empty/`None` on truncation rather
/// than erroring, so a damaged entry still yields its URL.
pub fn parse_entry(bytes: &[u8]) -> Result<CacheEntry, CacheError> {
    entry::parse(bytes)
}

/// Read a Simple Cache directory (`Cache_Data/`), parsing every `*_0` entry file.
///
/// Returns `(path, result)` per entry file so a single corrupt entry surfaces its
/// error without hiding the rest. The directory open is the bootstrap step: if it
/// cannot be read this returns [`std::io::Error`] loudly rather than an empty
/// result.
pub fn read_dir(dir: &Path) -> std::io::Result<Vec<(PathBuf, Result<CacheEntry, CacheError>)>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|name| name.ends_with("_0"))
        {
            paths.push(path);
        }
    }
    paths.sort();

    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        // A per-file read error is a per-artifact miss after a validated
        // directory bootstrap: record it against the path, keep going.
        match std::fs::read(&path) {
            Ok(buf) => out.push((path, parse_entry(&buf))),
            Err(_) => continue,
        }
    }
    Ok(out)
}
