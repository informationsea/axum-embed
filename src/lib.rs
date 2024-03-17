//! [![Crates.io](https://img.shields.io/crates/v/axum-embed)](https://crates.io/crates/axum-embed)
//! [![Crates.io](https://img.shields.io/crates/l/axum-embed)](https://crates.io/crates/axum-embed)
//! [![Build](https://github.com/informationsea/axum-embed/actions/workflows/build.yaml/badge.svg)](https://github.com/informationsea/axum-embed/actions/workflows/build.yaml)
//! [![docs.rs](https://img.shields.io/docsrs/axum-embed)](https://docs.rs/axum-embed/)
//! ![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/informationsea/axum-embed)
//! `axum_embed` is a library that provides a service for serving embedded files using the `axum` web framework.
//!
//! This library uses the `rust_embed` crate to embedded files into the binary at compile time, and the `axum` crate to serve these files over HTTP.
//!
//! # Features
//! - Serve embedded files over HTTP
//! - Customizable 404, fallback, and index files
//! - Response compressed files if the client supports it and the compressed file exists
//! - Response 304 if the client has the same file (based on ETag)
//! - Redirect to the directory if the client requests a directory without a trailing slash
//!
//! # Example
//! ```ignore
//! # use rust_embed::RustEmbed;
//! # use axum_embed::ServeEmbed;
//! # use tokio::net::TcpListener;
//! #
//! #[derive(RustEmbed, Clone)]
//! #[folder = "examples/assets/"]
//! struct Assets;
//!
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let listener = TcpListener::bind("127.0.0.1:8080").await?;
//! let serve_assets = ServeEmbed::<Assets>::new();
//! let app = axum::Router::new().nest_service("/", serve_assets);
//! axum::serve(listener, app).await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Usage
//!
//! Please see the [examples](https://github.com/informationsea/axum-embed/tree/main/examples) directory for a working example.
//!
//! ## Serve compressed file
//!
//! The `axum_embed` library has the capability to serve compressed files, given that the client supports it and the compressed file is available.
//! The compression methods supported include `br` (Brotli), `gzip`, and `deflate`.
//! If the client supports multiple compression methods, `axum_embed` will select the first one listed in the `Accept-Encoding` header. Please note that the weight of encoding is not considered in this selection.
//! In the absence of client support for any compression methods, `axum_embed` will serve the file in its uncompressed form.
//! If a file with the extension `.br` (for Brotli), `.gz` (for GZip), or `.zz` (for Deflate) is available, `axum_embed` will serve the file in its compressed form.
//! An uncompressed file is must be available for the compressed file to be served.
use std::{borrow::Cow, convert::Infallible, future::Future, pin::Pin, sync::Arc, task::Poll};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use rust_embed::RustEmbed;
use tower_service::Service;

#[derive(Clone, RustEmbed)]
#[folder = "src/assets"]
struct DefaultFallback;

/// [`FallbackBehavior`] is an enumeration representing different behaviors that a server might take when a requested resource is not found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FallbackBehavior {
    /// The server responds the fallback resource with 404 status code when the resource was not found.
    NotFound,
    /// The server redirects the user to a different resource when the resource was not found.
    Redirect,
    /// The server responds the fallback resource with 200 status code when the resource was not found.
    Ok,
}

/// [`ServeEmbed`] is a struct that represents a service for serving embedded files.
///
/// # Parameters
/// - `E`: A type that implements the [`RustEmbed`] and `Clone` trait. This type represents the embedded files.
///
/// # Example
/// ```ignore
/// # use rust_embed::RustEmbed;
/// # use axum_embed::ServeEmbed;
/// # use tokio::net::TcpListener;
/// #
/// #[derive(RustEmbed, Clone)]
/// #[folder = "examples/assets/"]
/// struct Assets;
///
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()> {
/// let listener = TcpListener::bind("127.0.0.1:8080").await?;
/// let serve_assets = ServeEmbed::<Assets>::new();
/// let app = axum::Router::new().nest_service("/", serve_assets);
/// axum::serve(listener, app).await?;
///
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ServeEmbed<E: RustEmbed + Clone> {
    _phantom: std::marker::PhantomData<E>,
    fallback_file: Arc<Option<String>>,
    fallback_behavior: FallbackBehavior,
    index_file: Arc<Option<String>>,
}

impl<E: RustEmbed + Clone> ServeEmbed<E> {
    /// Constructs a new `ServeEmbed` instance with default parameters.
    ///
    /// This function calls `with_parameters` internally with `None` for `fallback_file`, [`FallbackBehavior::NotFound`] for `fallback_behavior`, and `"index.html"` for `index_file`.
    ///
    /// # Returns
    /// A new `ServeEmbed` instance with default parameters.
    pub fn new() -> Self {
        Self::with_parameters(
            None,
            FallbackBehavior::NotFound,
            Some("index.html".to_owned()),
        )
    }

    /// Constructs a new `ServeEmbed` instance with the provided parameters.
    ///
    /// # Parameters
    /// - `fallback_file`: The path of the file to serve when a requested file is not found. If `None`, a default 404 response is served.
    /// - `fallback_behavior`: The behavior of the server when a requested file is not found. Please see [`FallbackBehavior`] for more information.
    /// - `index_file`: The name of the file to serve when a directory is accessed. If `None`, a 404 response is served for directory.
    ///
    /// # Returns
    /// A new `ServeEmbed` instance.
    pub fn with_parameters(
        fallback_file: Option<String>,
        fallback_behavior: FallbackBehavior,
        index_file: Option<String>,
    ) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
            fallback_file: Arc::new(fallback_file),
            fallback_behavior,
            index_file: Arc::new(index_file),
        }
    }
}

impl<E: RustEmbed + Clone, T: Send + 'static> Service<http::request::Request<T>> for ServeEmbed<E> {
    type Response = http::Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = ServeFuture<E, T>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: http::request::Request<T>) -> Self::Future {
        ServeFuture {
            _phantom: std::marker::PhantomData,
            fallback_behavior: self.fallback_behavior,
            fallback_file: self.fallback_file.clone(),
            index_file: self.index_file.clone(),
            request: req,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CompressionMethod {
    Identity,
    Brotli,
    Gzip,
    Zlib,
}

impl CompressionMethod {
    fn extension(self) -> &'static str {
        match self {
            Self::Identity => "",
            Self::Brotli => ".br",
            Self::Gzip => ".gz",
            Self::Zlib => ".zz",
        }
    }
}

fn from_acceptable_encoding(acceptable_encoding: Option<&str>) -> Vec<CompressionMethod> {
    let mut compression_methods = Vec::new();

    let mut identity_found = false;
    for acceptable_encoding in acceptable_encoding.unwrap_or("").split(',') {
        let acceptable_encoding = acceptable_encoding.trim().split(';').next().unwrap();
        if acceptable_encoding == "br" {
            compression_methods.push(CompressionMethod::Brotli);
        } else if acceptable_encoding == "gzip" {
            compression_methods.push(CompressionMethod::Gzip);
        } else if acceptable_encoding == "deflate" {
            compression_methods.push(CompressionMethod::Zlib);
        } else if acceptable_encoding == "identity" {
            compression_methods.push(CompressionMethod::Identity);
            identity_found = true;
        }
    }

    if !identity_found {
        compression_methods.push(CompressionMethod::Identity);
    }

    compression_methods
}

fn cow_to_bytes(cow: Cow<'static, [u8]>) -> Bytes {
    match cow {
        Cow::Borrowed(x) => Bytes::from(x),
        Cow::Owned(x) => Bytes::from(x),
    }
}

struct GetFileResult<'a> {
    path: Cow<'a, str>,
    file: Option<rust_embed::EmbeddedFile>,
    should_redirect: Option<String>,
    compression_method: CompressionMethod,
    is_fallback: bool,
}

/// `ServeFuture` is a future that represents a service for serving embedded files.
/// This future is created by `ServeEmbed`.
/// This future is not intended to be used directly.
#[derive(Debug, Clone)]
pub struct ServeFuture<E: RustEmbed, T> {
    _phantom: std::marker::PhantomData<E>,
    fallback_behavior: FallbackBehavior,
    fallback_file: Arc<Option<String>>,
    index_file: Arc<Option<String>>,
    request: Request<T>,
}

impl<E: RustEmbed, T> ServeFuture<E, T> {
    /// Attempts to get a file from the embedded files based on the provided path and acceptable encodings.
    ///
    /// # Parameters
    /// - `path`: The path of the requested file. This should be a relative path from the root of the embedded files.
    /// - `acceptable_encoding`: A list of compression methods that the client can accept. This is typically obtained from the `Accept-Encoding` header of the HTTP request.
    ///
    /// # Returns
    /// A `GetFileResult` instance. If a file is found that matches the path and one of the acceptable encodings, it is included in the result. Otherwise, the result includes the path and `None` for the file.
    fn get_file<'a>(
        &self,
        path: Cow<'a, str>,
        acceptable_encoding: &[CompressionMethod],
    ) -> GetFileResult<'a> {
        let mut path_candidate = Cow::Owned(path.trim_start_matches('/').to_string());

        if path_candidate == "" {
            if let Some(index_file) = self.index_file.as_ref() {
                path_candidate = Cow::Owned(index_file.to_string());
            }
        } else if path_candidate.ends_with('/') {
            if let Some(index_file) = self.index_file.as_ref().as_ref() {
                let new_path_candidate = format!("{}{}", path_candidate, index_file);
                if E::get(&new_path_candidate).is_some() {
                    path_candidate = Cow::Owned(new_path_candidate);
                }
            }
        } else {
            if let Some(index_file) = self.index_file.as_ref().as_ref() {
                let new_path_candidate = format!("{}/{}", path_candidate, index_file);
                if E::get(&new_path_candidate).is_some() {
                    return GetFileResult {
                        path: Cow::Owned(new_path_candidate),
                        file: None,
                        should_redirect: Some(format!("/{}/", path_candidate)),
                        compression_method: CompressionMethod::Identity,
                        is_fallback: false,
                    };
                }
            }
        }

        let mut file = E::get(&path_candidate);
        let mut compressed_method = CompressionMethod::Identity;

        if file.is_some() {
            for one_method in acceptable_encoding {
                if let Some(x) = E::get(&format!("{}{}", path_candidate, one_method.extension())) {
                    file = Some(x);
                    compressed_method = *one_method;
                    break;
                }
            }
        }

        GetFileResult {
            path: path_candidate,
            file,
            should_redirect: None,
            compression_method: compressed_method,
            is_fallback: false,
        }
    }

    fn get_file_with_fallback<'a, 'b: 'a>(
        &'b self,
        path: &'a str,
        acceptable_encoding: &[CompressionMethod],
    ) -> GetFileResult<'a> {
        // Check direct match
        let first_try = self.get_file(Cow::Borrowed(path), acceptable_encoding);
        if first_try.file.is_some() || first_try.should_redirect.is_some() {
            return first_try;
        }
        // Now check in case the request had HTML escape encoding
        let decoded_path = percent_encoding::percent_decode_str(path).decode_utf8_lossy();
        if decoded_path!=path {
            let decoded_try = self.get_file(decoded_path, acceptable_encoding);
            if decoded_try.file.is_some() || decoded_try.should_redirect.is_some() {
                return decoded_try;
            }
        }

        // Now check system-like fallback
        if let Some(fallback_file) = self.fallback_file.as_ref().as_ref() {
            if fallback_file != path && self.fallback_behavior == FallbackBehavior::Redirect {
                return GetFileResult {
                    path: Cow::Borrowed(path),
                    file: None,
                    should_redirect: Some(format!("/{}", fallback_file)),
                    compression_method: CompressionMethod::Identity,
                    is_fallback: true,
                };
            }
            let mut fallback_try = self.get_file(Cow::Borrowed(fallback_file), acceptable_encoding);
            fallback_try.is_fallback = true;
            if fallback_try.file.is_some() {
                return fallback_try;
            }
        }
        GetFileResult {
            path: Cow::Borrowed("404.html"),
            file: DefaultFallback::get("404.html"),
            should_redirect: None,
            compression_method: CompressionMethod::Identity,
            is_fallback: true,
        }
    }
}

impl<E: RustEmbed, T> Future for ServeFuture<E, T> {
    type Output = Result<Response<Full<Bytes>>, Infallible>;

    fn poll(self: Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        // Accept only GET and HEAD method
        if self.request.method() != http::Method::GET && self.request.method() != http::Method::HEAD
        {
            return Poll::Ready(Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(http::header::CONTENT_TYPE, "text/plain")
                .body(Full::new(Bytes::from("Method not allowed")))
                .unwrap()));
        }

        // get embedded file for the requested path
        let (path, file, compression_method, is_fallback) = match self.get_file_with_fallback(
            self.request.uri().path(),
            &from_acceptable_encoding(
                self.request
                    .headers()
                    .get(http::header::ACCEPT_ENCODING)
                    .map(|x| x.to_str().ok())
                    .flatten(),
            ),
        ) {
            // if the file is found, return it
            GetFileResult {
                path,
                file: Some(file),
                should_redirect: None,
                compression_method,
                is_fallback,
            } => (path, file, compression_method, is_fallback),
            // if the path is a directory and the client does not have a trailing slash, redirect to the directory with a trailing slash
            GetFileResult {
                path: _,
                file: _,
                should_redirect: Some(should_redirect),
                compression_method: _,
                is_fallback,
            } => {
                return Poll::Ready(Ok(Response::builder()
                    .status(if is_fallback {
                        StatusCode::TEMPORARY_REDIRECT
                    } else {
                        StatusCode::MOVED_PERMANENTLY
                    })
                    .header(http::header::LOCATION, should_redirect)
                    .header(http::header::CONTENT_TYPE, "text/plain")
                    .body(Full::new(if is_fallback {
                        Bytes::from("Temporary redirect")
                    } else {
                        Bytes::from("Moved permanently")
                    }))
                    .unwrap()));
            }
            // if the file is not found, return 404
            _ => {
                unreachable!();
            }
        };

        // If the client has the same file, return 304
        if !is_fallback
            && self
                .request
                .headers()
                .get(http::header::IF_NONE_MATCH)
                .and_then(|value| {
                    value
                        .to_str()
                        .ok()
                        .and_then(|value| Some(value.trim_matches('"')))
                })
                == Some(hash_to_string(&file.metadata.sha256_hash()).as_str())
        {
            return Poll::Ready(Ok(Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .body(Full::new(Bytes::from("")))
                .unwrap()));
        }

        // build response and set headers
        let mut response_builder = Response::builder()
            .header(
                http::header::CONTENT_TYPE,
                mime_guess::from_path(path.as_ref())
                    .first_or_octet_stream()
                    .to_string(),
            )
            .header(
                http::header::ETAG,
                hash_to_string(&file.metadata.sha256_hash()),
            );

        match compression_method {
            CompressionMethod::Identity => {}
            CompressionMethod::Brotli => {
                response_builder = response_builder.header(http::header::CONTENT_ENCODING, "br");
            }
            CompressionMethod::Gzip => {
                response_builder = response_builder.header(http::header::CONTENT_ENCODING, "gzip");
            }
            CompressionMethod::Zlib => {
                response_builder =
                    response_builder.header(http::header::CONTENT_ENCODING, "deflate");
            }
        }

        if let Some(last_modified) = file.metadata.last_modified() {
            response_builder =
                response_builder.header(http::header::LAST_MODIFIED, date_to_string(last_modified));
        }

        if is_fallback && self.fallback_behavior != FallbackBehavior::Ok {
            response_builder = response_builder.status(StatusCode::NOT_FOUND);
        } else {
            response_builder = response_builder.status(StatusCode::OK);
        }

        Poll::Ready(Ok(response_builder
            .body(Full::new(cow_to_bytes(file.data)))
            .unwrap()))
    }
}

fn hash_to_string(hash: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for byte in hash {
        s.push_str(&format!("{:02x}", byte));
    }
    s
}

fn date_to_string(date: u64) -> String {
    DateTime::<Utc>::from_timestamp(date as i64, 0)
        .unwrap()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string()
}

#[cfg(test)]
mod test;
