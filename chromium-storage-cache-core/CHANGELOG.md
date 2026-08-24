# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and the project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1](https://github.com/SecurityRonin/chromium-storage-forensic/releases/tag/chromium-storage-cache-v0.1.1) - 2026-08-04

### Changed

- Declared MSRV corrected to 1.87 (the previous per-crate `rust-version` values
  were wrong; no code change).

### Notes

- Maintenance release with **no API change**. Everything since 0.1.0 is CI,
  tests, docs and MSRV metadata. It is cut so the release tooling's comparison
  baseline moves past the repository's early history, which declared fleet
  dependencies by relative path and therefore cannot be packaged from a lone
  clone.

## [0.1.0](https://github.com/SecurityRonin/chromium-storage-forensic/releases/tag/chromium-storage-cache-v0.1.0) - 2026-07-29

### Added

- *(cache)* GREEN — decode Simple Cache _0 entries
