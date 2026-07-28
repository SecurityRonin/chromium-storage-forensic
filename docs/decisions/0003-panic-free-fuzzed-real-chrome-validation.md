# 0003 — Panic-free, fuzzed, validated against real Chrome output

*Status: accepted*

## Context

These readers parse attacker-controllable bytes (a cache file, a LevelDB record
value) recovered from an evidence image. A panic or out-of-bounds read on a
crafted input is a denial-of-service or worse; a subtly-wrong decode fabricates
evidence.

## Decision

- **Panic-free by construction.** `unsafe_code = forbid`; `unwrap_used` /
  `expect_used` denied in production; every fixed-width integer field read routes
  through `safe-read` (returns 0/`None` out of range); every length/offset from
  the data is range-checked before use. A truncated Simple Cache entry still
  yields its URL rather than erroring the whole parse.
- **Fuzzed per reader.** One libFuzzer target per untrusted-byte entry point
  (`parse_cache`, `decode_indexeddb`, `decode_localstorage`); `ci.yml` compiles
  them, `fuzz.yml` runs a weekly campaign.
- **Validated against real Chrome, not just synthetic fixtures.** The test corpus
  is minted by driving headless Google Chrome and reading back known writes — a
  tier-2 oracle. Decoder unit tests (tier-3) sit *under* that as regression
  scaffolding. See [Validation](../validation.md).

## Consequences

- The IDBKey/pickle/protobuf parsers are written defensively (checked_add,
  `.get(..)`, no blind indexing), which the fuzz campaign continuously exercises.
- The unrated/robustness tests assert malformed input returns an error or empty
  result, never a panic.
