# xcss::web_assets

Production resources are compiled into their Server executable. The asset manifest and its SHA-256 digest bind filenames, MIME types, sizes, and every embedded byte. No production disk directory is selected implicitly.

Declare the single `xcss` crate in both `[dependencies]` and `[build-dependencies]`. The whole crate requires Linux x86_64 GNU. In `build.rs`, after the frontend has been built:

```rust,ignore
fn main() -> anyhow::Result<()> {
    xcss::web_assets::build::generate("web/dist")
}
```

At runtime:

```rust,ignore
mod web {
    include!(concat!(env!("OUT_DIR"), "/xcss-web-assets.rs"));
}
let response = xcss::web_assets::response(web::ASSETS, uri.path(), &method, &headers);
// Axum: response.map(axum::body::Body::from)
// Hyper: response.map(http_body_util::Full::new)
```

The generated module exports `ASSETS`, `MANIFEST`, and `DIGEST`. `MANIFEST` is compact JSON with the exact shape `{ "format": "web-assets-v1", "files": [{ "path": "index.html", "content_type": "text/html; charset=utf-8", "size": 123, "sha256": "..." }] }`, with no surrounding whitespace and no trailing newline. Files are sorted by path. `DIGEST` is SHA-256 of those exact manifest bytes. `verify_manifest` validates canonical format and digest; `verify_embedded` also checks every embedded byte. Release tooling can capture the executable's manifest to verify served resources without shipping a duplicate Web directory.

`build::generate_to(root, filename)` supports separate generated modules. `build::generate_at(root, output, filename)` is available to build tools without changing `OUT_DIR`. Generation snapshots the input bytes into the output directory before `include_bytes!`, so an input change during compilation cannot silently invalidate the compiled inventory. All regular outputs are included, including licenses. Select intended inputs in the build's staging directory when a bundle contains non-runtime files.

For development, explicitly select `DirectoryAssets::new(absolute_root)` and call its `response(path, method, headers)` method. The provider rereads each resource on every request. On Unix, held directory descriptors and `openat` with `NOFOLLOW` prevent traversal through replaced directories or links. The portable provider checks every path component for symlinks; production support remains Linux AMD64 GNU. Generated inventories reject symlinks, hardlinks on Unix, special files, unsafe names, and excessive inventories or reads.

HTTP behavior is shared across both providers:

- Empty path and `/` select `index.html`; all other paths match exact assets. Products strip their mounting prefix before calling the handler.
- GET and HEAD are supported, with exact MIME types, `nosniff`, and `Referrer-Policy: same-origin`; other methods return 405.
- HTML uses `Cache-Control: no-store, max-age=0`. Other assets use `public, no-cache` and strong SHA-256 ETags; conditional GET/HEAD handles weak validators, validator lists, and `*`.
- Unknown or unsafe paths return 404. No automatic SPA fallback or disk fallback is used.
- Products retain control over their existing CSP and other application headers. Filenames alone do not prove content identity, so this library does not apply immutable caching based on a guessed filename convention.
