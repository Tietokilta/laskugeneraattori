use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use diesel_async::pooled_connection::PoolError;
use serde_derive::Serialize;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Bb8 run error")]
    PoolError(#[from] bb8::RunError<PoolError>),
    #[error("Diesel error {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("Error while parsing multipart form")]
    MultipartError(#[from] axum::extract::multipart::MultipartError),
    #[error("Error in handling multipart request")]
    MultipartRejection(#[from] axum::extract::multipart::MultipartRejection),
    #[error("Missing filename multipart")]
    MissingFilename,
    #[error("Error in handling json value")]
    JsonRejection(#[from] axum::extract::rejection::JsonRejection),
    #[error("Error while parsing json")]
    JsonError(#[from] serde_json::Error),
    #[error("Internal server error")]
    InternalServerError(#[from] std::io::Error),
    #[error("Invoice not found")]
    InvoiceNotFound,
    #[cfg(feature = "pdfgen")]
    #[error("Typst error")]
    TypstError,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            error: String,
        }

        error!(%self);

        let status = match self {
            Error::PoolError(_) | Error::DieselError(_) | Error::InternalServerError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Error::JsonError(_)
            | Error::MissingFilename
            | Error::MultipartError(_)
            | Error::MultipartRejection(_)
            | Error::JsonRejection(_) => StatusCode::BAD_REQUEST,
            Error::InvoiceNotFound => StatusCode::NOT_FOUND,
            #[cfg(feature = "pdfgen")]
            Error::TypstError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status,
            axum::Json(ErrorResponse {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}
