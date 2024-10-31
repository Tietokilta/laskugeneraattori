use std::sync::LazyLock;

use crate::error::Error;
#[cfg(feature = "email")]
use crate::mailgun::MailgunClient;

use axum::{async_trait, body::Bytes, http::StatusCode};
use axum_typed_multipart::{
    FieldData, FieldMetadata, TryFromChunks, TryFromMultipart, TypedMultipart, TypedMultipartError,
};

use axum_valid::Garde;
use futures::stream::Stream;
use garde::Validate;
use iban::Iban;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};

use typst::model::Document;

static ALLOWED_FILENAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(jpg|jpeg|png|gif|svg|pdf)$").unwrap());

#[async_trait]
impl TryFromChunks for Invoice {
    async fn try_from_chunks(
        chunks: impl Stream<Item = Result<Bytes, TypedMultipartError>> + Send + Sync + Unpin,
        metadata: FieldMetadata,
    ) -> Result<Self, TypedMultipartError> {
        let bytes = Bytes::try_from_chunks(chunks, metadata).await?;

        serde_json::from_slice(&bytes).map_err(|e| TypedMultipartError::Other { source: e.into() })
    }
}

fn is_valid_iban(value: &str, _: &()) -> garde::Result {
    match value.parse::<Iban>() {
        Err(e) => Err(garde::Error::new(e)),
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Address {
    #[garde(byte_length(max = 128))]
    pub street: String,
    #[garde(byte_length(max = 128))]
    pub city: String,
    #[garde(byte_length(max = 128))]
    pub zip: String,
}

/// Body for the request for creating new invoices
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct Invoice {
    /// The recipient's name
    #[garde(byte_length(max = 128))]
    pub recipient_name: String,
    /// The recipient's email
    #[garde(byte_length(max = 128))]
    pub recipient_email: String,
    /// The recipient's address
    #[garde(dive)]
    pub address: Address,
    /// The recipient's bank account number
    // TODO: maybe validate with https://crates.io/crates/iban_validate/
    #[garde(byte_length(max = 128), custom(is_valid_iban))]
    pub bank_account_number: String,
    #[garde(byte_length(min = 1, max = 128))]
    pub subject: String,
    #[garde(byte_length(max = 4096))]
    pub description: String,
    #[garde(phone_number, byte_length(max = 32))]
    pub phone_number: String,
    #[garde(inner(byte_length(max = 512)))]
    pub attachment_descriptions: Vec<String>,
    /// The rows of the invoice
    #[garde(length(min = 1), dive)]
    pub rows: Vec<InvoiceRow>,
    // NOTE: We get the attachments from the multipart form
    #[garde(skip)]
    #[serde(skip_deserializing)]
    pub attachments: Vec<InvoiceAttachment>,
}

#[derive(TryFromMultipart, Validate)]
pub struct InvoiceForm {
    #[garde(dive)]
    pub data: Invoice,
    // FIXME: Maybe use NamedTempFile
    #[garde(skip)]
    #[form_data(limit = "unlimited")]
    pub attachments: Vec<FieldData<Bytes>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct InvoiceRow {
    /// The product can be at most 128 characters
    #[garde(byte_length(max = 128))]
    pub product: String,
    /// The quantity of the product, must be positive
    #[garde(range(min = 1))]
    pub quantity: i32,
    /// The unit can be at most 128 characters
    #[garde(byte_length(max = 128))]
    pub unit: String,
    /// Unit price is encoded as number of cents to avoid floating-point precision bugs
    /// must be positive
    #[garde(range(min = 1))]
    pub unit_price: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvoiceAttachment {
    pub filename: String,
    pub bytes: Vec<u8>,
}

fn try_handle_file(field: FieldData<Bytes>) -> Result<InvoiceAttachment, Error> {
    let filename = field
        .metadata
        .file_name
        .as_ref()
        .ok_or(Error::MissingFilename)?
        .to_string();

    if !ALLOWED_FILENAME.is_match(&filename) {
        return Err(Error::UnsupportedFileFormat(filename));
    }

    Ok(InvoiceAttachment {
        filename,
        bytes: field.contents.to_vec(),
    })
}

#[cfg(feature = "email")]
pub async fn create_email(
    client: MailgunClient,
    Garde(TypedMultipart(mut multipart)): Garde<TypedMultipart<InvoiceForm>>,
) -> Result<(StatusCode, axum::Json<Invoice>), Error> {
    let orig = multipart.data.clone();
    multipart.data.attachments =
        Result::from_iter(multipart.attachments.into_iter().map(try_handle_file))?;

    let document: Document = multipart.data.to_owned().try_into()?;
    let pdf = typst_pdf::pdf(&document, typst::foundations::Smart::Auto, None);

    let mut pdfs = vec![pdf];
    pdfs.extend_from_slice(
        multipart
            .data
            .attachments
            .into_iter()
            .map(|a| a.bytes)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let pdf = crate::merge::merge_pdf(pdfs)?;

    client.send_mail(&orig, pdf).await?;
    Ok((StatusCode::CREATED, axum::Json(orig)))
}

#[cfg(not(feature = "email"))]
pub async fn create(
    Garde(TypedMultipart(mut multipart)): Garde<TypedMultipart<InvoiceForm>>,
) -> Result<axum::response::Response, Error> {
    use tempfile::NamedTempFile;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    multipart.data.attachments =
        Result::from_iter(multipart.attachments.into_iter().map(try_handle_file))?;

    let document: Document = multipart.data.to_owned().try_into()?;
    let pdf = typst_pdf::pdf(&document, typst::foundations::Smart::Auto, None);

    let mut pdfs = vec![pdf];
    pdfs.extend_from_slice(
        multipart
            .data
            .attachments
            .into_iter()
            .map(|a| a.bytes)
            .collect::<Vec<_>>()
            .as_slice(),
    );

    let pdf = crate::merge::merge_pdf(pdfs)?;

    let tmp = NamedTempFile::with_suffix(".pdf")?;
    let (file, path) = tmp.keep().unwrap();
    let mut file = File::from_std(file);
    file.write_all(&pdf).await?;

    info!("Wrote invoice to {:?}", path);

    Ok(axum::response::Response::builder()
        .status(StatusCode::CREATED)
        .header("Content-Type", "application/pdf")
        .body(Bytes::from(pdf).into())
        .unwrap())
}
