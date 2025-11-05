use std::net::IpAddr;

use tower_governor::{
    key_extractor::{KeyExtractor, PeerIpKeyExtractor},
    GovernorError,
};

#[derive(Clone)]
pub enum IpExtractor {
    HeaderKeyExtractor { header_name: &'static str },
    PeerIpKeyExtractor,
}

impl IpExtractor {
    pub fn header_extractor(header_name: &'static str) -> Self {
        Self::HeaderKeyExtractor { header_name }
    }
}

impl KeyExtractor for IpExtractor {
    type Key = IpAddr;

    fn extract<T>(
        &self,
        req: &axum::http::Request<T>,
    ) -> Result<Self::Key, tower_governor::GovernorError> {
        match self {
            IpExtractor::HeaderKeyExtractor { header_name } => req
                .headers()
                .get(*header_name)
                .and_then(|hv| hv.to_str().ok())
                .and_then(|s| s.parse::<IpAddr>().ok())
                .ok_or(GovernorError::UnableToExtractKey),

            IpExtractor::PeerIpKeyExtractor => PeerIpKeyExtractor {}.extract(req),
        }
    }
}
