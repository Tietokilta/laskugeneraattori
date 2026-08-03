use crate::error::Error;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, PoisonError, RwLock};
use std::time::{Duration, Instant};

/// A toimikunta and the accounting account its invoices are booked against, as the CMS returns
/// it. Any other field of the document (its id, timestamps) is ignored.
///
/// The list is maintained in the CMS (the `cost-pools` collection), so that adding or renaming
/// a toimikunta needs no deploy of this service.
#[derive(Clone, Debug, Deserialize)]
pub struct CostPool {
    /// The human-readable name of the toimikunta
    pub name: String,
    /// The four-digit accounting account the payment is routed to
    pub account: String,
}

/// Used when we cannot tell which toimikunta the invoice belongs to: the client did not pick one
/// or the CMS is unreachable. The account deliberately does not exist in the bookkeeping, so that
/// the payment cannot be booked by accident and the treasurer has to assign it by hand.
///
/// Hardcoded on purpose: this is the fallback for the CMS being broken, so it must not depend
/// on the CMS.
pub static UNASSIGNED: LazyLock<CostPool> = LazyLock::new(|| CostPool {
    name: "KOHDISTAMATON – toimikunta puuttuu".into(),
    account: "4999".into(),
});

/// The same fallback for a toimikunta we know the name of but cannot book against. Naming it
/// tells the treasurer which CMS document to have fixed.
fn unassigned_but_named(pool: &CostPool) -> CostPool {
    CostPool {
        name: format!(
            "KOHDISTAMATON – {} (virheellinen tili {})",
            pool.name, pool.account
        ),
        account: UNASSIGNED.account.clone(),
    }
}

/// An account must be exactly four digits and must not start with a zero: banking systems strip
/// leading zeros, which would shift every digit of the reference number and silently route the
/// payment to the wrong account. The CMS validates this as well, this is the safety net.
fn is_valid_account(account: &str) -> bool {
    account.len() == 4
        && account.starts_with(|c: char| c.is_ascii_digit() && c != '0')
        && account.chars().all(|c| c.is_ascii_digit())
}

/// How long a resolved cost pool is served from memory before it is fetched again. Cost pools
/// change rarely – a toimikunta is renamed or moved to another account once a year at most – so
/// keeping them for an hour takes the CMS round trip off the invoice request in practice, while
/// an edit in the CMS still reaches us without a restart.
const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

/// A pool the CMS does not have is the client's problem and is reported back to it, anything
/// else is ours and must not cost anyone their invoice.
#[derive(Debug)]
enum FetchError {
    NotFound,
    Unavailable(String),
}

/// Reads cost pools from the CMS, keeping the ones it has seen in memory.
///
/// Cloning shares the cache: the client lives in the axum state and is cloned per request.
#[derive(Clone, Debug)]
pub struct CostPoolClient {
    client: reqwest::Client,
    base_url: reqwest::Url,
    /// The pools seen so far, by id, each with the instant it goes stale
    cache: Arc<RwLock<HashMap<String, (CostPool, Instant)>>>,
}

impl CostPoolClient {
    pub fn new(base_url: reqwest::Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            cache: Arc::default(),
        }
    }

    fn cached(&self, id: &str) -> Option<CostPool> {
        let cache = self.cache.read().unwrap_or_else(PoisonError::into_inner);
        let (pool, stale_at) = cache.get(id)?;

        (Instant::now() < *stale_at).then(|| pool.clone())
    }

    fn cache(&self, id: &str, pool: &CostPool) {
        self.cache
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id.to_string(), (pool.clone(), Instant::now() + CACHE_TTL));
    }

    async fn fetch(&self, id: &str) -> Result<CostPool, FetchError> {
        // Appending the segments rather than formatting the URL keeps the base working with or
        // without a trailing slash, and keeps the id out of the rest of the URL
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| {
                FetchError::Unavailable(format!("{} cannot be a base url", self.base_url))
            })?
            .pop_if_empty()
            .extend(["api", "cost-pools", id]);
        url.set_query(Some("depth=0"));

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchError::Unavailable(format!("request to the CMS failed: {e}")))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(FetchError::NotFound);
        }

        if !response.status().is_success() {
            return Err(FetchError::Unavailable(format!(
                "the CMS returned {}",
                response.status()
            )));
        }

        response
            .json::<CostPool>()
            .await
            .map_err(|e| FetchError::Unavailable(format!("could not parse the CMS response: {e}")))
    }

    /// Resolves the cost pool an invoice is booked against. Only an id the CMS does not know is
    /// an error: our own trouble with the CMS falls back to [`UNASSIGNED`] rather than costing
    /// someone their invoice, and the treasurer assigns those by hand.
    ///
    /// Only a cache miss reaches the CMS. Nothing that fell back is cached, so the next invoice
    /// retries.
    pub async fn resolve(&self, id: Option<&str>) -> Result<CostPool, Error> {
        let Some(id) = id else {
            return Ok(UNASSIGNED.clone());
        };

        if let Some(pool) = self.cached(id) {
            return Ok(pool);
        }

        match self.fetch(id).await {
            Ok(pool) if is_valid_account(&pool.account) => {
                self.cache(id, &pool);
                Ok(pool)
            }
            // The CMS validates the account too, so this is a should-never-happen. Loud, but not
            // the filer's fault and not something they could fix by trying again.
            Ok(pool) => {
                error!(
                    "cost pool {id} ({}) has the invalid account {:?}, booking the invoice as unassigned",
                    pool.name, pool.account
                );
                Ok(unassigned_but_named(&pool))
            }
            Err(FetchError::NotFound) => Err(Error::UnknownCostPool(id.to_string())),
            Err(FetchError::Unavailable(e)) => {
                warn!("could not resolve cost pool {id}: {e}, booking the invoice as unassigned");
                Ok(UNASSIGNED.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn accepts_a_well_formed_account() {
        assert!(is_valid_account("4212"));
    }

    #[test]
    fn rejects_accounts_that_would_corrupt_the_reference() {
        // A leading zero is stripped by banking systems
        assert!(!is_valid_account("0123"));
        assert!(!is_valid_account("421"));
        assert!(!is_valid_account("42120"));
        assert!(!is_valid_account("4a12"));
        assert!(!is_valid_account(""));
    }

    #[tokio::test]
    async fn no_cost_pool_resolves_to_unassigned() {
        let client = CostPoolClient::new("http://localhost:1".parse().unwrap());
        assert_eq!(
            client.resolve(None).await.unwrap().account,
            UNASSIGNED.account
        );
    }

    #[tokio::test]
    async fn an_unreachable_cms_resolves_to_unassigned() {
        let client = CostPoolClient::new("http://localhost:1".parse().unwrap());
        let pool = client
            .resolve(Some("507f1f77bcf86cd799439011"))
            .await
            .unwrap();
        assert_eq!(pool.account, UNASSIGNED.account);
    }

    const ID: &str = "507f1f77bcf86cd799439011";

    fn cost_pool_response(account: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": ID,
            "name": "Liikuntatoimikunta",
            "account": account,
        }))
    }

    #[tokio::test]
    async fn a_known_cost_pool_is_fetched_once() {
        let cms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(cost_pool_response("4212"))
            .expect(1)
            .mount(&cms)
            .await;

        let client = CostPoolClient::new(cms.uri().parse().unwrap());
        assert_eq!(client.resolve(Some(ID)).await.unwrap().account, "4212");
        assert_eq!(client.resolve(Some(ID)).await.unwrap().account, "4212");
        // The clone shares the cache with the client it was cloned from
        assert_eq!(
            client.clone().resolve(Some(ID)).await.unwrap().name,
            "Liikuntatoimikunta"
        );

        // Dropping the mock server asserts the expectation
    }

    #[tokio::test]
    async fn a_base_url_with_a_trailing_slash_hits_the_same_path() {
        let cms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(cost_pool_response("4212"))
            .expect(1)
            .mount(&cms)
            .await;

        let client = CostPoolClient::new(format!("{}/", cms.uri()).parse().unwrap());
        assert_eq!(client.resolve(Some(ID)).await.unwrap().account, "4212");
    }

    #[tokio::test]
    async fn a_pool_that_resolved_to_unassigned_is_fetched_again() {
        let cms = MockServer::start().await;
        // The CMS is broken for the first request and healthy afterwards
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&cms)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(cost_pool_response("4212"))
            .mount(&cms)
            .await;

        let client = CostPoolClient::new(cms.uri().parse().unwrap());
        assert_eq!(
            client.resolve(Some(ID)).await.unwrap().account,
            UNASSIGNED.account
        );
        assert_eq!(client.resolve(Some(ID)).await.unwrap().account, "4212");
    }

    #[tokio::test]
    async fn an_invalid_account_is_not_cached() {
        let cms = MockServer::start().await;
        // A leading zero never makes it into the cache, so fixing it in the CMS takes effect
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(cost_pool_response("0212"))
            .up_to_n_times(1)
            .mount(&cms)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(cost_pool_response("4212"))
            .mount(&cms)
            .await;

        let client = CostPoolClient::new(cms.uri().parse().unwrap());
        let pool = client.resolve(Some(ID)).await.unwrap();
        assert_eq!(pool.account, UNASSIGNED.account);
        // The treasurer's email says which toimikunta it was and what was wrong with it
        assert_eq!(
            pool.name,
            "KOHDISTAMATON – Liikuntatoimikunta (virheellinen tili 0212)"
        );

        assert_eq!(client.resolve(Some(ID)).await.unwrap().account, "4212");
    }

    #[tokio::test]
    async fn a_cost_pool_the_cms_does_not_know_is_rejected() {
        let cms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(ResponseTemplate::new(404))
            .mount(&cms)
            .await;

        let client = CostPoolClient::new(cms.uri().parse().unwrap());
        assert!(matches!(
            client.resolve(Some(ID)).await,
            Err(Error::UnknownCostPool(_))
        ));
    }

    #[tokio::test]
    async fn a_stale_cost_pool_is_fetched_again() {
        let cms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/cost-pools/{ID}")))
            .respond_with(cost_pool_response("4212"))
            .expect(2)
            .mount(&cms)
            .await;

        let client = CostPoolClient::new(cms.uri().parse().unwrap());
        client.resolve(Some(ID)).await.unwrap();

        // Expire the entry instead of waiting an hour for it to go stale on its own
        client.cache.write().unwrap().get_mut(ID).unwrap().1 = Instant::now();

        client.resolve(Some(ID)).await.unwrap();
    }
}
