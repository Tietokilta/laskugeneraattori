use napi::bindgen_prelude::*;
use napi_derive::napi;

use garde::Validate;

#[napi]
pub fn create_receipt_pdf(receipt_json: String) -> Result<Buffer> {
    // Parse JSON into your existing Receipt type
    let receipt: laskugeneraattori::api::receipts::Receipt =
        serde_json::from_str(&receipt_json)
            .map_err(|e| Error::from_reason(format!("Invalid JSON: {e}")))?;

    // Enforce the same validation rules as your API (lengths, ranges, min rows, etc.)
    receipt
        .validate(&())
        .map_err(|e| Error::from_reason(format!("Validation error: {e}")))?;

    // Build the Typst document and compile to PDF
    let document = laskugeneraattori::pdfgen::ReceiptBuilder::new(receipt)
        .build()
        .map_err(|e| Error::from_reason(format!("Failed to build receipt: {e}")))?;

    let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|e| Error::from_reason(format!("Failed to render PDF: {e}")))?;

    Ok(Buffer::from(pdf_bytes))
}