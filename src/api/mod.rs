use axum::{extract::DefaultBodyLimit, routing::get, Router};
use tower_http::{limit::RequestBodyLimitLayer, trace::TraceLayer};

pub mod invoices;

pub fn app() -> Router<crate::database::State> {
    Router::new()
        .route("/health", get(health))
        .route("/invoices", get(invoices::list_all).post(invoices::create))
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(
            250 * 1024 * 1024, /* 250mb */
        ))
}

async fn health() {}
