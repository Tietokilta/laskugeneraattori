use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;
use utoipa::ToSchema;

/// A toimikunta and the accounting account its invoices are booked against
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CostPool {
    /// The identifier the client sends in the invoice
    pub id: String,
    /// The human-readable name of the toimikunta
    pub name: String,
    /// The four-digit accounting account the payment is routed to
    pub account: String,
}

#[derive(Deserialize)]
struct CostPoolFile {
    cost_pool: Vec<CostPool>,
}

/// Used when the client does not tell us which toimikunta the invoice belongs to, e.g. because
/// it is an older frontend that does not know about cost pools yet. The account deliberately
/// does not exist in the bookkeeping, so that the payment cannot be booked by accident and the
/// treasurer has to assign it by hand.
pub static UNASSIGNED: LazyLock<CostPool> = LazyLock::new(|| CostPool {
    id: "unassigned".into(),
    name: "KOHDISTAMATON – toimikunta puuttuu".into(),
    account: "4999".into(),
});

pub static COST_POOLS: LazyLock<Vec<CostPool>> = LazyLock::new(|| {
    let file: CostPoolFile = toml::from_str(include_str!("../cost_pools.toml"))
        .expect("BUG: cost_pools.toml is not valid TOML");

    let mut seen = HashSet::new();
    for pool in &file.cost_pool {
        // A leading zero would be stripped by banking systems and shift the account
        // digits out of the reference number
        assert!(
            pool.account.len() == 4
                && pool
                    .account
                    .starts_with(|c: char| c.is_ascii_digit() && c != '0')
                && pool.account.chars().all(|c| c.is_ascii_digit()),
            "cost pool {}: account must be four digits and must not start with a zero, got {:?}",
            pool.id,
            pool.account
        );
        assert!(
            pool.account != UNASSIGNED.account,
            "cost pool {} uses account {}, which is reserved for unassigned invoices",
            pool.id,
            UNASSIGNED.account
        );
        assert!(
            seen.insert(pool.id.as_str()),
            "cost pool {} is defined twice",
            pool.id
        );
    }

    file.cost_pool
});

pub fn get(id: &str) -> Option<&'static CostPool> {
    COST_POOLS.iter().find(|pool| pool.id == id)
}

/// Resolves the cost pool an invoice is booked against, falling back to [`UNASSIGNED`] when the
/// client did not pick one. An unknown id cannot reach this point, garde rejects it first.
pub fn resolve(id: Option<&str>) -> &'static CostPool {
    match id {
        Some(id) => get(id).expect("BUG: cost pool validated by garde"),
        None => &UNASSIGNED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_pool_list_is_valid() {
        // The LazyLock asserts on every entry
        assert!(!COST_POOLS.is_empty());
    }

    #[test]
    fn lookup_finds_a_known_pool() {
        assert_eq!(get("liikuntatoimikunta").unwrap().account, "4212");
        assert!(get("ei-olemassa").is_none());
    }
}
