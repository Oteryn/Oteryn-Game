//! Capture an observed Animate Dead spawn for the session replay test.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(600),
        verdict::observe(async {
            let directory = required("E2E_ALL_CAPTURE_DIR")?;
            full_e2ef5::capture_animate_dead(clients, world, |files| {
                for (name, bytes) in files {
                    std::fs::write(Path::new(&directory).join(name), bytes)?;
                }
                Ok(())
            }).await
        }),
    ).await;
    match result {
        Ok(Ok(())) => evidence("e2e_all scenario=C16c.client status=PASS evidence=live-animate-dead-replay"),
        Ok(Err(error)) => evidence(&format!("e2e_all scenario=C16c.client status=FAIL evidence={error}")),
        Err(_) => evidence("e2e_all scenario=C16c.client status=FAIL evidence=timeout"),
    }
}
