//! Chromium Simple Cache forensic analyzer, and the reader it grades over.
//!
//! Emits [`forensicnomicon::report`] observations over a [`CacheEntry`] parsed
//! by the [`chromium_storage_cache_core`] reader. Every finding is
//! **self-validating** — it recomputes a value the format itself stored (a CRC,
//! a hash) or compares two timestamps the format itself recorded — so it needs
//! no external oracle, and each is an *observation* ("consistent with"), never a
//! conclusion.
//!
//! The reader surface is re-exported, so `chromium_storage_cache_forensic::`
//! resolves the reader types too.
#![forbid(unsafe_code)]

pub use chromium_storage_cache_core::*;

use forensicnomicon::report::{Category, Observation, Severity};
use sha2::{Digest, Sha256};

/// A graded anomaly observed in a Simple Cache entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheAnomalyKind {
    /// The stream-1 body trailer recorded a CRC-32 that disagrees with the
    /// CRC-32 recomputed over the stored body. Consistent with the body having
    /// been edited without recomputing its trailer, or with corruption.
    BodyCrc32Mismatch { stored: u32, computed: u32 },
    /// The stream-0 trailer recorded a SHA-256 of the key that disagrees with
    /// the SHA-256 recomputed over the stored key bytes. Consistent with the
    /// key having been edited after the entry was written, or with corruption.
    KeySha256Mismatch,
    /// The response time precedes the request time — impossible for a real
    /// fetch. Consistent with an edited timestamp.
    ResponseBeforeRequest {
        request_micros: u64,
        response_micros: u64,
    },
}

impl CacheAnomalyKind {
    /// Severity — the single source of truth.
    #[must_use]
    pub fn severity(&self) -> Severity {
        match self {
            // A broken integrity value is a direct violation of a stored invariant.
            CacheAnomalyKind::BodyCrc32Mismatch { .. } | CacheAnomalyKind::KeySha256Mismatch => {
                Severity::High
            }
            // A timestamp contradiction is medium — often an artifact, not always tamper.
            CacheAnomalyKind::ResponseBeforeRequest { .. } => Severity::Medium,
        }
    }

    /// Analytical lens.
    #[must_use]
    pub fn category(&self) -> Category {
        match self {
            CacheAnomalyKind::BodyCrc32Mismatch { .. } | CacheAnomalyKind::KeySha256Mismatch => {
                Category::Integrity
            }
            CacheAnomalyKind::ResponseBeforeRequest { .. } => Category::History,
        }
    }

    /// Stable machine-readable code (published contract; never reused/renamed).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            CacheAnomalyKind::BodyCrc32Mismatch { .. } => "CACHE-BODY-CRC-MISMATCH",
            CacheAnomalyKind::KeySha256Mismatch => "CACHE-KEY-SHA256-MISMATCH",
            CacheAnomalyKind::ResponseBeforeRequest { .. } => "CACHE-TIME-RESPONSE-BEFORE-REQUEST",
        }
    }

    /// Human-readable note (observation, not a conclusion).
    #[must_use]
    pub fn note(&self) -> String {
        match self {
            CacheAnomalyKind::BodyCrc32Mismatch { stored, computed } => format!(
                "response body CRC-32 mismatch: trailer recorded {stored:#010x}, recomputed \
                 {computed:#010x} — consistent with the body having been edited after storage"
            ),
            CacheAnomalyKind::KeySha256Mismatch => {
                "key SHA-256 mismatch: the stream-0 trailer's key hash disagrees with the SHA-256 \
                 recomputed over the stored key — consistent with the key having been edited"
                    .to_owned()
            }
            CacheAnomalyKind::ResponseBeforeRequest {
                request_micros,
                response_micros,
            } => format!(
                "response time ({response_micros} µs) precedes request time ({request_micros} µs) \
                 — impossible for a real fetch, consistent with an edited timestamp"
            ),
        }
    }
}

/// A graded finding: an [`CacheAnomalyKind`] plus its derived severity/code/note.
#[derive(Debug, Clone)]
pub struct CacheAnomaly {
    pub kind: CacheAnomalyKind,
    severity: Severity,
    code: &'static str,
    note: String,
}

impl CacheAnomaly {
    #[must_use]
    pub fn new(kind: CacheAnomalyKind) -> Self {
        let severity = kind.severity();
        let code = kind.code();
        let note = kind.note();
        Self {
            kind,
            severity,
            code,
            note,
        }
    }
}

impl Observation for CacheAnomaly {
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }
    fn code(&self) -> &'static str {
        self.code
    }
    fn note(&self) -> String {
        self.note.clone()
    }
    fn category(&self) -> Category {
        self.kind.category()
    }
}

/// Grade a single parsed Simple Cache entry, returning every anomaly it exhibits.
///
/// Clean entries return an empty vector — a true negative, since every check
/// recomputes a value the format stored.
#[must_use]
pub fn analyze(entry: &CacheEntry) -> Vec<CacheAnomaly> {
    let mut out = Vec::new();

    // Body integrity: the stream-1 trailer's CRC-32 over the body.
    if let Some(stored) = entry.body_crc32 {
        let mut h = crc32fast::Hasher::new();
        h.update(&entry.body);
        let computed = h.finalize();
        if computed != stored {
            out.push(CacheAnomaly::new(CacheAnomalyKind::BodyCrc32Mismatch {
                stored,
                computed,
            }));
        }
    }

    // Key integrity: the stream-0 trailer's SHA-256 over the key.
    if let Some(stored) = entry.key_sha256 {
        let computed: [u8; 32] = Sha256::digest(entry.key.as_bytes()).into();
        if computed != stored {
            out.push(CacheAnomaly::new(CacheAnomalyKind::KeySha256Mismatch));
        }
    }

    // Timeline: response cannot precede request.
    if let (Some(req), Some(resp)) = (
        entry.request_time_webkit_micros,
        entry.response_time_webkit_micros,
    ) {
        if resp < req {
            out.push(CacheAnomaly::new(CacheAnomalyKind::ResponseBeforeRequest {
                request_micros: req,
                response_micros: resp,
            }));
        }
    }

    out
}
