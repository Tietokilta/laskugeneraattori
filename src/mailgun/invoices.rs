use crate::api::invoices::Invoice;
use crate::error::Error;

use super::Mailer;

impl Mailer {
    pub async fn send_mail(self, invoice: &Invoice, pdf: Vec<u8>) -> Result<(), Error> {
        match self {
            Mailer::Mailgun(mailgun_client) => {
                let invoice_recipient =
                    format!("{} <{}>", invoice.recipient_name, invoice.recipient_email);
                let form = reqwest::multipart::Form::new()
                    .text("from", mailgun_client.from)
                    .text("to", mailgun_client.default_to)
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

                let response = mailgun_client
                    .client
                    .post(mailgun_client.url)
                    .basic_auth(mailgun_client.api_user, Some(mailgun_client.api_key))
                    .multipart(form)
                    .send()
                    .await?;

                match response.error_for_status() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(Error::ReqwestError(e)),
                }
            }
            Mailer::Debug => {
                info!("Would send invoice: {}", serde_json::to_string(invoice)?);
                Ok(())
            }
        }
    }
}
