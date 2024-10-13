use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use clap::Parser;
use std::net::SocketAddr;
use std::sync::LazyLock;

mod api;
mod error;
#[cfg(feature = "email")]
mod mailgun;
mod merge;
mod state;

mod pdfgen;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate tracing;

#[cfg(feature = "email")]
#[derive(Parser, Clone, Debug)]
struct MailgunConfig {
    /// Url used by mailgun
    #[clap(long = "mailgun-url", env = "MAILGUN_URL")]
    url: String,
    /// Username used by mailgun
    #[clap(long = "mailgun-user", env = "MAILGUN_USER")]
    user: String,
    /// Password used by mailgun
    #[clap(long = "mailgun-password", env = "MAILGUN_PASSWORD")]
    password: String,
    /// Initial To-value used by mailgun
    #[clap(long = "mailgun-to", env = "MAILGUN_TO")]
    to: String,
    /// From-value used by mailgun
    #[clap(long = "mailgun-from", env = "MAILGUN_FROM")]
    from: String,
}

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
struct LaskugenConfig {
    #[cfg(feature = "email")]
    #[clap(flatten)]
    mailgun: MailgunConfig,
    /// The listen port for the HTTP server
    #[clap(long, env, required = false, default_value = "3000")]
    port: u16,
    /// The ip address to bound by the HTTP server
    #[clap(long, env, required = false, default_value = "127.0.0.1")]
    bind_addr: std::net::IpAddr,
    /// A comma-separated list of allowed origins
    #[clap(long, env, required = false, value_delimiter = ',')]
    allowed_origins: Vec<String>,
}

static CONFIG: LazyLock<LaskugenConfig> = LazyLock::new(LaskugenConfig::parse);

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "laskugeneraattori=debug,tower_http=debug,axum::rejection=trace".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = state::new().await;
    let addr = SocketAddr::from((CONFIG.bind_addr, CONFIG.port));
    debug!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TcpListener");

    axum::serve(
        listener,
        api::app()
            .with_state(state)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed to start server");
}
