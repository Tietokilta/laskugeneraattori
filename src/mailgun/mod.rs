use crate::state::State;
use axum::{
    extract::{FromRef, OptionalFromRequestParts},
    http::request::Parts,
};

mod invoices;

#[derive(Clone, Debug)]
pub struct MailgunClient {
    client: reqwest::Client,
    url: String,
    api_user: String,
    api_key: String,
    default_to: String,
    from: String,
}

impl TryFrom<crate::MailgunConfig> for MailgunClient {
    type Error = String;

    fn try_from(config: crate::MailgunConfig) -> Result<Self, Self::Error> {
        if !config.disable {
            Ok(Self {
                url: config.url.ok_or("mailgun URL is not configured")?,
                api_user: config.user.ok_or("mailgun user is not configured")?,
                api_key: config
                    .password
                    .ok_or("mailgun password is not configured")?,
                default_to: config.to.ok_or("mailgun 'to' address is not configured")?,
                from: config
                    .from
                    .ok_or("mailgun 'from' address is not configured")?,
                client: reqwest::Client::new(),
            })
        } else {
            Err("mailgun is disabled".to_string())
        }
    }
}

impl<S> OptionalFromRequestParts<S> for MailgunClient
where
    S: Send + Sync,
    State: FromRef<S>,
{
    type Rejection = crate::error::Error;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let state = State::from_ref(state);
        Ok(state.mailgun_client)
    }
}
