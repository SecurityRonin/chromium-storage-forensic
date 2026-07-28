# 0002 — Reuse fleet crates for constants, LevelDB, and V8 decoding

*Status: accepted*

## Context

All three formats share upstream concerns already solved elsewhere in the fleet:
format constants, LevelDB record reading, bounds-checked integer reads, and the
V8/Blink structured-clone deserialization IndexedDB values need.

## Decision

Depend down onto existing fleet crates rather than reinvent:

- **`forensicnomicon-core`** (KNOWLEDGE leaf) for every format constant — Simple
  Cache magics/offsets (`chromium_simple_cache`), the IndexedDB `KeyPrefix`
  coding (`chromium_indexeddb`), and the Local Storage schema
  (`chromium_local_storage`). No magic numbers are re-declared in the readers
  (the sole exception, `kDatabaseNameTypeByte = 201`, is cited inline pending its
  export from the KNOWLEDGE leaf).
- **`leveldb-core`** for raw LevelDB record enumeration (surfaces tombstones +
  sequence numbers) — the IndexedDB and Local Storage readers build on it.
- **`blob-decoder`** for the IndexedDB record value: its `v8_value::deserialize_blink`
  decodes the Blink `SerializedScriptValue` wrapper + V8 stream.
- **`safe-read`** as the single bounds-checked integer-read front door.

## Consequences

- Path deps today; switch to the published registry versions once each is on
  crates.io (fleet Dependency-Preference law).
- `blob-decoder` raises `chromium-storage-indexeddb`'s MSRV to 1.88; the crate
  takes the capability bump rather than gate the decoder out (batteries-included).
  The cache + localstorage readers keep the low 1.80 floor.
