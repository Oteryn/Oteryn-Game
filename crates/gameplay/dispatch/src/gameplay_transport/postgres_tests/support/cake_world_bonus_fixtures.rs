use crate::bestiary_postgres_harness::TestResult;

pub(super) async fn assert_regeneration(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    players: &[(crate::foundation::ExactActorRef, crate::foundation::GameSessionId)],
    cake_bonus_active: bool,
) -> TestResult {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, ConflictKey};
    use oteryn_simulation_determinism::SemanticTimeMicros;
    for (index, (actor, session)) in players.iter().copied().enumerate() {
        owner.tick_actor_vitals(actor, session).await;
        let (started_at, health_interval_ms, mana_interval_ms) = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, actor, session)
                .ok_or("cake player state")?;
            let effect = state.owned_conditions().get(ConflictKey::Recovery);
            if !cake_bonus_active {
                assert!(
                    effect.is_none(),
                    "expired world window must not grant regeneration"
                );
                continue;
            }
            let effect =
                effect.ok_or("world regeneration applies to both premium and free players")?;
            assert_eq!(effect.definition().key(), "world.cake.regeneration");
            let ConditionValues::SpellRegeneration {
                sub_id: 0,
                duration_ms,
                health_gain: 1,
                mana_gain: 1,
                health_interval_ms,
                mana_interval_ms,
            } = effect.definition().values()
            else {
                return Err("expected the Cake combat regeneration condition".into());
            };
            assert!((1..=604_800_000).contains(&duration_ms));
            assert_eq!(effect.started_at() % 1_000, 0);
            assert_eq!(
                (health_interval_ms, mana_interval_ms),
                if index == 0 { (7_000, 4_000) } else { (6_000, 6_000) }
            );
            let started_at = effect.started_at();
            state.restore_saved_vitals(1, 0);
            (started_at, health_interval_ms, mana_interval_ms)
        };
        let mut intervals = [health_interval_ms, mana_interval_ms];
        intervals.sort_unstable();
        for (index, interval) in intervals.into_iter().enumerate() {
            if index == 1 && interval == intervals[0] {
                break;
            }
            let due = started_at + u64::from(interval) * 1_000;
            let scheduled = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                states
                    .get(&runtime, actor, session)
                    .and_then(|state| state.owned_conditions().get(ConflictKey::Recovery))
                    .cloned()
                    .ok_or("scheduled Cake pulse")?
            };
            owner
                .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(due - 1))
                .await;
            let before = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, actor, session).ok_or("cake player state")?;
                assert_eq!(
                    state.owned_conditions().get(ConflictKey::Recovery),
                    Some(&scheduled),
                    "Cake regeneration must leave the pulse pending until its deadline"
                );
                state.vitals()
            };
            let (_, after) = owner
                .tick_spell_periodic_at(actor, session, SemanticTimeMicros::from_micros(due))
                .await
                .ok_or("Cake regeneration pulse did not publish vitals")?;
            assert_eq!(
                (after.health - before.health, after.mana - before.mana),
                (
                    u32::from(interval.is_multiple_of(health_interval_ms)),
                    u32::from(interval.is_multiple_of(mana_interval_ms)),
                )
            );
        }
    }
    Ok(())
}
