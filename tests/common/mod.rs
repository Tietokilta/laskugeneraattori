#[allow(dead_code)]
pub mod fixtures;
#[allow(dead_code)]
pub mod multipart;

use axum::Router;
use laskugeneraattori::{api::app, state};

#[allow(dead_code)]
pub const TEST_IP_HEADER: &str = "x-test-ip";
#[allow(dead_code)]
pub const TEST_IP: &str = "127.0.0.1";

#[allow(dead_code)]
pub fn setup_test_env() {
    std::env::set_var("MAILGUN_DISABLE", "true");
    std::env::set_var("ALLOWED_ORIGINS", "http://localhost:3000");
    std::env::set_var("RATE_LIMIT_PERIOD_SECS", "1");
    std::env::set_var("RATE_LIMIT_BURST_SIZE", "100");
    std::env::set_var("IP_EXTRACTOR_HEADER", TEST_IP_HEADER);
}

#[allow(dead_code)]
pub async fn create_test_app() -> Router<()> {
    setup_test_env();
    let state = state::new().await;
    app().with_state(state)
}

#[allow(dead_code)]
pub async fn create_test_app_with_rate_limit(period_secs: u64, burst_size: u32) -> Router<()> {
    std::env::set_var("MAILGUN_DISABLE", "true");
    std::env::set_var("ALLOWED_ORIGINS", "http://localhost:3000");
    std::env::set_var("RATE_LIMIT_PERIOD_SECS", period_secs.to_string());
    std::env::set_var("RATE_LIMIT_BURST_SIZE", burst_size.to_string());
    std::env::set_var("IP_EXTRACTOR_HEADER", TEST_IP_HEADER);
    let state = state::new().await;
    app().with_state(state)
}
