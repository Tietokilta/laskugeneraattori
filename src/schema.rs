// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "invoice_status"))]
    pub struct InvoiceStatus;
}

diesel::table! {
    addresses (id) {
        id -> Int4,
        #[max_length = 128]
        street -> Varchar,
        #[max_length = 128]
        city -> Varchar,
        #[max_length = 128]
        zip -> Varchar,
    }
}

diesel::table! {
    invoice_attachments (id) {
        id -> Int4,
        invoice_id -> Int4,
        #[max_length = 128]
        filename -> Varchar,
        #[max_length = 64]
        hash -> Varchar,
    }
}

diesel::table! {
    invoice_rows (id) {
        id -> Int4,
        invoice_id -> Int4,
        #[max_length = 128]
        product -> Varchar,
        quantity -> Int4,
        #[max_length = 128]
        unit -> Varchar,
        unit_price -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::InvoiceStatus;

    invoices (id) {
        id -> Int4,
        status -> InvoiceStatus,
        creation_time -> Timestamptz,
        #[max_length = 128]
        recipient_name -> Varchar,
        #[max_length = 128]
        recipient_email -> Varchar,
        #[max_length = 128]
        bank_account_number -> Varchar,
        address_id -> Int4,
        #[max_length = 128]
        subject -> Varchar,
        #[max_length = 512]
        description -> Varchar,
        #[max_length = 32]
        phone_number -> Varchar,
    }
}

diesel::joinable!(invoice_attachments -> invoices (invoice_id));
diesel::joinable!(invoice_rows -> invoices (invoice_id));
diesel::joinable!(invoices -> addresses (address_id));

diesel::allow_tables_to_appear_in_same_query!(
    addresses,
    invoice_attachments,
    invoice_rows,
    invoices,
);
