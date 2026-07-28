# chromium-storage-forensic — Purpose & Scope

*Library-tier suite (readers linked by consumers, no shipped binary), so this is
a concise Purpose & Scope rather than a full product-requirements doc (per the
fleet ADR-0003 filename convention). Load-bearing decisions are recorded as ADRs
under [`docs/decisions/`](decisions/).*

## What it is

A Pattern-B PARSER suite (`chromium-storage-*`) of three readers for Chromium's
per-origin storage backends, built for DFIR use where the browser is unavailable
and the evidence is an on-disk profile (or a carved copy of one).

## Who links it

- Orchestration/correlation layers (Issen and disk4n6-style tools) that fold
  browser-storage artifacts into a unified timeline.
- Analysts and tool authors who need `(url/key → value)` records out of a
  Chromium profile without a running browser.

## Scope

- **In:** decode Simple Cache entry files; walk IndexedDB-over-LevelDB and emit
  `(database, objectStore, key, decoded value)`; decode Local Storage records
  (META + type-prefixed key/values). Surface deleted/superseded records.
- **Out (non-goals):** carving free pages / recovering the LevelDB from raw
  unallocated space (that is a `-carve` concern); the blockfile (non-Simple)
  cache backend; writing or repairing stores (read-only); a CLI/GUI front-end
  (this suite is linked, not run — a front-end would be a separate `-cli` crate).

## Success criteria

- Correctly recovers the known writes from real minted Chrome storage
  (tier-2 oracle — see [Validation](validation.md)).
- Never panics on attacker-controlled input (fuzzed per reader; panic-free
  lints; bounds-checked reads).
- Format knowledge lives in `forensicnomicon-core`; value decoding reuses
  `blob-decoder`; LevelDB reading reuses `leveldb-core` — no reinvention.
