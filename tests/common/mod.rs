#[allow(dead_code)]
pub mod fixtures;

use axum_test::multipart::{MultipartForm, Part};
use axum_test::TestServer;
use laskugeneraattori::{api::app, state};
use serde_json::Value;
use std::path::Path;

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
pub async fn create_test_server() -> TestServer {
    setup_test_env();
    let state = state::new().await;
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
