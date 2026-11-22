use super::TestResult;
use super::fixture::{Harness, run};
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::ordinary_corpse_decay_fixtures::MINT_SCENARIO;

#[derive(serde::Deserialize)]
struct SourceCorpseChains {
    chains: std::collections::BTreeMap<u32, Vec<(i32, i64)>>,
}

type RegistryRow = (String, Option<Vec<i32>>, Option<Vec<i64>>);

#[test]
fn ordinary_corpse_decay_registry_installs_exact_source_chains_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let SourceCorpseChains { chains } = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/ordinary-corpse-cohort.json"
    )))?;
    assert_eq!(chains.len(), 260);
    scenario_thread::run("ordinary-corpse-decay-cohort", move || {
        run(async move |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "ordinary_corpse_decay", false, false)
                    .await?;
            let keys: Vec<String> = chains
                .keys()
                .map(|id| format!("oteryn:item.tibia.i{id}"))
                .collect();
            let actual: Vec<RegistryRow> = sqlx::query_as(
                "SELECT key, game_summer_corpse_chain(key), game_source_corpse_durations_v1(key)
                 FROM unnest($1::text[]) WITH ORDINALITY AS roots(key, ord) ORDER BY ord",
            )
            .bind(&keys)
            .fetch_all(&h.pool)
            .await?;
            h.cleanup().await?;
            assert_eq!(actual.len(), chains.len());
            for ((key, stages, durations), expected) in actual.into_iter().zip(chains.values()) {
                let (expected_stages, expected_durations) = expected.iter().copied().unzip();
                assert_eq!(stages, Some(expected_stages), "{key}");
                assert_eq!(durations, Some(expected_durations), "{key}");
            }
            Ok(())
        })
    })
}

#[test]
fn ordinary_orc_death_mints_the_source_corpse_and_schedules_its_lifecycle_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::qengine_rewards_fixtures::scenario(MINT_SCENARIO)
}

#[test]
fn ordinary_cobra_drag_refreshes_the_bound_corpse_and_durable_schedule_postgres() -> TestResult {
    super::qengine_rewards_fixtures::scenario(super::ordinary_corpse_decay_fixtures::DRAG_SCENARIO)
}
