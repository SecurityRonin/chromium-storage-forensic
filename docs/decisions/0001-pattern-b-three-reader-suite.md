# 0001 — Pattern-B suite of three `chromium-storage-*` readers

*Status: accepted*

## Context

Chromium persists web-storage in three unrelated on-disk formats: the Simple
Cache (one file per HTTP entry), IndexedDB (a LevelDB store with a bespoke
key-coding scheme wrapping V8-serialized values), and Local Storage (a different
LevelDB schema of type-prefixed strings). DLEAPP-style triage needs all three.

## Decision

Ship one repo, `chromium-storage-forensic`, as a **Pattern-B multi-crate PARSER
suite** with the prefix `chromium-storage-` and one reader crate per format:
`chromium-storage-cache`, `chromium-storage-indexeddb`,
`chromium-storage-localstorage`. The repo name is an umbrella, not a crate.

## Consequences

- The prefix is generic-word-adjacent, so it takes the full self-describing form
  `chromium-storage-*` (not a bare `chromium-*`), per the fleet crate-naming
  grammar.
- Each format's knowledge is isolated in its own crate; a consumer that only
  needs Local Storage does not pull `blob-decoder` (the V8 dep) transitively.
- No `-cli`/`-carve` crate: this suite is *linked*, not run, and free-page
  recovery is out of scope (see [PRD](../PRD.md)).
