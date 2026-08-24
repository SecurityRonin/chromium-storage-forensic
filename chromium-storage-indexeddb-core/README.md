# chromium-storage-indexeddb

Panic-free reader for **Chromium IndexedDB** stores
(`IndexedDB/<origin>.indexeddb.leveldb/`).

Walks every raw LevelDB record (surfacing tombstones + superseded versions),
decodes the `KeyPrefix` `(database id, object store id, index id)`, resolves the
database and object-store *names* from their metadata records, decodes each
object-store data record's `IDBKey`, and deserializes the value's Blink-wrapped
V8 structured-clone blob. It emits one record per object-store data entry:
`(database, objectStore, key, decoded value)`.

```rust
for r in chromium_storage_indexeddb::read_dir("site.indexeddb.leveldb".as_ref())? {
    println!("{} / {} / {:?} = {:?}",
        r.database.unwrap_or_default(),
        r.object_store.unwrap_or_default(),
        r.key, r.value);
}
# Ok::<(), leveldb_core::Error>(())
```

Key-coding constants come from `forensicnomicon-core::chromium_indexeddb`;
LevelDB reading from `leveldb-core`; V8 value decoding from `blob-decoder`. An
undecodable value surfaces its raw bytes + the decode error rather than being
dropped. Part of the
[`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic)
suite.

[Privacy Policy](https://securityronin.github.io/chromium-storage-forensic/privacy/) · [Terms of Service](https://securityronin.github.io/chromium-storage-forensic/terms/) · © 2026 Security Ronin Ltd
