# chromium-storage-indexeddb

Backward-compatible facade re-exporting `chromium-storage-indexeddb-core` (the reader). New consumers should depend on `chromium-storage-indexeddb-core` directly; the analyzer is `chromium-storage-indexeddb-forensic`.

Part of the [`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic) workspace. Every parser in this workspace is a reader (`-core`) plus an analyzer (`-forensic`), per fleet ADR-0008.
