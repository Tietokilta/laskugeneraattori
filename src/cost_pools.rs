use serde_derive::Deserialize;
use std::sync::LazyLock;

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

/// Used when we cannot tell which toimikunta the invoice belongs to: the client did not pick
/// one, the CMS is unreachable, or the cost pool it picked no longer exists. The account
/// deliberately does not exist in the bookkeeping, so that the payment cannot be booked by
/// accident and the treasurer has to assign it by hand.
///
/// Hardcoded on purpose: this is the fallback for the CMS being broken, so it must not depend
/// on the CMS.
pub static UNASSIGNED: LazyLock<CostPool> = LazyLock::new(|| CostPool {
    name: "KOHDISTAMATON – toimikunta puuttuu".into(),
    account: "4999".into(),
});

/// An account must be exactly four digits and must not start with a zero: banking systems strip
/// leading zeros, which would shift every digit of the reference number and silently route the
/// payment to the wrong account. The CMS validates this as well, this is the safety net.
fn is_valid_account(account: &str) -> bool {
    account.len() == 4
        && account.starts_with(|c: char| c.is_ascii_digit() && c != '0')
        && account.chars().all(|c| c.is_ascii_digit())
}

/// Reads cost pools from the CMS.
#[derive(Clone, Debug)]
pub struct CostPoolClient {
    client: reqwest::Client,
    base_url: String,
}

impl CostPoolClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    async fn fetch(&self, id: &str) -> Result<CostPool, String> {
        let url = format!("{}/api/cost-pools/{id}?depth=0", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("request to the CMS failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("the CMS returned {}", response.status()));
        }

        response
            .json::<CostPool>()
            .await
            .map_err(|e| format!("could not parse the CMS response: {e}"))
    }

    /// Resolves the cost pool an invoice is booked against.
    ///
    /// Falls back to [`UNASSIGNED`] whenever the cost pool cannot be established, so that a
    /// broken CMS never stops anyone from filing an invoice: the treasurer sees account 4999
    /// and assigns the payment by hand.
    pub async fn resolve(&self, id: Option<&str>) -> CostPool {
        let Some(id) = id else {
            return UNASSIGNED.clone();
        };

        match self.fetch(id).await {
            Ok(pool) if is_valid_account(&pool.account) => pool,
            Ok(pool) => {
                warn!(
                    "cost pool {id} has the invalid account {:?}, booking the invoice as unassigned",
                    pool.account
                );
                UNASSIGNED.clone()
            }
            Err(e) => {
                warn!("could not resolve cost pool {id}: {e}, booking the invoice as unassigned");
                UNASSIGNED.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let client = CostPoolClient::new("http://localhost:1".into());
        assert_eq!(client.resolve(None).await.account, UNASSIGNED.account);
    }

    #[tokio::test]
    async fn an_unreachable_cms_resolves_to_unassigned() {
        let client = CostPoolClient::new("http://localhost:1".into());
        let pool = client.resolve(Some("507f1f77bcf86cd799439011")).await;
        assert_eq!(pool.account, UNASSIGNED.account);
    }
}
