# chromium-storage-cache

Backward-compatible facade re-exporting `chromium-storage-cache-core` (the reader). New consumers should depend on `chromium-storage-cache-core` directly; the analyzer is `chromium-storage-cache-forensic`.

Part of the [`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic) workspace. Every parser in this workspace is a reader (`-core`) plus an analyzer (`-forensic`), per fleet ADR-0008.
