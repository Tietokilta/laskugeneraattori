use crate::{
    api::invoices::PopulatedInvoice,
    schema::{addresses, invoice_attachments, invoice_rows, invoices},
};
use chrono::{DateTime, Utc};
use garde::Validate;

use diesel::prelude::*;
use serde_derive::{Deserialize, Serialize};

// NOTES:
// This is implemented based on https://github.com/Tietokilta/laskugeneraattori/blob/main/backend/src/procountor.rs#L293
//
// InvoiceRow: Is VAT really necessary to account for? TODO(?)
// Invoice: due date TODO:
//   this is not prio 1, but implementation idea:
//   when creating new invoice is set to NULL, as we don't know if the invoice is valid
//   Set to date X when the treasurer accepts this invoice
//   the open question is: What is the date X? Have to coordinate with treasurer / is this even necessary
// - Having a massive enum for product units is necessary, just have it as string :D
// As one recipient has to be able to:
// - have different emails
// - have different addresses
// - have different bank account for payment
// - have different names even? ¯\_(ツ)_/¯
// It would involve creating a quite overcomplicated schema with multiple many-to-many relations for not that much of benefit
// as this is anyways a free fill form where people can spam whatever information.
// if there were to be any stronger authentication (guild auth?) then this would change (read: not in the near future.)
// Invoices table with recipient name, email and account number

#[derive(diesel_derive_enum::DbEnum, Debug, Clone, Copy, Serialize, Deserialize)]
#[ExistingTypePath = "crate::schema::sql_types::InvoiceStatus"]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceStatus {
    Open,
    Accepted,
    Paid,
    Cancelled,
}

#[derive(Identifiable, Queryable, Selectable, Clone, Debug, Serialize, Deserialize)]
#[diesel(table_name = addresses)]
pub struct Address {
    pub id: i32,
    /// The street address can be at most 128 characters
    pub street: String,
    /// The city can be at most 128 characters
    pub city: String,
    /// The zipcode can be at most 128 characters (:D)
    pub zip: String,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize, Validate)]
#[diesel(table_name=addresses)]
pub struct NewAddress {
    #[garde(byte_length(max = 128))]
    pub street: String,
    #[garde(byte_length(max = 128))]
    pub city: String,
    #[garde(byte_length(max = 128))]
    pub zip: String,
}

/// The invoice model as stored in the database
#[derive(Identifiable, Queryable, Selectable, Associations, Clone, Debug)]
#[diesel(belongs_to(Address))]
#[diesel(table_name = invoices)]
pub struct Invoice {
    pub id: i32,
    pub status: InvoiceStatus,
    pub creation_time: DateTime<Utc>,
    // TODO: see NOTES above
    // pub due_date:Option<NaiveDate>,
    /// invoice recipient's name can be at most 128 characters
    pub recipient_name: String,
    /// invoice recipient's email can be at most 128 characters
    pub recipient_email: String,
    /// A back account number can be at most 128 characters
    pub bank_account_number: String,
    pub address_id: i32,
    /// The subject of the invoice, has to be between 1 and 128 characters long
    pub subject: String,
    /// The description of the invoice, containing e.g. reasons for the purchase
    /// can be at most 512 characters
    pub description: String,
    /// invoice recipient's phone number. Can be at most 32 characters.
    /// Must be a valid phone number
    pub phone_number: String,
}

impl Invoice {
    pub fn into_populated(
        self,
        address: Address,
        rows: Vec<InvoiceRow>,
        attachments: Vec<Attachment>,
    ) -> PopulatedInvoice {
        PopulatedInvoice::new(self, address, rows, attachments)
    }
}

#[derive(Insertable)]
#[diesel(table_name = invoices)]
pub struct NewInvoice {
    pub address_id: i32,
    pub recipient_name: String,
    pub recipient_email: String,
    pub bank_account_number: String,
    pub subject: String,
    pub description: String,
    pub phone_number: String,
}

/// A single row of an invoice
#[derive(
    Identifiable, Queryable, Selectable, Associations, Clone, Debug, Serialize, Deserialize,
)]
#[diesel(belongs_to(Invoice))]
#[diesel(table_name = invoice_rows)]
pub struct InvoiceRow {
    #[serde(skip_serializing, skip_deserializing)]
    pub id: i32,
    #[serde(skip_serializing, skip_deserializing)]
    pub invoice_id: i32,
    /// The product can be at most 128 characters
    pub product: String,
    pub quantity: i32,
    /// The unit can be at most 128 characters
    pub unit: String,
    /// Unit price is encoded as number of cents to avoid floating-point precision bugs
    pub unit_price: i32,
}

#[derive(Insertable)]
#[diesel(table_name = invoice_rows)]
pub struct NewInvoiceRow {
    pub invoice_id: i32,
    pub product: String,
    pub quantity: i32,
    pub unit: String,
    pub unit_price: i32,
}

/// The metadata for an invoice attachment
/// The file itself can be requested using its hash and filename
/// => /somepath/{hash}/{filename}
#[derive(
    Identifiable, Queryable, Selectable, Associations, Clone, Debug, Serialize, Deserialize,
)]
#[diesel(belongs_to(Invoice))]
#[diesel(table_name = invoice_attachments)]
pub struct Attachment {
    #[serde(skip_serializing, skip_deserializing)]
    pub id: i32,
    #[serde(skip_serializing, skip_deserializing)]
    pub invoice_id: i32,
    /// The filename can be at most 128 characters
    pub filename: String,
    /// The SHA256 hash of the file contents as a hex string (64 characters)
    pub hash: String,
}

#[derive(Insertable)]
#[diesel(table_name = invoice_attachments)]
pub struct NewAttachment {
    pub invoice_id: i32,
    pub filename: String,
    pub hash: String,
}
