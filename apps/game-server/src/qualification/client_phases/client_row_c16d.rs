//! Client replay capture for SCENARIO_MATRIX C16d.

use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>, _world: &BundleWorld) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    // Capture this admission only; full F5 journeys retain their own dispatch and verdicts.
    let started = std::time::Instant::now();
    let result = verdict::observe(capture(clients)).await;
    let (status, detail) = match result {
        Ok(()) => ("PASS", "admission-snapshot-captured".to_owned()),
        Err(error) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C16d.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xd6).await?;
    let files = client
        .session_mut()
        .c16d_capture_files()
        .ok_or("C16d admission snapshot or replay context absent")?;
    let directory = required("E2E_ALL_CAPTURE_DIR")?;
    for (name, bytes) in files {
        std::fs::write(Path::new(&directory).join(name), bytes)?;
    }
    Ok(())
}
