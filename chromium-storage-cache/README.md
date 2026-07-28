# chromium-storage-cache

Panic-free reader for **Chromium Simple Cache** entry files (`Cache/Cache_Data/<hash>_0`).

Decodes one `_0` entry into the cache key and request URL carved from it, the
HTTP response status line + headers (from the stream-0 `HttpResponseInfo`
pickle), the response body (stream 1), the request/response WebKit-µs times, and
the trailer CRCs + key SHA-256.

```rust
let e = chromium_storage_cache::parse_entry(&std::fs::read("abc123_0")?)?;
println!("{} -> {:?}", e.url, e.status_line);
for (name, value) in &e.headers {
    println!("  {name}: {value}");
}
println!("{} bytes of body", e.body.len());
# Ok::<(), chromium_storage_cache::CacheError>(())
```

Format constants come from `forensicnomicon-core::chromium_simple_cache`; every
integer read is bounds-checked via `safe-read`. A valid header is required; body,
headers, and times degrade to empty/`None` on truncation so a damaged entry still
yields its URL. Part of the
[`chromium-storage-forensic`](https://github.com/SecurityRonin/chromium-storage-forensic)
suite.

[Privacy Policy](https://securityronin.github.io/chromium-storage-forensic/privacy/) · [Terms of Service](https://securityronin.github.io/chromium-storage-forensic/terms/) · © 2026 Security Ronin Ltd
