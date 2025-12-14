use axum::body::Body;
use axum::http::request::Request;
use axum::http::StatusCode;
use laskugeneraattori::{api::app, state};
use tower::ServiceExt;

fn setup_test_env() {
    std::env::set_var("MAILGUN_DISABLE", "true");
    std::env::set_var("ALLOWED_ORIGINS", "http://localhost:3000");
}

#[tokio::test]
async fn health() {
    setup_test_env();
    let app = app().with_state(state::new().await);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
