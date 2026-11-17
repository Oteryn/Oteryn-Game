//! Deepling guards credit the world through native kill settlement.
use super::*;
use crate::durability::world_event::changes::counters::CounterKind;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn worldchg_deepling_guard_death_credits_one_tenth_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("wcdeep-guard", || {
        super::world_event::scenario_with_native_deaths(1, "deepling_guard", true)
    })
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    owner.admit_progression_session(admitted).await;
    let day = h.root.world_changes(admitted.world_id, owner.reset_epoch).await?
        .ok_or("world change day")?;
    let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    {
        let mut runtime = owner.runtime.lock().await;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        runtime.install_world_change_cycles(
            owner.reset_epoch,
            CounterKind::ALL.into_iter().map(|kind| {
                (kind.key().to_owned(), day.counter_generations[kind.index()])
            }).collect(),
        ).map_err(debug)?;
    }
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.deepling_guard", 20_000).await?;
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (state->'counts'->>0)::bigint,(state->'counts'->>1)::bigint FROM game_world_change_counters WHERE change_key='deeplings'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(counts, (0, 1));
    Ok(())
}
