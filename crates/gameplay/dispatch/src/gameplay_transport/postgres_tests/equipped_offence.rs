//! PARITY-EQUIP-MODIFIERS-1: equipment critical hit and leech on real auto-attack dispatch.
use super::party_xp::{self, Case};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_combat_rules::resolution::equipped_offence::{EquippedOffence, leech_amount};
use std::time::Duration;

#[test]
fn equipped_offence_postgres_real_swings_critical_hit_and_leech() -> TestResult {
    party_xp::run(Case::Offence)
}

/// The rat's health before every pass: high enough that a doubled hit never caps the real damage.
const RAT_HEALTH: i64 = 1_000;

pub(super) async fn qualify(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    _h: &Harness,
    players: &[(ExactActorRef, GameSessionId)],
) -> TestResult {
    let (actor, session) = players[0];
    // Half of the real damage leeches; the first phase never crits, the second always does.
    let leech = EquippedOffence {
        life_leech: 5_000,
        mana_leech: 5_000,
        ..EquippedOffence::default()
    };
    let critical = EquippedOffence {
        critical_chance: 10_000,
        critical_damage: u16::MAX,
        ..leech.clone()
    };
    let creature;
    {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
        state.health = state.facts.max_health / 4;
        let (drained, _) = states
            .get(&runtime, actor, session)
            .and_then(|state| state.after_creature_mana_drain(state.facts.max_mana))
            .ok_or("drain mana")?;
        assert!(states.commit(&runtime, actor, session, drained));
        drop(states);
        assert!(owner.states.attack.lock().await.set_modes(
            &runtime,
            actor,
            session,
            oteryn_protocol_oteryn::attack::FightModes {
                fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
                ..oteryn_protocol_oteryn::attack::FightModes::DEFAULT
            }
        ));
        let visible = runtime.visible_entities();
        let row = visible.creatures.first().ok_or("rat")?;
        creature = row.actor;
        let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: creature.placement_identity(),
            generation: row.generation,
        };
        assert_eq!(
            owner.states.attack.lock().await.set_target(
                &runtime,
                owner.rooms.gate_auto_attack(),
                actor,
                session,
                owner.owner_now(),
                Some(target),
                30
            ),
            oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        );
    }
    let plain = passes(owner, actor, session, creature, leech).await?;
    let critical_hits = passes(owner, actor, session, creature, critical).await?;
    assert!(plain.0 >= 3 && critical_hits.0 >= 3, "{plain:?} {critical_hits:?}");
    assert!(
        critical_hits.1 > 2 * plain.1,
        "a critical hit multiplies the swing before the rat's mitigation: {plain:?} {critical_hits:?}"
    );
    Ok(())
}

/// 32 real auto-attack passes with `offence` worn: the landed hits and the largest real damage.
async fn passes(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    creature: ExactActorRef,
    offence: EquippedOffence,
) -> TestResult<(u32, u32)> {
    let (mut landed, mut largest) = (0, 0);
    for _ in 0..32 {
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("state")?;
            state.install_equipped_offence(offence.clone());
            // Keep both pools below their maxima so every leech is visible.
            state.health = state.health.min(state.facts.max_health / 4);
        }
        owner
            .runtime
            .lock()
            .await
            .set_test_creature_health(creature, RAT_HEALTH)
            .map_err(debug)?;
        let before = vitals(owner, actor, session).await?;
        owner.refresh_attack_weapon(actor, session).await;
        owner.drain_auto_attacks().await;
        let after = vitals(owner, actor, session).await?;
        let rat = owner
            .runtime
            .lock()
            .await
            .live_creature_health(creature)
            .ok_or("living rat")?;
        let real = u32::try_from(RAT_HEALTH.unsigned_abs() - rat)?;
        let gained = |current: u32, max: u32| current.saturating_add(leech_amount(real, 5_000)).min(max);
        assert_eq!(
            (after.health, after.mana),
            (
                gained(before.health, before.max_health),
                gained(before.mana, before.max_mana)
            ),
            "life and mana leech half of {real} real damage"
        );
        if real > 0 {
            landed += 1;
            largest = largest.max(real);
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(3))
            .ok_or("clock")?;
    }
    let installed = owner
        .states
        .spell_states
        .lock()
        .await
        .read_owned_player_state_test_snapshot(&*owner.runtime.lock().await, actor, session)
        .ok_or("state")?
        .equipped_offence();
    assert_eq!(installed, offence, "a weapon refresh keeps the worn offence");
    Ok((landed, largest))
}

async fn vitals(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> TestResult<oteryn_protocol_oteryn::actor_spell::ActorVitals> {
    let runtime = owner.runtime.lock().await;
    Ok(owner
        .states
        .spell_states
        .lock()
        .await
        .get(&runtime, actor, session)
        .ok_or("state")?
        .vitals())
}
