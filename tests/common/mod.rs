#[allow(dead_code)]
pub mod fixtures;

use axum_test::TestServer;
use axum_test::multipart::{MultipartForm, Part};
use laskugeneraattori::{api::app, cost_pools::CostPoolClient, state};
use serde_json::Value;
use std::path::Path;
use wiremock::matchers::{method, path as path_matcher};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(dead_code)]
pub const TEST_IP_HEADER: &str = "x-test-ip";
#[allow(dead_code)]
pub const TEST_IP: &str = "127.0.0.1";

/// A cost pool that the mock CMS knows about, see [`mock_cms`]
#[allow(dead_code)]
pub const TEST_COST_POOL_ID: &str = "507f1f77bcf86cd799439011";
#[allow(dead_code)]
pub const TEST_COST_POOL_NAME: &str = "Liikuntatoimikunta";
#[allow(dead_code)]
pub const TEST_COST_POOL_ACCOUNT: &str = "4212";

#[allow(dead_code)]
pub fn setup_test_env() {
    std::env::set_var("MAILGUN_DISABLE", "true");
    std::env::set_var("ALLOWED_ORIGINS", "http://localhost:3000");
    std::env::set_var("RATE_LIMIT_PERIOD_SECS", "1");
    std::env::set_var("RATE_LIMIT_BURST_SIZE", "100");
    std::env::set_var("IP_EXTRACTOR_HEADER", TEST_IP_HEADER);
    // Nothing listens here: unless a test points the server at a mock CMS, cost pools cannot be
    // resolved and every invoice is booked against the unassigned account
    std::env::set_var("CMS_URL", "http://localhost:1");
}

/// A CMS that knows about a single cost pool. Any other id gets a 404, which is what the real
/// CMS does for a cost pool that has been deleted.
#[allow(dead_code)]
pub async fn mock_cms() -> MockServer {
    let cms = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path_matcher(format!("/api/cost-pools/{TEST_COST_POOL_ID}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": TEST_COST_POOL_ID,
            "name": TEST_COST_POOL_NAME,
            "account": TEST_COST_POOL_ACCOUNT,
        })))
        .mount(&cms)
        .await;

    cms
}

#[allow(dead_code)]
pub async fn create_test_server() -> TestServer {
    setup_test_env();
    let state = state::new().await;
    let app = app().with_state(state);
    TestServer::new(app).unwrap()
}

/// A test server whose cost pools are resolved against the given CMS
#[allow(dead_code)]
pub async fn create_test_server_with_cms(cms_url: String) -> TestServer {
    setup_test_env();
    let mut state = state::new().await;
    state.cost_pool_client = CostPoolClient::new(cms_url);
    let app = app().with_state(state);
    TestServer::new(app).unwrap()
}

#[allow(dead_code)]
pub fn load_test_file(filename: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(filename);
    std::fs::read(&path).unwrap_or_else(|_| panic!("Failed to read test file: {}", filename))
}

#[allow(dead_code)]
pub fn create_invoice_form(invoice: &Value) -> MultipartForm {
    MultipartForm::new().add_part(
        "data",
        Part::bytes(invoice.to_string().into_bytes()).mime_type("application/json"),
    )
}

#[allow(dead_code)]
pub fn create_invoice_form_with_file(
    invoice: &Value,
    filename: &str,
    content: Vec<u8>,
) -> MultipartForm {
    MultipartForm::new()
        .add_part(
            "data",
            Part::bytes(invoice.to_string().into_bytes()).mime_type("application/json"),
        )
        .add_part("attachments", Part::bytes(content).file_name(filename))
}

#[allow(dead_code)]
pub fn create_invoice_form_with_files(
    invoice: &Value,
    files: Vec<(&str, Vec<u8>)>,
) -> MultipartForm {
    let mut form = MultipartForm::new().add_part(
        "data",
        Part::bytes(invoice.to_string().into_bytes()).mime_type("application/json"),
    );
    for (filename, content) in files {
        form = form.add_part("attachments", Part::bytes(content).file_name(filename));
    }
    form
}
