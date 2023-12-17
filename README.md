# axum-embed

[![Crates.io](https://img.shields.io/crates/v/axum-embed)](https://crates.io/crates/axum-embed)
[![Crates.io](https://img.shields.io/crates/l/axum-embed)](https://crates.io/crates/axum-embed)
[![Build](https://github.com/informationsea/axum-embed/actions/workflows/build.yaml/badge.svg)](https://github.com/informationsea/axum-embed/actions/workflows/build.yaml)
[![docs.rs](https://img.shields.io/docsrs/axum-embed)](https://docs.rs/axum-embed/)
![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/informationsea/axum-embed)


`axum-embed` is a library that provides a service for serving embedded files using the `axum` web framework.

This library uses the `rust_embed` crate to embedded files into the binary at compile time, and the `axum` crate to serve these files over HTTP.

# Features
- Serve embedded files over HTTP
- Customizable 404, fallback, and index files
- Response compressed files if the client supports it and the compressed file exists
- Response 304 if the client has the same file (based on ETag)
- Redirect to the directory if the client requests a directory without a trailing slash

# Example
```rust
use rust_embed::RustEmbed;
use axum_embed::ServeEmbed;
use tokio::net::TcpListener;

#[derive(RustEmbed, Clone)]
#[folder = "examples/assets/"]
struct Assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let serve_assets = ServeEmbed::<Assets>::new();
    let app = axum::Router::new().nest_service("/", serve_assets);
    axum::serve(listener, app).await?;
    Ok(())
}
```

# Usage

Please see the [examples](https://github.com/informationsea/axum-embed/tree/main/examples) directory for a working example.

## Serve compressed file

The `axum_embed` library has the capability to serve compressed files, given that the client supports it and the compressed file is available.
The compression methods supported include `br` (Brotli), `gzip`, and `deflate`.
If the client supports multiple compression methods, `axum_embed` will select the first one listed in the `Accept-Encoding` header. Please note that the weight of encoding is not considered in this ction.
In the absence of client support for any compression methods, `axum_embed` will serve the file in its uncompressed form.
If a file with the extension `.br` (for Brotli), `.gz` (for GZip), or `.zz` (for Deflate) is available, `axum_embed` will serve the file in its compressed form.
An uncompressed file is must be available for the compressed file to be served.