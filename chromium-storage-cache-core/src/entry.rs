//! Simple Cache `_0` entry-file parser.
//!
//! Layout (verified against real minted Chrome entries):
//! ```text
//! [SimpleFileHeader 24B] [key] [stream 1 body] [SimpleFileEOF]
//!                        [stream 0 HttpResponseInfo pickle] [key SHA-256?] [SimpleFileEOF]
//! ```
//! The body is delimited by the first `FINAL_MAGIC` trailer after the key; the
//! `HttpResponseInfo` pickle by the second, whose `stream_size` excludes the
//! optional 32-byte key SHA-256 that follows the pickle.

use crate::{CacheEntry, CacheError};
use forensicnomicon_core::chromium_simple_cache as fmt;
use safe_read::{le_u32, le_u64};

/// Parse a Simple Cache entry file.
pub(crate) fn parse(bytes: &[u8]) -> Result<CacheEntry, CacheError> {
    if bytes.len() < fmt::HEADER_LEN {
        return Err(CacheError::TooShort {
            available: bytes.len(),
        });
    }

    // ── SimpleFileHeader ──────────────────────────────────────────────────
    let magic = le_u64(bytes, fmt::HEADER_MAGIC_OFFSET);
    if magic != fmt::INITIAL_MAGIC {
        let mut found = [0u8; 8];
        found.copy_from_slice(&bytes[..8]);
        return Err(CacheError::BadMagic { found });
    }
    let version = le_u32(bytes, fmt::HEADER_VERSION_OFFSET);
    let key_length = le_u32(bytes, fmt::HEADER_KEY_LENGTH_OFFSET);
    let key_hash = le_u32(bytes, fmt::HEADER_KEY_HASH_OFFSET);

    let key_start = fmt::HEADER_LEN;
    let available = bytes.len() - key_start;
    let key_len = key_length as usize;
    if key_len > available {
        return Err(CacheError::KeyLengthOutOfRange {
            key_length: u64::from(key_length),
            available,
        });
    }
    let key_bytes = &bytes[key_start..key_start + key_len];
    let key = String::from_utf8_lossy(key_bytes).into_owned();
    let url = carve_url(&key);

    // ── Locate the EOF trailers after the key ─────────────────────────────
    let data_start = key_start + key_len;
    let eofs = find_eofs(bytes, data_start);

    let mut entry = CacheEntry {
        key,
        url,
        key_hash,
        version,
        status_line: None,
        headers: Vec::new(),
        body: Vec::new(),
        body_crc32: None,
        request_time_webkit_micros: None,
        response_time_webkit_micros: None,
        key_sha256: None,
    };

    // Stream 1 (body): key end → first trailer.
    let Some(&eof1) = eofs.first() else {
        return Ok(entry); // truncated: URL survives, nothing more to read
    };
    entry.body = bytes[data_start..eof1].to_vec();
    let eof1_flags = le_u32(bytes, eof1 + fmt::EOF_FLAGS_OFFSET);
    if fmt::has_crc32(eof1_flags) {
        entry.body_crc32 = Some(le_u32(bytes, eof1 + fmt::EOF_DATA_CRC32_OFFSET));
    }

    // Stream 0 (HttpResponseInfo pickle): after trailer 1 → second trailer.
    let Some(&eof0) = eofs.get(1) else {
        return Ok(entry);
    };
    let region_start = eof1 + fmt::EOF_LEN;
    // The guard is kept as defence-in-depth: it is what makes the `bytes[..]`
    // slice below infallible no matter how `find_eofs` is later changed.
    if region_start <= eof0 {
        let region = &bytes[region_start..eof0];
        let stream_size = le_u32(bytes, eof0 + fmt::EOF_STREAM_SIZE_OFFSET) as usize;
        let eof0_flags = le_u32(bytes, eof0 + fmt::EOF_FLAGS_OFFSET);
        // The pickle occupies the first `stream_size` bytes; a trailing 32-byte
        // key SHA-256 (when flagged) follows it inside the same region.
        let pickle = region
            .get(..stream_size.min(region.len()))
            .unwrap_or(region);
        parse_response_info(pickle, &mut entry);
        if fmt::has_key_sha256(eof0_flags) {
            if let Some(sha) = region.get(stream_size..stream_size + 32) {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(sha);
                entry.key_sha256 = Some(arr);
            }
        }
    } // cov:unreachable: find_eofs pushes an offset then advances `i += EOF_LEN` and never rewinds, so eofs[1] >= eofs[0] + EOF_LEN == region_start — this else arm cannot be taken.

    Ok(entry)
}

/// Scan for `FINAL_MAGIC` (little-endian) at every offset from `start`, returning
/// the offsets in order. Each is the start of a `SimpleFileEOF` trailer.
fn find_eofs(bytes: &[u8], start: usize) -> Vec<usize> {
    let mut offs = Vec::new();
    let magic = fmt::FINAL_MAGIC.to_le_bytes();
    let mut i = start;
    // Only positions where a full 24-byte trailer fits are valid.
    while i + fmt::EOF_LEN <= bytes.len() {
        if bytes[i..i + 8] == magic {
            offs.push(i);
            i += fmt::EOF_LEN; // trailers do not overlap
        } else {
            i += 1;
        }
    }
    offs
}

/// Carve the request URL from a cache key. Chrome prefixes the key with a
/// Network-Isolation-Key (`1/0/_dk_<top> <frame> <url>`); the URL is the last
/// whitespace-separated token. A bare-URL key returns itself.
fn carve_url(key: &str) -> String {
    key.rsplit(|c: char| c.is_whitespace())
        .next()
        .unwrap_or(key)
        .to_owned()
}

/// Parse the stream-0 `HttpResponseInfo` pickle into status line, headers, and
/// request/response times. Best-effort and panic-free: on any shortfall it fills
/// what it can and leaves the rest `None`/empty.
///
/// Pickle layout (`base::Pickle`, little-endian, verified on real entries):
/// `[payload_size u32][flags u32][int u32][request_time i64][response_time i64]…`
/// followed by the null-separated raw header block introduced by `HTTP/`.
fn parse_response_info(pickle: &[u8], entry: &mut CacheEntry) {
    // request_time / response_time sit at fixed pickle offsets 12 and 20.
    let rt = le_u64(pickle, 12);
    if rt != 0 {
        entry.request_time_webkit_micros = Some(rt);
    }
    let st = le_u64(pickle, 20);
    if st != 0 {
        entry.response_time_webkit_micros = Some(st);
    }

    // The raw header block is a run of NUL-separated ASCII strings beginning with
    // "HTTP/". Locating it by content is robust against pickle field drift.
    let Some(rel) = find_subslice(pickle, b"HTTP/") else {
        return;
    };
    let block = &pickle[rel..];
    let mut parts = block.split(|&b| b == 0);
    if let Some(status) = parts.next() {
        entry.status_line = Some(String::from_utf8_lossy(status).into_owned());
    }
    for part in parts {
        if part.is_empty() {
            break; // the block is terminated by an empty string
        }
        let line = String::from_utf8_lossy(part);
        if let Some((name, value)) = line.split_once(':') {
            entry
                .headers
                .push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }
}

/// First offset of `needle` in `haystack`, or `None`. Panic-free.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}
