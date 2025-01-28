use crate::mailgun::Mailer;

use axum::extract::FromRef;

#[derive(FromRef, Clone)]
pub struct State {
    pub mailer: Mailer,
    pub for_garde: (),
}

pub async fn new() -> State {
    dotenv::dotenv().ok();

    State {
        mailer: Mailer::from(crate::CONFIG.mailgun.clone()),
        for_garde: (),
    }
}
