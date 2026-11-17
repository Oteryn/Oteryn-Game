//! The original 90s callback, including its late physical MINT, precedes death.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{CommandId, CommandRef, RuntimeScopeRefV1};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

pub(super) async fn exercise(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual Vemiath Instance fence")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("actual Vemiath Instance".into());
    };
    let registered = owner
        .activity_runtimes
        .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        .ok_or("actual Vemiath runtime")?;
    let player = admitted.runtime_actor.ok_or("actual transferred Player")?;
    owner.runtime_view(&registered).drain_monster_melee().await;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("first Think clock")?;
    owner.runtime_view(&registered).drain_monster_melee().await;
    let (boss, original_due, original_think) = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        let boss = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|entry| {
                runtime.matches_live_creature_identity(entry.actor, b"oteryn:creature.vemiath")
            })
            .ok_or("actual native Vemiath")?
            .actor;
        assert!(
            runtime
                .visible_entities()
                .creatures
                .iter()
                .any(|entry| runtime.matches_live_creature_identity(
                    entry.actor,
                    b"oteryn:creature.elder_bloodjaw"
                )),
            "the original source Think created the required live Bloodjaw"
        );
        let states = view.states.spell_states.lock().await;
        let clock = states
            .rotten_blood_callbacks
            .portal_clock_for_test(boss)
            .ok_or("actual Vemiath first-Think clock")?;
        assert!(clock.callbacks.is_empty());
        (
            boss,
            clock.cycle_due.ok_or("real original 90s callback")?,
            clock.think_sequence,
        )
    };
    let wait = original_due
        .checked_sub(owner.owner_now().get())
        .and_then(|wait| wait.checked_add(1_000_000))
        .ok_or("original due must still be future")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_micros(wait))
        .ok_or("original portal clock")?;
    let notice = {
        let view = owner.runtime_view(&registered);
        let mut runtime = view.runtime.lock().await;
        let now = view.owner_now();
        assert!(original_due < now.get());
        assert!(
            runtime
                .next_source_creature_death()
                .map_err(debug)?
                .is_none()
        );
        let maximum = runtime
            .companion_snapshot(boss)
            .map_err(debug)?
            .maximum_health;
        let batch = OwnerCombatBatch {
            caster: player,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(admitted.game_session_id, CommandId::new(94).map_err(debug)?),
            occurrence: SpellOccurrenceBinding {
                id: "srx3:vemiath-portal-before-death".into(),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: b"srx3:vemiath-portal-before-death".to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target: boss,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(boss).map_err(debug)?,
                    magnitude: maximum,
                },
            }],
            deferred: None,
        };
        let stage = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(stage).map_err(debug)?;
        assert!(
            receipt
                .effects
                .iter()
                .any(|effect| effect.health.is_some_and(|health| health.health_after == 0)),
            "the actual canonical HP commit killed Vemiath"
        );
        let notice = runtime
            .next_source_creature_death()
            .map_err(debug)?
            .expect("the real Rotten registry must publish Vemiath's copied source death");
        assert_eq!(notice.actor(), boss);
        assert_eq!(notice.enqueued_at_us(), Some(now.get()));
        notice
    };
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(10))
        .ok_or("late callback execution clock")?;
    let late_now = owner.owner_now().get();
    assert!(late_now > notice.enqueued_at_us().ok_or("actual source cutoff")?);
    owner.runtime_view(&registered).drain_monster_melee().await;
    let mint_at = {
        let view = owner.runtime_view(&registered);
        let runtime = view.runtime.lock().await;
        assert_eq!(
            runtime.next_source_creature_death().map_err(debug)?,
            Some(notice.clone()),
            "death cannot start before the original portal callback's physical continuation"
        );
        assert!(runtime.source_creature_is_registered(boss).map_err(debug)?);
        assert_eq!(
            runtime
                .companion_snapshot_including_dead(boss)
                .map_err(debug)?
                .health,
            0
        );
        let states = view.states.spell_states.lock().await;
        let clock = states
            .rotten_blood_callbacks
            .portal_clock_for_test(boss)
            .ok_or("retained exact Vemiath clock")?;
        assert_eq!(
            clock.think_sequence, original_think,
            "a due-only pass never invents a fresh Think"
        );
        let [(cause, mint_at, true)] = clock.callbacks.as_slice() else {
            return Err(
                "exactly one physical creation continuation of the due 90s callback".into(),
            );
        };
        assert_eq!(*cause, original_due);
        assert!(
            *mint_at >= late_now,
            "physical MINT uses the real execution clock"
        );
        assert_eq!(
            states
                .rotten_blood_callbacks
                .next_source_due_time(&runtime)
                .map_err(debug)?,
            Some(original_due),
            "the later MINT must retain its parent's original place before death"
        );
        *mint_at
    };
    assert_eq!(counts(h, instance_id).await?, (0, 0, 0));
    let view = owner.runtime_view(&registered);
    view.drain_rotten_portal_callbacks().await;
    assert_eq!(
        counts(h, instance_id).await?,
        (1, 1, 0),
        "real MINT receipt and physical Item before death"
    );
    {
        let runtime = view.runtime.lock().await;
        assert_eq!(
            runtime.next_source_creature_death().map_err(debug)?,
            Some(notice.clone())
        );
        let states = view.states.spell_states.lock().await;
        let clock = states
            .rotten_blood_callbacks
            .portal_clock_for_test(boss)
            .ok_or("retained portal expiry")?;
        let expiry = mint_at.checked_add(15_000_000).ok_or("source 15s expiry")?;
        assert_eq!(
            clock.callbacks,
            vec![(expiry, expiry, false)],
            "the new expiry is based on actual MINT, never the old 90s parent deadline"
        );
        assert!(expiry > notice.enqueued_at_us().ok_or("actual source cutoff")?);
    }
    view.drain_rotten_portal_callbacks().await;
    assert_eq!(
        counts(h, instance_id).await?,
        (1, 1, 0),
        "replay neither remints nor expires the new portal early"
    );
    Ok(())
}

async fn counts(h: &Harness, instance: [u8; 16]) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT \
        (SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) \
         WHERE r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL \
         AND r.instance_id=encode($1,'hex')::uuid AND r.creature_definition_key='oteryn:creature.vemiath' \
         AND l.definition_production_key='oteryn:item.tibia.i37000' AND l.operation_kind=1), \
        (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) \
         WHERE g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=encode($1,'hex')::uuid \
         AND i.definition_production_key='oteryn:item.tibia.i37000' AND i.lifecycle=1), \
        (SELECT count(*) FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id) \
         WHERE r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL \
         AND r.instance_id=encode($1,'hex')::uuid AND l.definition_production_key='oteryn:item.tibia.i37000' \
         AND l.operation_kind=2)"
    ).bind(instance.as_slice()).fetch_one(&h.pool).await?)
}
