# Validation

How the three readers are checked for correctness, and at what evidence tier.

## Summary

Every reader is validated against **real Chromium output**, not only synthetic
fixtures. The test corpus was minted by driving headless Google Chrome
(v-headless-new) against a local page + cacheable resource, then copying the
profile's `Local Storage/leveldb`, `IndexedDB/*.indexeddb.leveldb`, and
`Cache/Cache_Data` directories. Because the page's writes are known, those writes
are the ground-truth oracle — a **tier-2** check (real engine output; ground
truth derivable from the documented construction).

## Evidence tiers

| Reader | Oracle | Tier |
|---|---|---|
| `chromium-storage-localstorage` | Real Chrome wrote `greeting=hello` and `mint_ls_key=mint_ls_value_δ` (δ forces the UTF-16 path) + a per-origin `META` record; the reader recovers all of them. | **T2** |
| `chromium-storage-indexeddb` | Real Chrome stored `put({title:'first note', n:42, tags:['a','b']}, 'note-1')` in `mintdb`/`notes`; the reader recovers the database + object-store names, the `note-1` key, and the decoded V8 object. | **T2** |
| `chromium-storage-cache` | Real Chrome cached a local resource with a known URL, body (`CACHED-CONTENT-MARKER-9427`), and HTTP headers; the reader recovers the URL, status line, headers, body, and request/response times. A second real entry (google.com telemetry) confirms the layout generalises. | **T2** |

Each reader additionally carries **tier-3** decoder unit tests (hand-built
records for each key/value shape, tombstones, unknown markers) as fast
regression scaffolding *under* the T2 oracle, and robustness tests asserting that
malformed input never panics.

## Fuzzing (present-robustness evidence)

One libFuzzer target per reader drives its untrusted-byte entry point:

| Target | Entry point |
|---|---|
| `parse_cache` | `chromium_storage_cache::parse_entry` |
| `decode_indexeddb` | `chromium_storage_indexeddb::decode_records` |
| `decode_localstorage` | `chromium_storage_localstorage::decode_records` |

Each ran clean (no panic, no OOM) over millions of executions during
development. `ci.yml` compiles every target (`cargo fuzz check`); `fuzz.yml` runs
a bounded weekly campaign.

## Reproducing the corpus

The exact mint procedure (server + Chrome flags + the two-phase run) is recorded
in [`tests/data/README.md`](https://github.com/SecurityRonin/chromium-storage-forensic/blob/main/tests/data/README.md).
The committed LevelDB directories and `_0` entry files are small and checked in;
re-minting reproduces equivalent artifacts (byte layout is stable; hashes and
timestamps differ per run).

## Open scaffolding

Two fleet-standard gates are scaffolded but **not yet enabled in CI**, pending a
run that regenerates their machine state:

- **cargo-vet** — `supply-chain/config.toml` declares the imports + our
  first-party crates; the per-version third-party `[[exemptions]]` block must be
  generated with `cargo vet` against the resolved graph before the vet job is
  turned on.
- **Coverage gate** — the 100 %-line `cargo llvm-cov` gate is not yet wired; the
  readers are exercised end-to-end by the T2 tests, but the ratchet job should be
  added once a baseline run confirms the number.
