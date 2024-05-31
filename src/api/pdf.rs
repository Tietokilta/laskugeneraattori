use axum::{
    body::Body,
    extract::Path,
    http::{header, Response},
};
use typst::model::Document;

use crate::database::DatabaseConnection;
use crate::error::Error;

pub async fn pdf(
    mut conn: DatabaseConnection,
    Path(id): Path<i32>,
) -> Result<Response<Body>, Error> {
    let invoice = conn.get_invoice(id).await?;
    let document: Document = invoice.try_into()?;
    let pdf = typst_pdf::pdf(&document, typst::foundations::Smart::Auto, None);

    Ok(Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "application/pdf")
        .body(Body::from(pdf))
        .unwrap())
}
