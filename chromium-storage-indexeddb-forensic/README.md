# chromium-storage-indexeddb-forensic

Forensic analyzer over the Chromium IndexedDB reader. Emits graded `forensicnomicon::report` observations — undecodable Blink/V8 values, records orphaned from their database/object-store metadata — each an observation (“consistent with”), never a conclusion. Re-exports the reader surface.

Part of the [`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic) workspace. Every parser in this workspace is a reader (`-core`) plus an analyzer (`-forensic`), per fleet ADR-0008.
