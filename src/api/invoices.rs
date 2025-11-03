use std::sync::LazyLock;

use crate::error::Error;
use crate::mailgun::MailgunClient;

use axum::{body::Bytes, http::StatusCode};
use axum_typed_multipart::{
    FieldData, FieldMetadata, TryFromChunks, TryFromMultipart, TypedMultipart, TypedMultipartError,
};

use axum_valid::Garde;
use futures::stream::Stream;
use garde::Validate;
use iban::Iban;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;

static ALLOWED_FILENAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(jpg|jpeg|png|gif|svg|pdf)$").unwrap());

#[axum_typed_multipart::async_trait]
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

/// An address consisting of a street, a city and a zipcode
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct Address {
    /// The recipient's street address, maximum length of 128 characters
    #[garde(length(chars, max = 128))]
    pub street: String,
    /// The recipient's city, maximum length of 128 characters
    #[garde(length(chars, max = 128))]
    pub city: String,
    /// The recipient's zip code, maximum length of 128 characters
    #[garde(length(chars, max = 128))]
    pub zip: String,
}

/// Body for the request for creating new invoices
#[derive(Clone, Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct Invoice {
    /// The recipient's name, maximum length of 128 characters
    #[garde(length(chars, max = 128))]
    pub recipient_name: String,
    /// The recipient's email, maximum length of 128 characters
    #[garde(length(chars, max = 128))]
    pub recipient_email: String,
    /// The recipient's address
    #[garde(dive)]
    pub address: Address,
    /// The recipient's bank account number, must be a valid iban bank account number
    #[garde(length(chars, max = 128), custom(is_valid_iban))]
    pub bank_account_number: String,
    /// The subject of the invoice, at least 1 character and at most 128 characters long
    #[garde(length(chars, min = 1, max = 128))]
    pub subject: String,
    /// The description of the invoice, maximum length of 128 characters
    #[garde(length(chars, max = 4096))]
    pub description: String,
    /// The recipient's phone number, maximum length of 32 characters, must be valid and include
    /// the counter prefix (e.g. +358)
    #[garde(phone_number, length(chars, max = 32))]
    pub phone_number: String,
    /// A list of descriptions for the attached files, each with the maximum length of 512
    /// characters
    #[garde(inner(length(chars, max = 512)))]
    pub attachment_descriptions: Vec<String>,
    /// The rows of the invoice
    #[garde(length(min = 1), dive)]
    pub rows: Vec<InvoiceRow>,
    // NOTE: We get the attachments from the multipart form
    #[garde(skip)]
    #[serde(skip_deserializing)]
    pub attachments: Vec<InvoiceAttachment>,
}

#[derive(TryFromMultipart, Validate, ToSchema)]
pub struct InvoiceForm {
    /// The JSON data of the invoice
    #[garde(dive)]
    pub data: Invoice,
    /// The attachments of the invoice
    // FIXME: Maybe use NamedTempFile
    #[garde(skip)]
    #[form_data(limit = "unlimited")]
    #[schema(value_type = Vec<u8>)]
    pub attachments: Vec<FieldData<Bytes>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct InvoiceRow {
    /// The product can be at most 128 characters
    #[garde(length(chars, max = 128))]
    pub product: String,
    /// Unit price is encoded as number of cents to avoid floating-point precision bugs
    /// must be positive
    #[garde(range(min = 1))]
    pub unit_price: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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

/// Creates an invoice with the given data and attachments and sends it by email to the treasurer
#[utoipa::path(post, path = "/invoices", 
    request_body(content_type = "multipart/form-data", content = InvoiceForm), 
    responses(
        (status = 201, body = Invoice)
    )
)]
pub async fn create(
    client: Option<MailgunClient>,
    Garde(TypedMultipart(mut multipart)): Garde<TypedMultipart<InvoiceForm>>,
) -> Result<(StatusCode, axum::Json<Invoice>), Error> {
    use crate::pdfgen::DocumentBuilder;

    let attachments: Vec<InvoiceAttachment> =
        Result::from_iter(multipart.attachments.into_iter().map(try_handle_file))?;

    multipart.data.attachments = attachments
        .iter()
        .map(|a| InvoiceAttachment {
            filename: a.filename.clone(),
            bytes: vec![],
        })
        .collect();

    let inner_data = multipart.data.clone();

    // PDF compilation is heavily blocking
    let pdf = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        let (document, attached_pdfs) =
            DocumentBuilder::new(inner_data, attachments).build_with_pdfs()?;

        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).unwrap();

        let mut pdfs = vec![pdf];
        pdfs.extend_from_slice(
            attached_pdfs
                .into_iter()
                .map(|a| a.bytes)
                .collect::<Vec<_>>()
                .as_slice(),
        );

        let pdf = crate::merge::merge_pdf(pdfs)?;
        Ok(pdf)
    })
    .await??;

    if let Some(client) = client {
        client.send_mail(&multipart.data, pdf).await?;
    } else {
        use tempfile::NamedTempFile;
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;

        let tmp = NamedTempFile::with_suffix(".pdf")?;
        let (file, path) = tmp.keep().unwrap();
        let mut file = File::from_std(file);
        file.write_all(&pdf).await?;

        info!("Wrote invoice to {:?}", path);
    }

    Ok((StatusCode::CREATED, axum::Json(multipart.data)))
}
