//! The real admission and creature-hit owners, under routed PostgreSQL authority.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::spell::Vocation;

#[test]
fn vocationless_vitals_admission_keeps_saved_pools_and_combat_reaches_death_postgres() -> TestResult
{
    super::kill_reward_e2e::scenario_thread::run("tutorial-vitals", scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "tutorial_vitals", false, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
        let mut owner = dependencies.owner(&h, &authority);
        admit(&owner, &dependencies, &request).await?;
        {
            let runtime = dependencies.runtime.lock().await;
            let mut states = owner.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("missing vocationless vitals")?;
            assert_eq!(
                (
                    state.character_facts().vocation,
                    state.vitals().health,
                    state.vitals().mana
                ),
                (Vocation::None, 150, 55)
            );
            assert_eq!(state.saved_vitals(), Some((150, 55)));
            state.restore_saved_vitals(27, 9);
        }
        // The actual playable admission's same-session retry must keep live damage/mana.
        admit(&owner, &dependencies, &request).await?;
        {
            let runtime = dependencies.runtime.lock().await;
            assert_eq!(
                owner
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, actor, session)
                    .ok_or("live actor")?
                    .saved_vitals(),
                Some((27, 9))
            );
        }
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            dependencies.admitted.world_id,
            "content-1",
        )?;
        owner.active_generation = controller.active();
        let mut admitted = dependencies.admitted;
        Box::pin(owner.admit_progression_session(&mut admitted)).await;
        let death = {
            let mut runtime = dependencies.runtime.lock().await;
            let mut states = owner.spell_states.lock().await;
            let foreign = crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
            let now = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                owner.owner_now().get(),
            );
            assert!(
                states
                    .apply_creature_damage(&mut runtime, actor, foreign, 1000, now)
                    .is_none()
            );
            let hit = states
                .apply_creature_damage(&mut runtime, actor, session, 1000, now)
                .ok_or("real creature hit")?;
            assert_eq!(hit.damage.health_after, 0);
            assert!(hit.death.is_some());
            assert!(
                states
                    .apply_creature_damage(&mut runtime, actor, session, 1000, now)
                    .is_none()
            );
            states
                .player_death(&runtime, actor, session)
                .ok_or("real death occurrence")?
        };
        assert!(
            Box::pin(owner.respawn_after_death(actor, session, death))
                .await
                .is_some()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_death_receipts")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        assert!(!owner.spell_states.lock().await.is_dead(actor));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
