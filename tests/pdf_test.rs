mod common;

use axum::body::to_bytes;
use axum::http::StatusCode;
use common::{
    create_test_app,
    fixtures::{invoice_with_attachment_descriptions, valid_invoice_json},
    multipart::{create_invoice_request, create_invoice_request_with_file, load_test_file},
};
use tower::ServiceExt;

#[tokio::test]
async fn invoice_creation_returns_valid_json_response() {
    let app = create_test_app().await;
    let invoice = valid_invoice_json();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(response_json["recipient_name"], "Test User");
    assert_eq!(response_json["subject"], "Test Invoice");
}

#[tokio::test]
async fn invoice_with_image_returns_attachment_info() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.jpg");
    let request = create_invoice_request_with_file(&invoice, "test.jpg", image_data);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let attachments = response_json["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["filename"], "test.jpg");
}

#[tokio::test]
async fn invoice_with_pdf_attachment_returns_attachment_info() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Receipt"]);
    let pdf_data = load_test_file("test.pdf");
    let request = create_invoice_request_with_file(&invoice, "receipt.pdf", pdf_data);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let attachments = response_json["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["filename"], "receipt.pdf");
}

#[tokio::test]
async fn invoice_with_multiple_attachments_returns_all_attachment_info() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Image", "Document"]);
    let jpg_data = load_test_file("test.jpg");
    let pdf_data = load_test_file("test.pdf");

    let request = common::multipart::create_invoice_request_with_files(
        &invoice,
        vec![("photo.jpg", jpg_data), ("document.pdf", pdf_data)],
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let attachments = response_json["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 2);
}

#[tokio::test]
async fn invoice_rows_are_preserved_in_response() {
    let app = create_test_app().await;
    let mut invoice = valid_invoice_json();
    invoice["rows"] = serde_json::json!([
        { "product": "Product A", "unit_price": 1000 },
        { "product": "Product B", "unit_price": 2500 }
    ]);
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let rows = response_json["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["product"], "Product A");
    assert_eq!(rows[0]["unit_price"], 1000);
    assert_eq!(rows[1]["product"], "Product B");
    assert_eq!(rows[1]["unit_price"], 2500);
}
