//! The full migration chain preserves Corruptor and ordinary corpse timings.
use crate::bestiary_postgres_harness as f;

#[test]
fn qmech_corpse_policy_preserves_corruptor_and_source_durations() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let h = f::Harness::create(admin, "qmech_corpse_policy", false).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "QMECH corpse policy worker failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    for (root, chain, durations) in [
        (
            52113,
            vec![52113, 52114, 52115, 52116],
            vec![600_000_i64, 600_000, 10_000, 600_000],
        ),
        (
            54678,
            vec![54678, 54679, 54680, 54681],
            vec![10_000, 300_000, 300_000, 60_000],
        ),
        (
            33859,
            vec![33859, 33858, 33857, 33856],
            vec![10_000, 900_000, 600_000, 600_000],
        ),
    ] {
        let key = format!("oteryn:item.tibia.i{root}");
        let actual: (Vec<i32>, Vec<i64>) = sqlx::query_as(
            "SELECT game_summer_corpse_chain($1), game_source_corpse_durations_v1($1)",
        )
        .bind(&key)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(actual, (chain, durations), "source corpse {root}");
    }
    Ok(())
}
