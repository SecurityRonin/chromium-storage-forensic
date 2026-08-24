# chromium-storage-localstorage-forensic

Forensic analyzer over the Chromium Local Storage reader. Emits graded `forensicnomicon::report` observations — implausibly early `META` timestamps, `DATA` rows orphaned from their origin metadata — each an observation (“consistent with”), never a conclusion. Re-exports the reader surface.

Part of the [`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic) workspace. Every parser in this workspace is a reader (`-core`) plus an analyzer (`-forensic`), per fleet ADR-0008.
