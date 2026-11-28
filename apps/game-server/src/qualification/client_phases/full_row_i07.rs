//! Independent indexed I07 server journey, including ammunition custody.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(capture_journey(clients)).await;
    evidence(&format!(
        "e2e_all scenario=I07.full status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
    ));
    if let Err(error) = result {
        evidence(&format!("I07_FULL_FAILURE {error}"));
    }
}

async fn capture_journey(clients: &SeamClients<'_>) -> TestResult {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("I07.full requires a fresh E2E_ALL_CAPTURE_DIR")?;
    let directory = Path::new(&root).join("i07-full");
    std::fs::create_dir(&directory)?;
    client_row_i07::journey(clients, &directory, true).await
}
