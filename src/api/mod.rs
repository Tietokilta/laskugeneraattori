use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, Request},
    Router,
};
use std::sync::Arc;
use std::time::Duration;
use tower_governor::{governor::GovernorConfigBuilder, key_extractor::KeyExtractor, GovernorLayer};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};
use utoipa::openapi::{ContactBuilder, InfoBuilder, OpenApiBuilder};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

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

    // Customize OpenAPI info
    let (router, api) = OpenApiRouter::with_openapi(
        OpenApiBuilder::new()
            .info(
                InfoBuilder::new()
                    .title("Laskugeneraattori API")
                    .version(env!("CARGO_PKG_VERSION"))
                    .description(Some("Invoice generation service for Tietokilta"))
                    .contact(Some(
                        ContactBuilder::new()
                            .name(Some("Tietokilta"))
                            .url(Some(
                                "https://github.com/Tietokilta/laskugeneraattori/issues",
                            ))
                            .build(),
                    ))
                    .build(),
            )
            .build(),
    )
    .routes(routes!(health, invoices::create))
    .split_for_parts();

    Router::new()
        .merge(router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", api))
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

/// Checks the health of the service and returns build information
#[utoipa::path(get, path = "/health", responses((status = 200, body = String)))]
async fn health() -> String {
    format!(
        "Laskugeneraattori {} {}",
        &env!("CARGO_PKG_VERSION"),
        &env!("COMMIT_HASH")
    )
}
