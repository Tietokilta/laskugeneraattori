use clap::Parser;
use std::sync::LazyLock;

pub mod api;
pub mod error;
pub mod mailgun;
pub mod merge;
pub mod pdfgen;
pub mod state;

#[macro_use]
extern crate tracing;

#[derive(Parser, Clone, Debug)]
pub struct MailgunConfig {
    #[clap(
        long = "mailgun-disable",
        env = "MAILGUN_DISABLE",
        default_value = "false"
    )]
    pub disable: bool,
    #[clap(
        long = "mailgun-url",
        env = "MAILGUN_URL",
        required_if_eq("disable", "false")
    )]
    pub url: Option<String>,
    #[clap(
        long = "mailgun-user",
        env = "MAILGUN_USER",
        required_if_eq("disable", "false")
    )]
    pub user: Option<String>,
    #[clap(
        long = "mailgun-password",
        env = "MAILGUN_PASSWORD",
        required_if_eq("disable", "false")
    )]
    pub password: Option<String>,
    #[clap(
        long = "mailgun-to",
        env = "MAILGUN_TO",
        required_if_eq("disable", "false")
    )]
    pub to: Option<String>,
    #[clap(
        long = "mailgun-from",
        env = "MAILGUN_FROM",
        required_if_eq("disable", "false")
    )]
    pub from: Option<String>,
}

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
pub struct LaskugenConfig {
    #[clap(flatten)]
    pub mailgun: MailgunConfig,
    #[clap(long, env, required = false, default_value = "3000")]
    pub port: u16,
    #[clap(long, env, required = false, default_value = "127.0.0.1")]
    pub bind_addr: std::net::IpAddr,
    #[clap(long, env, required = false, value_delimiter = ',')]
    pub allowed_origins: Vec<String>,
    #[clap(long, env, required = false)]
    pub ip_extractor_header: Option<String>,
    #[clap(long, env, default_value = "720")]
    pub rate_limit_period_secs: u64,
    #[clap(long, env, default_value = "5")]
    pub rate_limit_burst_size: u32,
}

pub static CONFIG: LazyLock<LaskugenConfig> = LazyLock::new(LaskugenConfig::parse);
