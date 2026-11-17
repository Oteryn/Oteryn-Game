//! Native boat storage guard through the production World USE owner.
use crate::bestiary_postgres_harness as f;

#[test]
fn qstore_real_boat_storage_gate_refuses_then_passes_dispatch_postgres() -> f::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create_with_admission_seed(
                        admin,
                        "qstore_real_boat",
                        true,
                        false,
                    )
                    .await?;
                    let result = super::engnpc2_paint_transport::exercise_qstore(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "QSTORE_REAL_GATE_POSTGRES_EXECUTED duration={:?}",
                        started.elapsed()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "QSTORE real gate thread failed")?
        .map_err(Into::into)
}
