use super::{TEST_IP, TEST_IP_HEADER};
use axum::body::Body;
use axum::http::Request;
use serde_json::Value;
use std::path::Path;

pub fn load_test_file(filename: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(filename);
    std::fs::read(&path).unwrap_or_else(|_| panic!("Failed to read test file: {}", filename))
}

pub struct MultipartBuilder {
    boundary: String,
    parts: Vec<Vec<u8>>,
}

impl MultipartBuilder {
    pub fn new() -> Self {
        Self {
            boundary: "----TestBoundary7MA4YWxkTrZu0gW".to_string(),
            parts: Vec::new(),
        }
    }

    pub fn add_json_field(mut self, name: &str, value: &Value) -> Self {
        let mut part = Vec::new();
        part.extend_from_slice(format!("--{}\r\n", self.boundary).as_bytes());
        part.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
        );
        part.extend_from_slice(b"Content-Type: application/json\r\n\r\n");
        part.extend_from_slice(value.to_string().as_bytes());
        part.extend_from_slice(b"\r\n");
        self.parts.push(part);
        self
    }

    pub fn add_file(mut self, field_name: &str, filename: &str, content: Vec<u8>) -> Self {
        let content_type = match filename
            .rsplit('.')
            .next()
            .map(|s| s.to_lowercase())
            .as_deref()
        {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("pdf") => "application/pdf",
            _ => "application/octet-stream",
        };

        let mut part = Vec::new();
        part.extend_from_slice(format!("--{}\r\n", self.boundary).as_bytes());
        part.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                field_name, filename
            )
            .as_bytes(),
        );
        part.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
        part.extend_from_slice(&content);
        part.extend_from_slice(b"\r\n");
        self.parts.push(part);
        self
    }

    pub fn add_file_without_filename(mut self, field_name: &str, content: Vec<u8>) -> Self {
        let mut part = Vec::new();
        part.extend_from_slice(format!("--{}\r\n", self.boundary).as_bytes());
        part.extend_from_slice(
            format!(
                "Content-Disposition: form-data; name=\"{}\"\r\n",
                field_name
            )
            .as_bytes(),
        );
        part.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
        part.extend_from_slice(&content);
        part.extend_from_slice(b"\r\n");
        self.parts.push(part);
        self
    }

    pub fn add_raw_field(mut self, name: &str, content_type: &str, content: &[u8]) -> Self {
        let mut part = Vec::new();
        part.extend_from_slice(format!("--{}\r\n", self.boundary).as_bytes());
        part.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes(),
        );
        part.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
        part.extend_from_slice(content);
        part.extend_from_slice(b"\r\n");
        self.parts.push(part);
        self
    }

    pub fn build(self) -> (String, Vec<u8>) {
        let mut body = Vec::new();
        for part in self.parts {
            body.extend_from_slice(&part);
        }
        body.extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());

        let content_type = format!("multipart/form-data; boundary={}", self.boundary);
        (content_type, body)
    }

    pub fn into_request(self, uri: &str) -> Request<Body> {
        let (content_type, body) = self.build();
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", content_type)
            .header(TEST_IP_HEADER, TEST_IP)
            .body(Body::from(body))
            .unwrap()
    }
}

impl Default for MultipartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_invoice_request(invoice: &Value) -> Request<Body> {
    MultipartBuilder::new()
        .add_json_field("data", invoice)
        .into_request("/invoices")
}

pub fn create_invoice_request_with_file(
    invoice: &Value,
    filename: &str,
    content: Vec<u8>,
) -> Request<Body> {
    MultipartBuilder::new()
        .add_json_field("data", invoice)
        .add_file("attachments", filename, content)
        .into_request("/invoices")
}

pub fn create_invoice_request_with_files(
    invoice: &Value,
    files: Vec<(&str, Vec<u8>)>,
) -> Request<Body> {
    let mut builder = MultipartBuilder::new().add_json_field("data", invoice);
    for (filename, content) in files {
        builder = builder.add_file("attachments", filename, content);
    }
    builder.into_request("/invoices")
}
