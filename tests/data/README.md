# Test data provenance

All fixtures here were **minted on this host** by driving headless Google Chrome,
then copying the profile's storage directories. They are real Chromium output;
the known writes are the ground-truth oracle (tier-2). See
[`docs/validation.md`](../../docs/validation.md) and the fleet catalog
`ronin-issen/docs/test-data-catalog.md`.

Classification: **REAL-self** (real engine output, self-driven scenario).

## Mint procedure (verbatim)

A local HTTP server served an `index.html` that wrote the known storage, plus a
cacheable `cached.txt`. Chrome ran in two phases against a throwaway profile:

```bash
# Phase 1 — localStorage + IndexedDB + a subresource fetch:
"Google Chrome" --headless=new --disable-gpu --no-first-run \
  --user-data-dir=$PROFILE --disable-background-networking \
  --disable-component-update --disable-sync --disable-default-apps \
  --disable-domain-reliability --metrics-recording-only \
  "http://127.0.0.1:8731/index.html"    # ~8 s, then SIGTERM (flush LevelDB)

# Phase 2 — top-level navigation to the cacheable resource (HTTP-cache write):
"Google Chrome" --headless=new ... "http://127.0.0.1:8731/cached.txt"  # ~6 s, SIGTERM
```

`index.html` wrote (these are the oracle):

- **localStorage:** `greeting = "hello"`, `mint_ls_key = "mint_ls_value_δ"`
  (the δ / U+03B4 forces the UTF-16LE value path).
- **IndexedDB:** database `mintdb`, object store `notes`,
  `put({title:'first note', n:42, tags:['a','b']}, 'note-1')`.
- **cached.txt:** body `CACHED-CONTENT-MARKER-9427\n`, served with
  `Cache-Control: public, max-age=999999`.

## Files

#### local-storage/leveldb/

- Source: minted (see above); copied from `<profile>/Default/Local Storage/leveldb`.
- Contents: `000003.log`, `CURRENT`, `MANIFEST-000001` (the `LOCK`/`LOG` files are
  excluded — they are not needed to read the store).
- Consumed by: `chromium-storage-localstorage/tests/decode.rs`.

#### indexeddb/http_127.0.0.1_8731.indexeddb.leveldb/

- Source: minted; copied from `<profile>/Default/IndexedDB/http_127.0.0.1_8731.indexeddb.leveldb`.
- Contents: `000003.log`, `CURRENT`, `MANIFEST-000001`.
- Consumed by: `chromium-storage-indexeddb/tests/decode.rs`.

#### simple-cache/ea2e47cbdc22305e_0

- Source: minted; the `cached.txt` Simple Cache entry from `<profile>/Default/Cache/Cache_Data`.
- Contents: key `1/0/_dk_http://127.0.0.1 … http://127.0.0.1:8731/cached.txt`,
  body `CACHED-CONTENT-MARKER-9427\n`, `HTTP/1.0 200 OK` + headers, request/response
  times, key SHA-256.
- Consumed by: `chromium-storage-cache/tests/entry.rs`.

#### simple-cache/d9c2c72a2ec24e84_0

- Source: minted; a second real entry (a `www.google.com/async/…` request Chrome
  cached), used to confirm the entry layout generalises beyond the local resource.
- Consumed by: `chromium-storage-cache/tests/entry.rs` (`read_dir` returns both).
