use crate::error::Error;
use axum::body::Bytes;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum_typed_multipart::{
    FieldMetadata, TryFromChunks, TryFromMultipart, TypedMultipart, TypedMultipartError,
};
use axum_valid::Garde;
use futures::Stream;
use garde::Validate;
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;

#[axum_typed_multipart::async_trait]
impl TryFromChunks for Receipt {
    async fn try_from_chunks(
        chunks: impl Stream<Item = Result<Bytes, TypedMultipartError>> + Send + Sync + Unpin,
        metadata: FieldMetadata,
    ) -> Result<Self, TypedMultipartError> {
        let bytes = Bytes::try_from_chunks(chunks, metadata).await?;

        serde_json::from_slice(&bytes).map_err(|e| TypedMultipartError::Other { source: e.into() })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct Receipt {
    #[garde(length(chars, max = 128))]
    pub receipt_number: String,
    /// Name of the purchaser, maximum length of 128 characters
    #[garde(length(chars, max = 128))]
    pub purchaser_name: String,
    /// Email of the purchaser, maximum length of 128 characters
    #[garde(length(chars, max = 256))]
    pub purchaser_email: String,
    /// The rows of products in the receipt
    #[garde(length(min = 1), dive)]
    pub rows: Vec<ReceiptRow>,
}

#[derive(TryFromMultipart, Validate, ToSchema)]
pub struct ReceiptForm {
    /// The JSON data of the receipt
    #[garde(dive)]
    pub data: Receipt,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct ReceiptRow {
    /// The product can be at most 128 characters
    #[garde(length(chars, max = 128))]
    pub product: String,
    /// The value added tax of the product in ‰ (1/10th of a %)
    #[garde(range(min = 0))]
    pub vat: i32,
    /// Unit price is encoded as number of cents to avoid floating-point precision bugs
    /// must be positive
    #[garde(range(min = 1))]
    pub unit_price: i32,
}

#[utoipa::path(post, path = "/receipts",
    request_body(content_type = "multipart/form-data", content = ReceiptForm),
    responses(
        (status = 201, description = "Receipt PDF", content_type = "application/pdf")
    )
)]
pub async fn create_receipt(
    Garde(TypedMultipart(multipart)): Garde<TypedMultipart<ReceiptForm>>,
) -> Result<Response, Error> {
    use crate::pdfgen::ReceiptBuilder;

    let inner_data = multipart.data.clone();

    // PDF compilation is heavily blocking
    let pdf = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        let document = ReceiptBuilder::new(inner_data).build_with_pdfs()?;

        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).unwrap();

        Ok(pdf)
    })
    .await??;

    Ok((
        StatusCode::CREATED,
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"receipt.pdf\"",
            ),
        ],
        pdf,
    )
        .into_response())
}
