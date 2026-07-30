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

- Source: minted; a second real entry Chrome cached on its own during the mint —
  `https://www.google.com/async/folae?async=_fmt:pb&udm=50&client_locale=en-US&client_country=hk`,
  `content-type: application/x-protobuffer`. Used to confirm the entry layout
  generalises beyond the local resource.
- Classification note: this one entry is **REAL-ext**, not REAL-self — the bytes are a
  genuine third-party (Google) HTTP response, not a loopback mint. No personal data,
  but it is not ours to license; do not redistribute it as our own fixture.
- Consumed by: `chromium-storage-cache/tests/entry.rs` (`read_dir` returns both).

## MD5 manifest

`tests/data/` is committed for these small fixtures, so hashes live beside the files
here as well (`md5`, 2026-07-30). The fleet index carries the same rows in
`ronin-issen/docs/test-data-catalog.md` §H.

| File | Bytes | MD5 |
|---|---|---|
| `local-storage/leveldb/000003.log` | 243 | `c615e82a28579922d3d0caa1fdc176d1` |
| `local-storage/leveldb/CURRENT` | 16 | `46295cac801e5d4857d09837238a6394` |
| `local-storage/leveldb/MANIFEST-000001` | 41 | `5af87dfd673ba2115e2fcf5cfdb727ab` |
| `indexeddb/http_127.0.0.1_8731.indexeddb.leveldb/000003.log` | 1360 | `dd48c7a058efaa2e81490f4629dd6b01` |
| `indexeddb/http_127.0.0.1_8731.indexeddb.leveldb/CURRENT` | 16 | `46295cac801e5d4857d09837238a6394` |
| `indexeddb/http_127.0.0.1_8731.indexeddb.leveldb/MANIFEST-000001` | 23 | `3fd11ff447c1ee23538dc4d9724427a3` |
| `simple-cache/ea2e47cbdc22305e_0` | 449 | `58a8001d87b2d970be5e5d4293896f4e` |
| `simple-cache/d9c2c72a2ec24e84_0` | 4853 | `34c8262922bc9d8ddd1f5a7d9ef3ff0c` |

Re-minting produces different bytes (LevelDB sequence numbers, cache entry file
names), so replace these hashes if you re-mint. The `index.html` / `cached.txt` served
on `127.0.0.1:8731` are **not committed** — the writes listed above are the record.
