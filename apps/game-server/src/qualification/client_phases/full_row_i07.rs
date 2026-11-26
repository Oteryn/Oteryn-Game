//! Independent indexed I07 server journey, including ammunition custody.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(45),
        verdict::observe(client_row_i07::journey(clients, None)),
    )
    .await
    .unwrap_or_else(|_| Err("I07 full verification timed out".into()));
    evidence(&format!(
        "e2e_all scenario=I07.full status={}",
        if result.is_ok() { "PASS" } else { "FAIL" }
    ));
    if let Err(error) = result {
        evidence(&format!("I07_FULL_FAILURE {error}"));
    }
}
