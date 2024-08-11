use crate::state::State;
use axum::async_trait;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use diesel_async::{pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection};

mod invoices;

pub struct DatabaseConnection(
    pub bb8::PooledConnection<'static, AsyncDieselConnectionManager<AsyncPgConnection>>,
);

#[async_trait]
impl<S> FromRequestParts<S> for DatabaseConnection
where
    S: Send + Sync,
    State: FromRef<S>,
{
    type Rejection = crate::error::Error;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = State::from_ref(state);
        let conn = state.pool.get_owned().await?;

        Ok(Self(conn))
    }
}
