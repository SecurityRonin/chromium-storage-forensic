# Privacy Policy

*Last updated: 2026-07-10*

## Summary

chromium-storage-forensic is a local Rust library and CLI. It does not collect, transmit, or store any personal data on remote servers.

## Data Access

chromium-storage-forensic reads only the LevelDB files you point it at. All processing happens in memory on your local machine. Nothing is uploaded anywhere. It opens evidence read-only, never takes the database `LOCK`, and never writes to the directory.

## Telemetry

chromium-storage-forensic has **no telemetry**. It makes no network requests of any kind.

## Open Source

chromium-storage-forensic is open source (Apache-2.0). You can audit every line of code at [github.com/SecurityRonin/chromium-storage-forensic](https://github.com/SecurityRonin/chromium-storage-forensic).

## Contact

Privacy questions: [security@securityronin.com](mailto:security@securityronin.com)

---

[Terms of Service](terms.md) · [Home](index.md) · © 2026 Security Ronin Ltd.
