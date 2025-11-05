use std::net::IpAddr;

use tower_governor::{
    key_extractor::{KeyExtractor, PeerIpKeyExtractor},
    GovernorError,
};

#[derive(Copy, Clone)]
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
                .and_then(|s| s.trim().parse().ok())
                .ok_or(GovernorError::UnableToExtractKey),

            IpExtractor::PeerIpKeyExtractor => PeerIpKeyExtractor {}.extract(req),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn header_extractor_with_valid_ipv4() {
        let extractor = IpExtractor::header_extractor("x-forwarded-for");
        let req = Request::builder()
            .header("x-forwarded-for", "192.168.1.1")
            .body(())
            .unwrap();

        let result = extractor.extract(&req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "192.168.1.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn header_extractor_with_valid_ipv6() {
        let extractor = IpExtractor::header_extractor("x-real-ip");
        let req = Request::builder()
            .header("x-real-ip", "2001:0db8::1")
            .body(())
            .unwrap();

        let result = extractor.extract(&req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2001:0db8::1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn header_extractor_trims_whitespace() {
        let extractor = IpExtractor::header_extractor("x-forwarded-for");
        let req = Request::builder()
            .header("x-forwarded-for", "  192.168.1.1  ")
            .body(())
            .unwrap();

        let result = extractor.extract(&req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "192.168.1.1".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn header_extractor_with_invalid_ip() {
        let extractor = IpExtractor::header_extractor("x-forwarded-for");
        let req = Request::builder()
            .header("x-forwarded-for", "not-an-ip")
            .body(())
            .unwrap();

        let result = extractor.extract(&req);
        assert!(result.is_err());
        assert!(matches!(result, Err(GovernorError::UnableToExtractKey)));
    }

    #[test]
    fn header_extractor_with_missing_header() {
        let extractor = IpExtractor::header_extractor("x-forwarded-for");
        let req = Request::builder().body(()).unwrap();

        let result = extractor.extract(&req);
        assert!(result.is_err());
        assert!(matches!(result, Err(GovernorError::UnableToExtractKey)));
    }

    #[test]
    fn header_extractor_with_empty_header() {
        let extractor = IpExtractor::header_extractor("x-forwarded-for");
        let req = Request::builder()
            .header("x-forwarded-for", "")
            .body(())
            .unwrap();

        let result = extractor.extract(&req);
        assert!(result.is_err());
        assert!(matches!(result, Err(GovernorError::UnableToExtractKey)));
    }

    #[test]
    fn ip_extractor_implements_copy() {
        let extractor = IpExtractor::header_extractor("x-forwarded-for");
        let _extractor_copy = extractor;
        let _extractor_again = extractor;
    }
}
