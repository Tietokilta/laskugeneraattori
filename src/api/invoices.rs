use crate::database::DatabaseConnection;
use crate::error::Error;
use crate::models::{Address, Attachment, Invoice, InvoiceRow};
use axum::{async_trait, body::Bytes, http::StatusCode, Json};
use axum_typed_multipart::{
    FieldData, FieldMetadata, TryFromChunks, TryFromMultipart, TypedMultipart, TypedMultipartError,
};
use axum_valid::Garde;
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use futures::stream::{self, TryStreamExt};
use garde::Validate;
use serde_derive::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

#[async_trait]
impl TryFromChunks for CreateInvoice {
    async fn try_from_chunks(
        chunks: impl Stream<Item = Result<Bytes, TypedMultipartError>> + Send + Sync + Unpin,
        metadata: FieldMetadata,
    ) -> Result<Self, TypedMultipartError> {
        let bytes = Bytes::try_from_chunks(chunks, metadata).await?;

        serde_json::from_slice(&bytes).map_err(|e| TypedMultipartError::Other { source: e.into() })
    }
}

/// Body for the request for creating new invoices
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct CreateInvoice {
    /// The recipient's name
    #[garde(byte_length(max = 128))]
    pub recipient_name: String,
    /// The recipient's email
    #[garde(byte_length(max = 128))]
    pub recipient_email: String,
    /// The recipient's address
    #[garde(dive)]
    pub address: crate::models::NewAddress,
    /// The recipient's bank account number
    // TODO: maybe validate with https://crates.io/crates/iban_validate/
    #[garde(byte_length(max = 128))]
    pub bank_account_number: String,
    #[garde(byte_length(min = 1, max = 128))]
    pub subject: String,
    #[garde(byte_length(max = 512))]
    pub description: String,
    #[garde(phone_number, byte_length(max = 32))]
    pub phone_number: String,
    #[garde(inner(byte_length(max = 512)))]
    pub attachment_descriptions: Vec<String>,
    /// The rows of the invoice
    #[garde(length(min = 1), dive)]
    pub rows: Vec<CreateInvoiceRow>,
    // NOTE: We get the attachments from the multipart form
    #[garde(skip)]
    #[serde(skip_deserializing)]
    pub attachments: Vec<CreateInvoiceAttachment>,
}

#[derive(TryFromMultipart, Validate)]
pub struct CreateInvoiceForm {
    #[garde(dive)]
    pub data: CreateInvoice,
    // FIXME: Maybe use NamedTempFile
    #[garde(skip)]
    #[form_data(limit = "unlimited")]
    pub attachments: Vec<FieldData<Bytes>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct CreateInvoiceRow {
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
pub struct CreateInvoiceAttachment {
    pub filename: String,
    pub hash: String,
}

/// A populated invoice type that is returned to the user
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PopulatedInvoice {
    pub id: i32,
    pub status: crate::models::InvoiceStatus,
    pub creation_time: DateTime<Utc>,
    pub recipient_name: String,
    pub recipient_email: String,
    pub bank_account_number: String,
    pub phone_number: String,
    pub subject: String,
    pub description: String,
    pub address: Address,
    pub rows: Vec<InvoiceRow>,
    pub attachments: Vec<Attachment>,
}

impl PopulatedInvoice {
    pub fn new(
        invoice: Invoice,
        address: Address,
        rows: Vec<InvoiceRow>,
        attachments: Vec<Attachment>,
    ) -> Self {
        Self {
            id: invoice.id,
            status: invoice.status,
            creation_time: invoice.creation_time,
            recipient_name: invoice.recipient_name,
            recipient_email: invoice.recipient_email,
            phone_number: invoice.phone_number,
            subject: invoice.subject,
            description: invoice.description,
            address,
            bank_account_number: invoice.bank_account_number,
            rows,
            attachments,
        }
    }
}

async fn try_handle_file(field: &FieldData<Bytes>) -> Result<CreateInvoiceAttachment, Error> {
    let filename = field
        .metadata
        .file_name
        .as_ref()
        .ok_or(Error::MissingFilename)?
        .to_string();

    let cont = field.contents.clone();
    let hash = tokio::task::spawn_blocking(move || hex::encode(Sha256::digest(&cont)))
        .await
        .unwrap();

    let file_path = format!(
        "{}/{hash}",
        std::env::var("ATTACHMENT_PATH").unwrap_or(String::from("."))
    );

    if tokio::fs::File::open(&file_path).await.is_err() {
        let mut file = tokio::fs::File::create(&file_path).await?;

        // FIXME: Properly handle write error
        // as failing here should remove the created file
        file.write_all(&field.contents).await?;
    } else {
        debug!("Skipping duplicate file: {hash}")
    }

    Ok(CreateInvoiceAttachment {
        filename: filename.to_string(),
        hash,
    })
}

pub async fn create(
    mut conn: DatabaseConnection,
    Garde(TypedMultipart(mut multipart)): Garde<TypedMultipart<CreateInvoiceForm>>,
) -> Result<(StatusCode, Json<PopulatedInvoice>), Error> {
    multipart.data.attachments = stream::iter(
        multipart
            .attachments
            .iter()
            .map(try_handle_file)
            .map(Ok)
            // NOTE: This collect might seem harmless but
            // I dare you to try removing it
            .collect::<Vec<_>>(),
    )
    // FIXME: Don't hardcode buffer size
    .try_buffer_unordered(50)
    .try_collect::<Vec<_>>()
    .await?;

    Ok((
        StatusCode::CREATED,
        axum::Json(conn.create_invoice(multipart.data.clone()).await?),
    ))
}

pub async fn list_all(mut conn: DatabaseConnection) -> Result<Json<Vec<PopulatedInvoice>>, Error> {
    Ok(axum::Json(conn.list_invoices().await?))
}

#[allow(dead_code)]
pub async fn send_mail(invoice: &PopulatedInvoice) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .text(
            "from",
            std::env::var("MAILGUN_FROM").unwrap_or(String::from("")),
        )
        .text(
            "to",
            format!("{} <{}>", invoice.recipient_name, invoice.recipient_email),
        )
        .text(
            "to",
            std::env::var("MAILGUN_TO").unwrap_or(String::from("")),
        )
        .text("subject", format!("Uusi lasku #{}", invoice.id))
        .text("html", format!("Uusi lasku #{}", invoice.id));

    let form = invoice
        .attachments
        .iter()
        .try_fold(form, |form, attachment| {
            let path = std::env::var("ATTACHMENT_PATH").unwrap_or(String::from("."));
            let path = std::path::Path::new(&path).join(&attachment.hash);
            dbg!(&path);
            let bytes = std::fs::read(path)?;
            Ok::<reqwest::multipart::Form, Error>(form.part(
                "attachment",
                reqwest::multipart::Part::bytes(bytes).file_name(attachment.filename.clone()),
            ))
        })?;

    let response = client
        .post(std::env::var("MAILGUN_URL").unwrap_or(String::from("")))
        .basic_auth(
            std::env::var("MAILGUN_USER").unwrap_or(String::from("")),
            Some(std::env::var("MAILGUN_PASSWORD").unwrap_or(String::from(""))),
        )
        .multipart(form)
        .send()
        .await?;

    match response.error_for_status() {
        Ok(_) => Ok(()),
        Err(e) => Err(Error::ReqwestError(e)),
    }
}
