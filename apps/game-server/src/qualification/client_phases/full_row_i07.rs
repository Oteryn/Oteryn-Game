//! Independent indexed I07 server journey, including ammunition custody.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(
        Duration::from_secs(45),
        verdict::observe(client_row_i07::journey(clients, None)),
    )
    .await
    .unwrap_or_else(|_| Err("I07 full verification timed out".into()));
    evidence(&format!(
        "e2e_all scenario=I07.full status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
    ));
    if let Err(error) = result {
        evidence(&format!("I07_FULL_FAILURE {error}"));
    }
}
