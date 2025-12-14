mod common;

use axum::http::StatusCode;
use common::{
    create_test_app,
    fixtures::{
        invoice_with_attachment_descriptions, invoice_with_empty_rows, invoice_with_empty_subject,
        invoice_with_invalid_iban, invoice_with_invalid_phone, invoice_with_long_subject,
        invoice_with_multiple_rows, invoice_with_negative_price, invoice_with_zero_price,
        valid_invoice_json,
    },
    multipart::{
        create_invoice_request, create_invoice_request_with_file,
        create_invoice_request_with_files, load_test_file, MultipartBuilder,
    },
};
use tower::ServiceExt;

#[tokio::test]
async fn create_invoice_without_attachments_succeeds() {
    let app = create_test_app().await;
    let invoice = valid_invoice_json();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_jpg_attachment_succeeds() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.jpg");
    let request = create_invoice_request_with_file(&invoice, "test.jpg", image_data);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_png_attachment_succeeds() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.png");
    let request = create_invoice_request_with_file(&invoice, "test.png", image_data);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_pdf_attachment_succeeds() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test PDF"]);
    let pdf_data = load_test_file("test.pdf");
    let request = create_invoice_request_with_file(&invoice, "receipt.pdf", pdf_data);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_multiple_attachments_succeeds() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Image", "PDF"]);
    let jpg_data = load_test_file("test.jpg");
    let pdf_data = load_test_file("test.pdf");
    let request = create_invoice_request_with_files(
        &invoice,
        vec![("test.jpg", jpg_data), ("doc.pdf", pdf_data)],
    );

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_multiple_rows_succeeds() {
    let app = create_test_app().await;
    let invoice = invoice_with_multiple_rows();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn reject_invalid_iban() {
    let app = create_test_app().await;
    let invoice = invoice_with_invalid_iban();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_invalid_phone_number() {
    let app = create_test_app().await;
    let invoice = invoice_with_invalid_phone();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_empty_rows() {
    let app = create_test_app().await;
    let invoice = invoice_with_empty_rows();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_negative_unit_price() {
    let app = create_test_app().await;
    let invoice = invoice_with_negative_price();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_zero_unit_price() {
    let app = create_test_app().await;
    let invoice = invoice_with_zero_price();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_subject_exceeding_max_length() {
    let app = create_test_app().await;
    let invoice = invoice_with_long_subject();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_empty_subject() {
    let app = create_test_app().await;
    let invoice = invoice_with_empty_subject();
    let request = create_invoice_request(&invoice);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reject_unsupported_file_format() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Bad file"]);
    let request =
        create_invoice_request_with_file(&invoice, "malicious.exe", b"fake executable".to_vec());

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reject_txt_file_format() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Text file"]);
    let request =
        create_invoice_request_with_file(&invoice, "notes.txt", b"some text content".to_vec());

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn accept_uppercase_file_extensions() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.png");
    let request = create_invoice_request_with_file(&invoice, "TEST.PNG", image_data);

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn reject_missing_filename() {
    let app = create_test_app().await;
    let invoice = invoice_with_attachment_descriptions(vec!["No filename"]);

    let request = MultipartBuilder::new()
        .add_json_field("data", &invoice)
        .add_file_without_filename("attachments", b"some content".to_vec())
        .into_request("/invoices");

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reject_malformed_json() {
    let app = create_test_app().await;

    let request = MultipartBuilder::new()
        .add_raw_field("data", "application/json", b"{ invalid json }")
        .into_request("/invoices");

    let response = app.oneshot(request).await.unwrap();

    // Malformed JSON is rejected - axum_typed_multipart returns 500 for JSON parse errors
    assert!(
        response.status().is_client_error() || response.status().is_server_error(),
        "Expected error response for malformed JSON"
    );
}
