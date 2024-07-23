use clap::Parser;
use rust_embed::RustEmbed;
use tokio::net::TcpListener;

#[derive(RustEmbed, Clone)]
#[folder = "examples/assets/"]
struct Assets;

#[derive(Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
enum FallbackBehavior {
    Ok,
    Redirect,
    NotFound,
}

#[derive(clap::Parser)]
struct Opt {
    #[clap(default_value = "127.0.0.1:8080", help = "Listen address")]
    listen: String,
    #[clap(
        long,
        short,
        help = "Serve fallback.html with code 200 if file not found"
    )]
    fallback: bool,
    #[clap(
        long,
        short = 'b',
        help = "Serve 404.html with code 404 if file not found",
        default_value = "not-found"
    )]
    fallback_behavior: FallbackBehavior,
    #[clap(
        long,
        short = 'i',
        help = "Disable serving index.html if path is directory"
    )]
    no_index: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();
    let listener = TcpListener::bind(opt.listen).await?;
    eprintln!("Listening on http://{}", listener.local_addr()?);
    let assets = axum_embed::ServeEmbed::<Assets>::with_parameters(
        if opt.fallback {
            Some("404.html".to_owned())
        } else {
            None
        },
        match opt.fallback_behavior {
            FallbackBehavior::Ok => axum_embed::FallbackBehavior::Ok,
            FallbackBehavior::Redirect => axum_embed::FallbackBehavior::Redirect,
            FallbackBehavior::NotFound => axum_embed::FallbackBehavior::NotFound,
        },
        if opt.no_index {
            None
        } else {
            Some("index.html".to_owned())
        },
        None,
    );
    let app = axum::Router::new().nest_service("/", assets);
    axum::serve(listener, app).await?;

    Ok(())
}
