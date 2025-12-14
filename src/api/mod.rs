use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, Request},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::KeyExtractor, GovernorLayer};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};

use crate::{api::key_extractor::IpExtractor, CONFIG};

pub mod invoices;
mod key_extractor;

pub fn app() -> Router<crate::state::State> {
    let cors_layer = CorsLayer::new().allow_origin(
        crate::CONFIG
            .allowed_origins
            .iter()
            .map(|c| c.parse::<HeaderValue>().unwrap())
            .collect::<Vec<_>>(),
    );

    let extractor = CONFIG
        .ip_extractor_header
        .as_ref()
        .map(|ip_header| IpExtractor::header_extractor(ip_header))
        .unwrap_or(IpExtractor::PeerIpKeyExtractor);

    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .const_period(Duration::from_secs(CONFIG.rate_limit_period_secs))
            .burst_size(CONFIG.rate_limit_burst_size)
            .use_headers()
            .methods(vec![Method::POST])
            .key_extractor(extractor)
            .finish()
            .unwrap(),
    );
    let governor_limiter = governor_config.limiter().clone();

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        governor_limiter.retain_recent();
    });

    Router::new()
        .route("/health", get(health))
        .route("/invoices", post(invoices::create))
        .layer(cors_layer)
        .layer(DefaultBodyLimit::disable())
        // Limit the body to 24 MiB since the email is limited to 25 MiB
        .layer(RequestBodyLimitLayer::new(24 * 1024 * 1024))
        .layer(GovernorLayer::new(governor_config))
        .layer(
            TraceLayer::new_for_http().make_span_with(move |req: &Request<_>| {
                let ip = extractor
                    .extract(req)
                    .map(|k| k.to_string())
                    .unwrap_or_else(|_| "unknown ip".into());

                info_span!(
                    "http_request",
                    method = ?req.method(),
                    uri = ?req.uri(),
                    ip = %ip,
                )
            }),
        )
}

async fn health() -> String {
    format!(
        "Laskugeneraattori {} {}",
        &env!("CARGO_PKG_VERSION"),
        &env!("COMMIT_HASH")
    )
}
