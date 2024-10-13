use super::MailgunClient;
use crate::api::invoices::Invoice;
use crate::error::Error;

impl MailgunClient {
    pub async fn send_mail(self, invoice: &Invoice, pdf: Vec<u8>) -> Result<(), Error> {
        let invoice_recipient = format!("{} <{}>", invoice.recipient_name, invoice.recipient_email);
        let form = reqwest::multipart::Form::new()
            .text("from", self.from)
            .text("to", self.default_to)
            .text("cc", invoice_recipient)
            .text(
                "subject",
                format!("Uusi lasku, lähettäjä {}", invoice.recipient_name),
            )
            .text(
                "html",
                format!("Uusi lasku, lähettäjä {}", invoice.recipient_name),
            )
            .part(
                "attachment",
                reqwest::multipart::Part::bytes(pdf).file_name("invoice.pdf"),
            );

        let response = self
            .client
            .post(self.url)
            .basic_auth(self.api_user, Some(self.api_key))
            .multipart(form)
            .send()
            .await?;

        match response.error_for_status() {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::ReqwestError(e)),
        }
    }
}
