use super::*;
use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use rust_embed::RustEmbed;
use tower::ServiceExt;

#[derive(RustEmbed, Clone)]
#[folder = "examples/assets"]
struct Assets;

#[derive(Debug, Clone)]
struct Expected {
    uri: &'static str,
    status: http::StatusCode,
    content_type: &'static str,
    encoding: Option<&'static str>,
    location: Option<&'static str>,
    body: &'static [u8],
}

impl Expected {
    async fn test(&self, assets: ServeEmbed<Assets>) -> anyhow::Result<()> {
        let app = axum::Router::new().fallback_service(assets);
        let response = app
            .oneshot(
                Request::builder()
                    .uri(self.uri)
                    .header(http::header::ACCEPT_ENCODING, "br, gzip, deflate")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), self.status);
        assert_eq!(
            response
                .headers()
                .get(http::header::CONTENT_TYPE)
                .map(|x| x.to_str().unwrap()),
            Some(self.content_type)
        );
        assert_eq!(
            response
                .headers()
                .get(http::header::CONTENT_ENCODING)
                .map(|x| x.to_str().unwrap()),
            self.encoding
        );
        assert_eq!(
            response
                .headers()
                .get(http::header::LOCATION)
                .map(|x| x.to_str().unwrap()),
            self.location
        );

        let data = response.into_body().collect().await?.to_bytes();
        assert_eq!(&data[..], self.body);
        Ok(())
    }
}

#[tokio::test]
async fn test_default_index() -> anyhow::Result<()> {
    let assets = ServeEmbed::<Assets>::new();
    // for one_file in Assets::iter() {
    //     eprintln!("file: {}", one_file.as_ref());
    // }

    Expected {
        uri: "/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/index.html",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/sample.js",
        status: http::StatusCode::OK,
        content_type: "application/javascript",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/sample.js.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/style.css",
        status: http::StatusCode::OK,
        content_type: "text/css",
        encoding: Some("gzip"),
        location: None,
        body: include_bytes!("../examples/assets/style.css.gz"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/subdir/index.html.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir",
        status: http::StatusCode::MOVED_PERMANENTLY,
        content_type: "text/plain",
        encoding: None,
        location: Some("/subdir/"),
        body: b"Moved permanently",
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox.webp",
        status: http::StatusCode::OK,
        content_type: "image/webp",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/images/fox/fox.webp"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox2.webp",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/not-found",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_no_default_index() -> anyhow::Result<()> {
    let assets = ServeEmbed::<Assets>::with_parameters(None, FallbackBehavior::NotFound, None);

    Expected {
        uri: "/",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("./assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/index.html",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/sample.js",
        status: http::StatusCode::OK,
        content_type: "application/javascript",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/sample.js.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/style.css",
        status: http::StatusCode::OK,
        content_type: "text/css",
        encoding: Some("gzip"),
        location: None,
        body: include_bytes!("../examples/assets/style.css.gz"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir/",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("./assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("./assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox.webp",
        status: http::StatusCode::OK,
        content_type: "image/webp",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/images/fox/fox.webp"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox2.webp",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/not-found",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_fallback_ok() -> anyhow::Result<()> {
    let assets = ServeEmbed::<Assets>::with_parameters(
        Some("404.html".to_string()),
        FallbackBehavior::Ok,
        Some("index.html".to_string()),
    );
    // for one_file in Assets::iter() {
    //     eprintln!("file: {}", one_file.as_ref());
    // }

    Expected {
        uri: "/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/index.html",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/sample.js",
        status: http::StatusCode::OK,
        content_type: "application/javascript",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/sample.js.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/style.css",
        status: http::StatusCode::OK,
        content_type: "text/css",
        encoding: Some("gzip"),
        location: None,
        body: include_bytes!("../examples/assets/style.css.gz"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/subdir/index.html.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir",
        status: http::StatusCode::MOVED_PERMANENTLY,
        content_type: "text/plain",
        encoding: None,
        location: Some("/subdir/"),
        body: b"Moved permanently",
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox.webp",
        status: http::StatusCode::OK,
        content_type: "image/webp",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/images/fox/fox.webp"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox2.webp",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/not-found",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_redirect() -> anyhow::Result<()> {
    let assets = ServeEmbed::<Assets>::with_parameters(
        Some("404.html".to_string()),
        FallbackBehavior::Redirect,
        Some("index.html".to_string()),
    );

    Expected {
        uri: "/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/index.html",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/sample.js",
        status: http::StatusCode::OK,
        content_type: "application/javascript",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/sample.js.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/style.css",
        status: http::StatusCode::OK,
        content_type: "text/css",
        encoding: Some("gzip"),
        location: None,
        body: include_bytes!("../examples/assets/style.css.gz"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/subdir/index.html.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir",
        status: http::StatusCode::MOVED_PERMANENTLY,
        content_type: "text/plain",
        encoding: None,
        location: Some("/subdir/"),
        body: b"Moved permanently",
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox",
        status: http::StatusCode::TEMPORARY_REDIRECT,
        content_type: "text/plain",
        encoding: None,
        location: Some("/404.html"),
        body: b"Temporary redirect",
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox.webp",
        status: http::StatusCode::OK,
        content_type: "image/webp",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/images/fox/fox.webp"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox2.webp",
        status: http::StatusCode::TEMPORARY_REDIRECT,
        content_type: "text/plain",
        encoding: None,
        location: Some("/404.html"),
        body: b"Temporary redirect",
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/not-found",
        status: http::StatusCode::TEMPORARY_REDIRECT,
        content_type: "text/plain",
        encoding: None,
        location: Some("/404.html"),
        body: b"Temporary redirect",
    }
    .test(assets.clone())
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_custom_404() -> anyhow::Result<()> {
    let assets = ServeEmbed::<Assets>::with_parameters(
        Some("404.html".to_string()),
        FallbackBehavior::NotFound,
        Some("index.html".to_string()),
    );

    Expected {
        uri: "/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/index.html",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/index.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/sample.js",
        status: http::StatusCode::OK,
        content_type: "application/javascript",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/sample.js.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/style.css",
        status: http::StatusCode::OK,
        content_type: "text/css",
        encoding: Some("gzip"),
        location: None,
        body: include_bytes!("../examples/assets/style.css.gz"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir/",
        status: http::StatusCode::OK,
        content_type: "text/html",
        encoding: Some("br"),
        location: None,
        body: include_bytes!("../examples/assets/subdir/index.html.br"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/subdir",
        status: http::StatusCode::MOVED_PERMANENTLY,
        content_type: "text/plain",
        encoding: None,
        location: Some("/subdir/"),
        body: b"Moved permanently",
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox.webp",
        status: http::StatusCode::OK,
        content_type: "image/webp",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/images/fox/fox.webp"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/images/fox/fox2.webp",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Expected {
        uri: "/not-found",
        status: http::StatusCode::NOT_FOUND,
        content_type: "text/html",
        encoding: None,
        location: None,
        body: include_bytes!("../examples/assets/404.html"),
    }
    .test(assets.clone())
    .await?;

    Ok(())
}
