#[cfg(feature = "email")]
use crate::mailgun::MailgunClient;

use axum::extract::FromRef;

#[derive(FromRef, Clone)]
pub struct State {
    #[cfg(feature = "email")]
    pub mailgun_client: MailgunClient,
    pub for_garde: (),
}

pub async fn new() -> State {
    dotenv::dotenv().ok();

    State {
        #[cfg(feature = "email")]
        mailgun_client: MailgunClient::from(crate::CONFIG.mailgun.clone()),
        for_garde: (),
    }
}
