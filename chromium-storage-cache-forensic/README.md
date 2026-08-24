# chromium-storage-cache-forensic

Forensic analyzer over the Chromium Simple Cache reader. Emits graded `forensicnomicon::report` observations — body CRC-32 and key SHA-256 integrity mismatches (recomputed from the entry's own stored values), response-before-request timeline contradictions — each an observation (“consistent with”), never a conclusion. Re-exports the reader surface.

Part of the [`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic) workspace. Every parser in this workspace is a reader (`-core`) plus an analyzer (`-forensic`), per fleet ADR-0008.
