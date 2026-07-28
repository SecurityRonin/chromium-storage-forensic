//! Fuzz the Simple Cache entry parser on arbitrary bytes.
//!
//! Exercises the header/magic/key-length validation, the EOF-trailer scan, the
//! body/stream-0 delimiting, and the `HttpResponseInfo` pickle + header block
//! parse. Must never panic on hostile bytes (lying key_length, missing trailers,
//! oversized stream_size, truncated pickle).
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let _ = chromium_storage_cache::parse_entry(bytes);
});
