//! Decoded IndexedDB record value.

use blob_decoder::v8_value::V8Value;

/// The value stored against an IndexedDB object-store record. On disk it is a
/// wrapper-version varint followed by a Blink `SerializedScriptValue`; a clean
/// decode yields [`RecordValue::V8`], and any failure surfaces the raw bytes plus
/// the decode error rather than dropping the value.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordValue {
    /// A decoded V8 structured-clone value.
    V8(V8Value),
    /// The value could not be deserialized; raw bytes + error retained.
    Undecoded {
        /// The value bytes after the wrapper-version varint (verbatim).
        raw: Vec<u8>,
        /// A human-readable description of the decode failure.
        error: String,
    },
}
