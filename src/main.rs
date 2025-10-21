use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use clap::Parser;
use std::net::SocketAddr;
use std::sync::LazyLock;

mod api;
mod error;
mod mailgun;
mod merge;
mod state;

mod pdfgen;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate tracing;

#[derive(Parser, Clone, Debug)]
struct MailgunConfig {
    #[clap(
        long = "mailgun-disable",
        env = "MAILGUN_DISABLE",
        default_value = "false"
    )]
    disable: bool,
    /// Url used by mailgun
    #[clap(
        long = "mailgun-url",
        env = "MAILGUN_URL",
        required_if_eq("disable", "false")
    )]
    url: Option<String>,
    /// Username used by mailgun
    #[clap(
        long = "mailgun-user",
        env = "MAILGUN_USER",
        required_if_eq("disable", "false")
    )]
    user: Option<String>,
    /// Password used by mailgun
    #[clap(
        long = "mailgun-password",
        env = "MAILGUN_PASSWORD",
        required_if_eq("disable", "false")
    )]
    password: Option<String>,
    /// Initial To-value used by mailgun
    #[clap(
        long = "mailgun-to",
        env = "MAILGUN_TO",
        required_if_eq("disable", "false")
    )]
    to: Option<String>,
    /// From-value used by mailgun
    #[clap(
        long = "mailgun-from",
        env = "MAILGUN_FROM",
        required_if_eq("disable", "false")
    )]
    from: Option<String>,
}

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
struct LaskugenConfig {
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
