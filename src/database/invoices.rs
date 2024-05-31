use super::DatabaseConnection;
use crate::api::invoices::{CreateInvoice, PopulatedInvoice};
use crate::error::Error;
use crate::models::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

impl DatabaseConnection {
    /// create an address, returning an id of the address, either
    pub async fn create_address(&mut self, address: &NewAddress) -> Result<i32, Error> {
        use crate::schema::addresses::dsl::*;

        diesel::insert_into(addresses)
            .values(address)
            .on_conflict(diesel::upsert::on_constraint("no_duplicates"))
            .do_nothing()
            .execute(&mut self.0)
            .await?;

        // NOTE: Diesel is dumb so we have to requery for the data
        // because on_conflict() doesn't support returning()
        Ok(addresses
            .select(id)
            .filter(
                street
                    .eq(&address.street)
                    .and(city.eq(&address.city))
                    .and(zip.eq(&address.zip)),
            )
            .first::<i32>(&mut self.0)
            .await?)
    }

    pub async fn create_invoice(
        &mut self,
        invoice: CreateInvoice,
    ) -> Result<PopulatedInvoice, Error> {
        let address_id = self.create_address(&invoice.address).await?;

        // TODO: this could (but should it is totally another question) be done with an impl for CreateInvoice,
        // as this is the only thing CreateInvoice is used for
        let inv = NewInvoice {
            address_id,
            recipient_name: invoice.recipient_name,
            recipient_email: invoice.recipient_email,
            bank_account_number: invoice.bank_account_number,
        };

        let created_invoice = {
            use crate::schema::invoices::dsl::*;
            diesel::insert_into(invoices)
                .values(&inv)
                .returning(invoices::all_columns())
                .get_result::<Invoice>(&mut self.0)
                .await?
        };

        let rows = {
            use crate::schema::invoice_rows::dsl::*;
            diesel::insert_into(invoice_rows)
                .values(
                    &invoice
                        .rows
                        .into_iter()
                        .map(|r| NewInvoiceRow {
                            invoice_id: created_invoice.id,
                            product: r.product,
                            quantity: r.quantity,
                            unit: r.unit,
                            unit_price: r.unit_price,
                        })
                        .collect::<Vec<_>>(),
                )
                .returning(invoice_rows::all_columns())
                .get_results::<InvoiceRow>(&mut self.0)
                .await?
        };

        let attachments = {
            use crate::schema::invoice_attachments::dsl::*;
            diesel::insert_into(invoice_attachments)
                .values(
                    &invoice
                        .attachments
                        .into_iter()
                        .map(|a| NewAttachment {
                            invoice_id: created_invoice.id,
                            hash: a.hash,
                            filename: a.filename,
                        })
                        .collect::<Vec<_>>(),
                )
                .returning(invoice_attachments::all_columns())
                .get_results::<Attachment>(&mut self.0)
                .await?
        };

        Ok(created_invoice.into_populated(rows, attachments))
    }

    pub async fn list_invoices(&mut self) -> Result<Vec<PopulatedInvoice>, Error> {
        use crate::schema::invoices;
        let invoices = invoices::table
            .select(Invoice::as_select())
            .load(&mut self.0)
            .await?;

        let invoice_rows: Vec<Vec<_>> = InvoiceRow::belonging_to(&invoices)
            .select(InvoiceRow::as_select())
            .load(&mut self.0)
            .await?
            .grouped_by(&invoices);
        let attachments: Vec<Vec<_>> = Attachment::belonging_to(&invoices)
            .select(Attachment::as_select())
            .load(&mut self.0)
            .await?
            .grouped_by(&invoices);
        Ok(invoice_rows
            .into_iter()
            .zip(attachments)
            .zip(invoices)
            .map(|((rows, attachments), invoice)| invoice.into_populated(rows, attachments))
            .collect::<Vec<PopulatedInvoice>>())
    }

    pub async fn get_invoice(&mut self, invoice_id: i32) -> Result<PopulatedInvoice, Error> {
        use crate::schema::invoices::dsl::invoices;

        let invoice = invoices
            .find(invoice_id)
            .first::<Invoice>(&mut self.0)
            .await?;

        let attachments = Attachment::belonging_to(&invoice).load(&mut self.0).await?;

        let rows = InvoiceRow::belonging_to(&invoice).load(&mut self.0).await?;

        Ok(invoice.into_populated(rows, attachments))
    }
}
