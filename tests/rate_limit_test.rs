mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::{
    create_invoice_form, fixtures::valid_invoice_json, setup_test_env, TEST_IP, TEST_IP_HEADER,
};
use laskugeneraattori::{api::app, state};

async fn create_test_server_with_rate_limit(period_secs: u64, burst_size: u32) -> TestServer {
    setup_test_env();
    std::env::set_var("RATE_LIMIT_PERIOD_SECS", period_secs.to_string());
    std::env::set_var("RATE_LIMIT_BURST_SIZE", burst_size.to_string());
    let state = state::new().await;
    let app = app().with_state(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn rate_limiting_blocks_after_burst_exceeded() {
    let server = create_test_server_with_rate_limit(3600, 2).await;
    let invoice = valid_invoice_json();

    // First 2 requests should succeed (burst_size=2)
    for i in 0..2 {
        let form = create_invoice_form(&invoice);
        let response = server
            .post("/invoices")
            .add_header(TEST_IP_HEADER, TEST_IP)
            .multipart(form)
            .await;
        assert_eq!(
            response.status_code(),
            StatusCode::CREATED,
            "Request {} should succeed within burst limit",
            i + 1
        );
    }

    // 3rd request should be rate limited
    let form = create_invoice_form(&invoice);
    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;
    assert_eq!(
        response.status_code(),
        StatusCode::TOO_MANY_REQUESTS,
        "Request after burst should be rate limited"
    );
}
