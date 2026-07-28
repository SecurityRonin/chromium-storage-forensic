# chromium-storage-forensic

Three panic-free Rust readers for the Chromium storage backends an examiner
reaches for after `History` and `Cookies`:

- **[`chromium-storage-cache`](https://crates.io/crates/chromium-storage-cache)** —
  Simple Cache `<hash>_0` entry files → URL, HTTP status + headers, body,
  request/response times, key SHA-256.
- **[`chromium-storage-indexeddb`](https://crates.io/crates/chromium-storage-indexeddb)** —
  IndexedDB-over-LevelDB → `(database, objectStore, key, decoded V8 value)`.
- **[`chromium-storage-localstorage`](https://crates.io/crates/chromium-storage-localstorage)** —
  Local Storage LevelDB → per-origin `META` timestamps + type-prefixed key/values.

Each reader walks raw bytes (surfacing deleted/superseded records), forbids
`unsafe`, denies `unwrap`/`expect`, routes every integer read through
`safe-read`, and carries a libFuzzer target. Format constants come from the
KNOWLEDGE leaf `forensicnomicon-core`; IndexedDB values decode through
`blob-decoder`'s V8/Blink deserializer; LevelDB records come from `leveldb-core`.

See [Validation](validation.md) for the tier-2 real-Chrome oracle, and the
[Product Requirements](PRD.md) for scope.
