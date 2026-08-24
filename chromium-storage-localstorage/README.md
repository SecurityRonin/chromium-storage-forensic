# chromium-storage-localstorage

Backward-compatible facade re-exporting `chromium-storage-localstorage-core` (the reader). New consumers should depend on `chromium-storage-localstorage-core` directly; the analyzer is `chromium-storage-localstorage-forensic`.

Part of the [`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic) workspace. Every parser in this workspace is a reader (`-core`) plus an analyzer (`-forensic`), per fleet ADR-0008.
