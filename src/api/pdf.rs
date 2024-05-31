use axum::{
    body::Body,
    extract::Path,
    http::{header, Response},
    response::IntoResponse,
};
use typst::model::Document;

use crate::database::DatabaseConnection;

pub async fn pdf(mut conn: DatabaseConnection, Path(id): Path<i32>) -> impl IntoResponse {
    let invoice = conn.get_invoice(id).await.unwrap();
    let document: Document = invoice.try_into().unwrap();
    let pdf = typst_pdf::pdf(&document, typst::foundations::Smart::Auto, None);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "application/pdf")
        .body(Body::from(pdf))
        .unwrap()
}
