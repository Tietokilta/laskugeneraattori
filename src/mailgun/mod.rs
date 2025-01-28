use crate::state::State;
use axum::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};

mod invoices;

#[derive(Clone)]
pub enum Mailer {
    Mailgun(MailgunClient),
    Debug,
}

#[derive(Clone, Debug)]
pub struct MailgunClient {
    client: reqwest::Client,
    url: String,
    api_user: String,
    api_key: String,
    default_to: String,
    from: String,
}

impl From<crate::MailerConfig> for Mailer {
    fn from(config: crate::MailerConfig) -> Self {
        match config.disable {
            true => Self::Debug,
            false => Self::Mailgun(MailgunClient {
                client: reqwest::Client::new(),
                url: config.url.expect("must exist if not disabled"),
                api_user: config.user.expect("must exist if not disabled"),
                api_key: config.password.expect("must exist if not disabled"),
                default_to: config.to.expect("must exist if not disabled"),
                from: config.from.expect("must exist if not disabled"),
            }),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Mailer
where
    S: Send + Sync,
    State: FromRef<S>,
{
    type Rejection = crate::error::Error;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = State::from_ref(state);
        Ok(state.mailer)
    }
}
