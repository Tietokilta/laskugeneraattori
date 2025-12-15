mod common;

use axum::http::StatusCode;
use axum_test::multipart::{MultipartForm, Part};
use common::{
    create_invoice_form, create_invoice_form_with_file, create_invoice_form_with_files,
    create_test_server,
    fixtures::{
        invoice_with_attachment_descriptions, invoice_with_empty_rows, invoice_with_empty_subject,
        invoice_with_invalid_iban, invoice_with_invalid_phone, invoice_with_long_subject,
        invoice_with_multiple_rows, invoice_with_negative_price, invoice_with_zero_price,
        valid_invoice_json,
    },
    load_test_file, TEST_IP, TEST_IP_HEADER,
};
use serde_json::Value;

#[tokio::test]
async fn create_invoice_without_attachments_succeeds() {
    let server = create_test_server().await;
    let invoice = valid_invoice_json();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_jpg_attachment_succeeds() {
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
}

#[tokio::test]
async fn create_invoice_with_png_attachment_succeeds() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.png");
    let form = create_invoice_form_with_file(&invoice, "test.png", image_data);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_pdf_attachment_succeeds() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test PDF"]);
    let pdf_data = load_test_file("test.pdf");
    let form = create_invoice_form_with_file(&invoice, "receipt.pdf", pdf_data);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_multiple_attachments_succeeds() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Image", "PDF"]);
    let jpg_data = load_test_file("test.jpg");
    let pdf_data = load_test_file("test.pdf");
    let form = create_invoice_form_with_files(
        &invoice,
        vec![("test.jpg", jpg_data), ("doc.pdf", pdf_data)],
    );

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn create_invoice_with_multiple_rows_succeeds() {
    let server = create_test_server().await;
    let invoice = invoice_with_multiple_rows();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn reject_invalid_iban() {
    let server = create_test_server().await;
    let invoice = invoice_with_invalid_iban();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "bank_account_number"]],
            { "message": "the string does not follow the base IBAN rules" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_invalid_phone_number() {
    let server = create_test_server().await;
    let invoice = invoice_with_invalid_phone();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "phone_number"]],
            { "message": "not a valid phone number: not a number" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_empty_rows() {
    let server = create_test_server().await;
    let invoice = invoice_with_empty_rows();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "rows"]],
            { "message": "length is lower than 1" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_negative_unit_price() {
    let server = create_test_server().await;
    let invoice = invoice_with_negative_price();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "rows"], ["index", "0"], ["key", "unit_price"]],
            { "message": "lower than 1" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_zero_unit_price() {
    let server = create_test_server().await;
    let invoice = invoice_with_zero_price();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "rows"], ["index", "0"], ["key", "unit_price"]],
            { "message": "lower than 1" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_subject_exceeding_max_length() {
    let server = create_test_server().await;
    let invoice = invoice_with_long_subject();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "subject"]],
            { "message": "length is greater than 128" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_empty_subject() {
    let server = create_test_server().await;
    let invoice = invoice_with_empty_subject();
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "errors": [[
            [["key", "data"], ["key", "subject"]],
            { "message": "length is lower than 1" }
        ]]
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_unsupported_file_format() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Bad file"]);
    let form =
        create_invoice_form_with_file(&invoice, "malicious.exe", b"fake executable".to_vec());

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "error": "Unsupported file format: malicious.exe. Supported file formats are (jpg|jpeg|png|gif|svg|pdf)"
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_txt_file_format() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Text file"]);
    let form = create_invoice_form_with_file(&invoice, "notes.txt", b"some text content".to_vec());

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "error": "Unsupported file format: notes.txt. Supported file formats are (jpg|jpeg|png|gif|svg|pdf)"
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn accept_uppercase_file_extensions() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["Test image"]);
    let image_data = load_test_file("test.png");
    let form = create_invoice_form_with_file(&invoice, "TEST.PNG", image_data);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn reject_missing_filename() {
    let server = create_test_server().await;
    let invoice = invoice_with_attachment_descriptions(vec!["No filename"]);

    let form = MultipartForm::new()
        .add_part(
            "data",
            Part::bytes(invoice.to_string().into_bytes()).mime_type("application/json"),
        )
        .add_part("attachments", Part::bytes(b"some content".to_vec()));

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = response.json();
    let expected: Value = serde_json::json!({
        "error": "Missing filename multipart"
    });
    assert_eq!(body, expected);
}

#[tokio::test]
async fn reject_malformed_json() {
    let server = create_test_server().await;

    let form = MultipartForm::new().add_part(
        "data",
        Part::bytes(b"{ invalid json }".to_vec()).mime_type("application/json"),
    );

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    assert!(
        response.status_code().is_client_error() || response.status_code().is_server_error(),
        "Expected error response for malformed JSON"
    );
}

#[tokio::test]
async fn iban_whitespace_is_stripped() {
    let server = create_test_server().await;
    let invoice = valid_invoice_json(); // Uses "FI21 1234 5600 0007 85"
    let form = create_invoice_form(&invoice);

    let response = server
        .post("/invoices")
        .add_header(TEST_IP_HEADER, TEST_IP)
        .multipart(form)
        .await;

    response.assert_status(StatusCode::CREATED);
    let response_json: Value = response.json();

    assert_eq!(
        response_json["bank_account_number"], "FI2112345600000785",
        "IBAN should be stripped of whitespace"
    );
}
