use crate::mailgun::MailgunClient;

use axum::extract::FromRef;

#[derive(FromRef, Clone)]
pub struct State {
    pub mailgun_client: Option<MailgunClient>,
    pub for_garde: (),
}

pub async fn new() -> State {
    dotenv::dotenv().ok();

    State {
        mailgun_client: match crate::CONFIG.mailgun.clone().try_into() {
            Err(e) if !crate::CONFIG.mailgun.disable => {
                panic!("failed to initialize mailgun client: {e}")
            }
            res => res.ok(),
        },
        for_garde: (),
    }
}
