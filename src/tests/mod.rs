use axum::http::StatusCode;
use axum_test::TestServer;
use laskugeneraattori::{api::app, state};

fn setup_test_env() {
    std::env::set_var("MAILGUN_DISABLE", "true");
    std::env::set_var("ALLOWED_ORIGINS", "http://localhost:3000");
    std::env::set_var("CMS_URL", "http://localhost:1");
}

#[tokio::test]
async fn health() {
    setup_test_env();
    let app = app().with_state(state::new().await);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/health").await;

    response.assert_status(StatusCode::OK);
}
