# chromium-storage-forensic

[![Crates.io: cache](https://img.shields.io/crates/v/chromium-storage-cache.svg?label=chromium-storage-cache)](https://crates.io/crates/chromium-storage-cache)
[![Crates.io: indexeddb](https://img.shields.io/crates/v/chromium-storage-indexeddb.svg?label=chromium-storage-indexeddb)](https://crates.io/crates/chromium-storage-indexeddb)
[![Crates.io: localstorage](https://img.shields.io/crates/v/chromium-storage-localstorage.svg?label=chromium-storage-localstorage)](https://crates.io/crates/chromium-storage-localstorage)
[![Docs.rs](https://img.shields.io/docsrs/chromium-storage-cache?label=docs.rs)](https://docs.rs/chromium-storage-cache)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-blue.svg)](https://www.rust-lang.org)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Sponsor](https://img.shields.io/badge/sponsor-h4x0r-ea4aaa)](https://github.com/sponsors/h4x0r)

[![CI](https://github.com/SecurityRonin/chromium-storage-forensic/actions/workflows/ci.yml/badge.svg)](https://github.com/SecurityRonin/chromium-storage-forensic/actions/workflows/ci.yml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![fuzzed](https://img.shields.io/badge/fuzzed-libFuzzer-orange.svg)](fuzz/)
[![Security audit](https://img.shields.io/badge/security-cargo--deny-success.svg)](deny.toml)

**Read every trace a Chromium profile left in its three LevelDB/Simple-Cache storage backends — the cached pages an origin served, the JavaScript objects a site stored, and the DOM key/values it kept — straight from the on-disk bytes, no browser required.**

Three focused, panic-free readers for the storage an examiner reaches for after `History` and `Cookies`:

| Crate | Reads | Emits |
|---|---|---|
| [`chromium-storage-cache`](chromium-storage-cache/) | `Cache/Cache_Data/<hash>_0` Simple Cache entry files | URL, HTTP status + headers, body, request/response times, key SHA-256 |
| [`chromium-storage-indexeddb`](chromium-storage-indexeddb/) | `IndexedDB/<origin>.indexeddb.leveldb/` | `(database, objectStore, key, decoded V8 value)` per record |
| [`chromium-storage-localstorage`](chromium-storage-localstorage/) | `Local Storage/leveldb/` | per-origin `META` (WebKit-µs timestamp) + type-prefixed key/value pairs |

## Above the fold

```rust
// Decode a site's localStorage — origins, keys, values, and tombstones.
for rec in chromium_storage_localstorage::read_dir("Local Storage/leveldb".as_ref())? {
    println!("{rec:?}");
}

// Decode an IndexedDB store into (database, objectStore, key, value) records.
for rec in chromium_storage_indexeddb::read_dir("IndexedDB/site.indexeddb.leveldb".as_ref())? {
    println!("{} / {} / {:?} = {:?}",
        rec.database.unwrap_or_default(), rec.object_store.unwrap_or_default(),
        rec.key, rec.value);
}

// Decode one Simple Cache entry file: the URL it cached and the response.
let e = chromium_storage_cache::parse_entry(&std::fs::read("abc123_0")?)?;
println!("{} -> {:?}\n{} bytes body", e.url, e.status_line, e.body.len());
```

## How it fits the fleet

Each reader walks raw bytes, so it decodes deleted/superseded records a live
`Get()` would hide. The LevelDB-backed readers sit on
[`leveldb-core`](../leveldb-forensic) (which surfaces tombstones + sequence
numbers); IndexedDB values decode through [`blob-decoder`](../../utility/blob-decoder)'s
V8/Blink structured-clone deserializer; all format constants (Simple Cache
magics, IndexedDB key-coding, Local Storage schema) come from the KNOWLEDGE leaf
[`forensicnomicon-core`](../../knowledge/forensicnomicon).

## Trust, but verify

- **Fuzzed.** One libFuzzer target per reader (`parse_cache`, `decode_indexeddb`,
  `decode_localstorage`) drives its untrusted-byte entry point; each runs clean
  over millions of executions. See [`fuzz/`](fuzz/).
- **Panic-free by lint.** `unsafe_code = forbid`; `unwrap_used`/`expect_used`
  denied in production; every integer field read goes through the bounds-checked
  [`safe-read`](../../utility/safe-read) front door.
- **Validated against real Chrome output.** Correctness is checked against
  storage minted by driving headless Google Chrome and reading back the known
  writes — not only synthetic fixtures. See [`docs/validation.md`](docs/validation.md).

[Privacy Policy](https://securityronin.github.io/chromium-storage-forensic/privacy/) · [Terms of Service](https://securityronin.github.io/chromium-storage-forensic/terms/) · © 2026 Security Ronin Ltd
