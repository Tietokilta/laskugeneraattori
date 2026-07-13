use crate::cost_pools::{COST_POOLS, CostPool};

/// Lists the toimikunnat an invoice can be booked against
#[utoipa::path(get, path = "/cost-pools", responses((status = 200, body = Vec<CostPool>)))]
pub async fn list() -> axum::Json<&'static Vec<CostPool>> {
    axum::Json(&COST_POOLS)
}
