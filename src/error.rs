use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use serde_derive::Serialize;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "email")]
    #[error("Reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Error while parsing multipart form")]
    MultipartError(#[from] axum::extract::multipart::MultipartError),
    #[error("Error in handling multipart request")]
    MultipartRejection(#[from] axum::extract::multipart::MultipartRejection),
    #[error("Missing filename multipart")]
    MissingFilename,
    #[error("Unsupported file format: {0}. Supported file formats are (jpg|jpeg|png|gif|svg|pdf)")]
    UnsupportedFileFormat(String),
    #[error("Error in handling json value")]
    JsonRejection(#[from] axum::extract::rejection::JsonRejection),
    #[error("Error while parsing json")]
    JsonError(#[from] serde_json::Error),
    #[error("Internal server error")]
    InternalServerError(#[from] std::io::Error),
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
            Error::InternalServerError(_) | Error::TypstError => StatusCode::INTERNAL_SERVER_ERROR,
            #[cfg(feature = "email")]
            Error::ReqwestError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::JsonError(_)
            | Error::MissingFilename
            | Error::MultipartError(_)
            | Error::MultipartRejection(_)
            | Error::JsonRejection(_)
            | Error::UnsupportedFileFormat(_) => StatusCode::BAD_REQUEST,
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
