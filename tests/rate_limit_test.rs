mod common;

use axum::http::StatusCode;
use common::{
    create_test_app_with_rate_limit, fixtures::valid_invoice_json,
    multipart::create_invoice_request,
};
use tower::ServiceExt;

#[tokio::test]
async fn rate_limiting_blocks_after_burst_exceeded() {
    let app = create_test_app_with_rate_limit(3600, 2).await;
    let invoice = valid_invoice_json();

    // First 2 requests should succeed (burst_size=2)
    for i in 0..2 {
        let request = create_invoice_request(&invoice);
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "Request {} should succeed within burst limit",
            i + 1
        );
    }

    // 3rd request should be rate limited
    let request = create_invoice_request(&invoice);
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Request after burst should be rate limited"
    );
}
