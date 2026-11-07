//! SRX-3: source policy must reach the existing durable corpse-chain owner.
//! Crystal 201189f data/items/items.xml:73328-73347: 39530→39529→39528→39527,
//! with 10s/300s/300s stages and a 60s terminal corpse (the existing 670s owner).
use super::TestResult;
use super::fixture::{Harness, run};

#[test]
fn srx3_primal_corpse_source_chain_policy_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-primal-corpse", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "srx3_primal_corpse", false, false)
                .await?;
            let chain: Option<Vec<i32>> =
                sqlx::query_scalar("SELECT game_summer_corpse_chain('oteryn:item.tibia.i39530')")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                chain,
                Some(vec![39530, 39529, 39528, 39527]),
                "the source-qualified Primal corpse must install its real decay chain"
            );
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_PRIMAL_CORPSE_POLICY_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_w11_corpse_source_chain_policy_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("srx3-w11-corpse", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "srx3_w11_corpse", false, false).await?;
            for (root, expected, durations) in [
                (
                    5973,
                    vec![5973, 4025, 4026, 4027],
                    vec![10000i64, 300000, 300000, 60000],
                ),
                (44012, vec![44012, 44013, 44014], vec![5000i64, 5000, 5000]),
                (
                    33859,
                    vec![33859, 33858, 33857, 33856],
                    vec![10000, 900000, 600000, 600000],
                ),
                (
                    33863,
                    vec![33863, 33862, 33861, 33860],
                    vec![10000, 600000, 600000, 600000],
                ),
                (
                    6028,
                    vec![6028, 4206, 4207, 4208],
                    vec![10000, 300000, 300000, 60000],
                ),
            ] {
                let key = format!("oteryn:item.tibia.i{root}");
                let chain: Option<Vec<i32>> =
                    sqlx::query_scalar("SELECT game_summer_corpse_chain($1)")
                        .bind(&key)
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(chain, Some(expected), "actual source corpse {root}");
                let actual: Option<Vec<i64>> =
                    sqlx::query_scalar("SELECT game_source_corpse_durations_v1($1)")
                        .bind(&key)
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(
                    actual,
                    Some(durations),
                    "source stage and terminal body durations {root}"
                );
            }
            h.cleanup().await
        })
    })?;
    eprintln!(
        "SRX3_W11_CORPSE_POLICY_POSTGRES_CASE_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_rotten_ambient_corpse_source_chain_policy_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-rotten-ambient-corpse", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_rotten_ambient_corpse",
                false,
                false,
            )
            .await?;
            for root in [
                43816i32, 43567, 43840, 43583, 43571, 43844, 43559, 43555, 43579, 43820, 43832,
                43836, 43824, 43828,
            ] {
                let key = format!("oteryn:item.tibia.i{root}");
                let chain: Option<Vec<i32>> =
                    sqlx::query_scalar("SELECT game_summer_corpse_chain($1)")
                        .bind(&key)
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(
                    chain,
                    Some(vec![root, root - 1, root - 2, root - 3]),
                    "actual Rotten hunter decay closure {root}"
                );
                let durations: Option<Vec<i64>> =
                    sqlx::query_scalar("SELECT game_source_corpse_durations_v1($1)")
                        .bind(&key)
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(
                    durations,
                    Some(if matches!(root, 43832 | 43836) {
                        vec![5000; 4]
                    } else {
                        vec![10000, 300000, 300000, 60000]
                    }),
                    "actual Rotten hunter decay durations {root}"
                );
            }
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ROTTEN_AMBIENT_CORPSE_POLICY_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn srx3_all52_source_corpse_policy_exact_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-all52-source-corpse", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "srx3_all52_source_corpse",
                false,
                false,
            )
            .await?;
            let capture: serde_json::Value = serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/native-gameplay/boss-corpse-item-profiles.json"
            )))?;
            let chains = capture["decay_chains"].as_array().expect("source chains");
            assert_eq!(chains.len(), 52);
            for chain in chains {
                let expected = chain
                    .as_array()
                    .expect("source chain")
                    .iter()
                    .map(|n| n.as_i64().expect("sourceID") as i32)
                    .collect::<Vec<_>>();
                let key = format!("oteryn:item.tibia.i{}", expected[0]);
                let mut durations = Vec::new();
                for stage in &expected {
                    let record = capture["records"]
                        .as_array()
                        .expect("source records")
                        .iter()
                        .find(|r| {
                            r["production_binding"]["external_id"].as_str()
                                == Some(stage.to_string().as_str())
                        })
                        .expect("complete source stage");
                    durations.push(if record["semantics"]["temporal"]["state"] == "KNOWN" {
                        record["semantics"]["temporal"]["value"]["duration"]["value"]
                            .as_i64()
                            .expect("source lifetime")
                    } else {
                        0
                    });
                }
                let actual: Option<Vec<i32>> =
                    sqlx::query_scalar("SELECT game_summer_corpse_chain($1)")
                        .bind(&key)
                        .fetch_one(&h.pool)
                        .await?;
                let actual_durations: Option<Vec<i64>> =
                    sqlx::query_scalar("SELECT game_source_corpse_durations_v1($1)")
                        .bind(&key)
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(actual, Some(expected), "source chain {key}");
                assert_eq!(actual_durations, Some(durations), "source durations {key}");
            }
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_ALL52_SOURCE_CORPSE_POLICY_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
