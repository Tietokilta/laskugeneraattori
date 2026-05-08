mod common;

use axum::http::StatusCode;
use common::{
    create_invoice_form, create_invoice_form_with_file, create_invoice_form_with_files,
    create_test_server,
    fixtures::{invoice_with_attachment_descriptions, valid_invoice_json},
    load_test_file, TEST_IP, TEST_IP_HEADER,
};
use serde_json::Value;

fn is_valid_finnish_reference_number(value: &str) -> bool {
    if value.len() < 4 || value.len() > 20 || !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let weights = [7u32, 3, 1];
    let (base, check_digit_str) = value.split_at(value.len() - 1);
    let sum = base
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| u32::from(b - b'0') * weights[i % weights.len()])
        .sum::<u32>();
    let expected = ((10 - (sum % 10)) % 10) as u8;

    check_digit_str
        .chars()
        .next()
        .and_then(|c| c.to_digit(10))
        .is_some_and(|d| d as u8 == expected)
}

#[tokio::test]
async fn invoice_creation_returns_valid_json_response() {
    let server = create_test_server().await;
    let invoice = valid_invoice_json();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
    let response_json: Value = response.json();
    assert_eq!(response_json["recipient_name"], "Test User");
    assert_eq!(response_json["subject"], "Test Invoice");
    let reference_number = response_json["reference_number"].as_str().unwrap();
    assert!(is_valid_finnish_reference_number(reference_number));
}

#[tokio::test]
async fn invoice_with_image_returns_attachment_info() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.jpg");
    let form = create_invoice_form_with_file(&invoice, "test.jpg", image_data);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
    let response_json: Value = response.json();
    let attachments = response_json["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["filename"], "test.jpg");
}

#[tokio::test]
async fn invoice_with_pdf_attachment_returns_attachment_info() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Receipt"]);
    let pdf_data = load_test_file("test.pdf");
    let form = create_invoice_form_with_file(&invoice, "receipt.pdf", pdf_data);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
    let response_json: Value = response.json();
    let attachments = response_json["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["filename"], "receipt.pdf");
}

#[tokio::test]
async fn invoice_with_multiple_attachments_returns_all_attachment_info() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Image", "Document"]);
    let jpg_data = load_test_file("test.jpg");
    let pdf_data = load_test_file("test.pdf");
    let form = create_invoice_form_with_files(
        &invoice,
        vec![("photo.jpg", jpg_data), ("document.pdf", pdf_data)],
    );

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
    let response_json: Value = response.json();
    let attachments = response_json["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 2);
}

#[tokio::test]
async fn invoice_rows_are_preserved_in_response() {
    let server = create_test_server().await;
    let mut invoice = valid_invoice_json();
    invoice["rows"] = serde_json::json!([
        { "product": "Product A", "unit_price": 1000 },
        { "product": "Product B", "unit_price": 2500 }
    ]);
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
    let response_json: Value = response.json();
    let rows = response_json["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["product"], "Product A");
    assert_eq!(rows[0]["unit_price"], 1000);
    assert_eq!(rows[1]["product"], "Product B");
    assert_eq!(rows[1]["unit_price"], 2500);
}
