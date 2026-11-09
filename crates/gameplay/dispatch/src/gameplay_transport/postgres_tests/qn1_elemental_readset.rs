//! M49's database admission predicate is independent of the live Rust map proof.
use crate::bestiary_postgres_harness as f;

#[test]
fn qn1_elemental_readset_postgres_rejects_incomplete_and_substituted_rings() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "qn1_elemental_readset", false).await?;
                    let version: String =
                        sqlx::query_scalar("SELECT current_setting('server_version')")
                            .fetch_one(&h.pool)
                            .await?;
                    eprintln!("QN1_ELEMENTAL_PG_READY PostgreSQL {version}");
                    let result = sqlx::raw_sql(include_str!("qn1_elemental_readset.sql"))
                        .execute(&h.pool)
                        .await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!("QN1_ELEMENTAL_PG_EXECUTED duration={:?}", started.elapsed());
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "Elemental Spheres PG case thread failed")?
        .map_err(Into::into)
}
