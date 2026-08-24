# chromium-storage-localstorage

Panic-free reader for **Chromium Local Storage** (DOM Storage on LevelDB,
`Local Storage/leveldb/`).

Walks every raw LevelDB record (surfacing tombstones + superseded versions) and
classifies each into a per-origin `META` record (a `StorageMetadata` protobuf →
WebKit-µs last-modified timestamp + size), a data record (`_` + origin + NUL +
type-prefixed script key → type-prefixed value, UTF-16LE / Latin-1), or `Other`
(surfaced raw).

```rust
for rec in chromium_storage_localstorage::read_dir("Local Storage/leveldb".as_ref())? {
    println!("{rec:?}");
}
# Ok::<(), leveldb_core::Error>(())
```

Schema + encoding-marker constants come from
`forensicnomicon-core::chromium_local_storage`; LevelDB reading from
`leveldb-core`. Decoding never panics: a lossy transcode becomes U+FFFD and sets
a `lossy` flag, and the raw bytes are always retained. Part of the
[`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic)
suite.

[Privacy Policy](https://securityronin.github.io/chromium-storage-forensic/privacy/) · [Terms of Service](https://securityronin.github.io/chromium-storage-forensic/terms/) · © 2026 Security Ronin Ltd
