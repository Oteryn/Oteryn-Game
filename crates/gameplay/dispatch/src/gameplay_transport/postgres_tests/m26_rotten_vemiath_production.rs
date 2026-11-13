//! Vemiath population: actual source thinks, native placement, AI, refill and orphan cleanup.
use super::m26_rotten_family_production::drive;
use super::m26_rotten_wave_production::admit_source_actor;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::{
    ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates,
};

#[allow(
    clippy::too_many_arguments,
    reason = "current PG scope, native map and actual driver owners remain explicit"
)]
pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    revisions: &crate::ability::RevisionSet,
    position: MovementLocalPosition,
    now: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    states.rotten_waves = Default::default();
    let boss = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now,
        "oteryn:creature.vemiath",
        u16::MAX - 9,
    )
    .await?;
    let mut tick = 0;
    let mut children = Vec::new();
    let mut filled = false;
    let mut initial_pillars = Vec::new();
    for _ in 0..30 {
        drive(
            owner,
            runtime,
            states,
            attack,
            room,
            native,
            permit,
            revisions,
            position,
            now + tick * 1_000_000,
        )
        .await?;
        tick += 1;
        let (pillars, bloodjaws) = population(runtime, boss)?;
        for child in pillars.iter().chain(&bloodjaws) {
            assert!(
                states.monster_melee.creature_ai.state(*child).is_some(),
                "source-created child enters native AI"
            );
            assert!(
                runtime.native_summon_master(*child).is_none(),
                "source createMonster is masterless"
            );
            children.push(*child);
        }
        if pillars.len() == 4 && bloodjaws.len() == 2 {
            initial_pillars = pillars;
            filled = true;
            break;
        }
    }
    assert!(
        filled,
        "Vemiath's production source callback fills four Pillars and two untainted Bloodjaws"
    );
    let removed = initial_pillars[0];
    runtime
        .remove_test_actor(removed)
        .map_err(super::fixture::debug)?;
    states.monster_melee.creature_ai.remove(removed);
    let mut refilled = false;
    for _ in 0..15 {
        drive(
            owner,
            runtime,
            states,
            attack,
            room,
            native,
            permit,
            revisions,
            position,
            now + tick * 1_000_000,
        )
        .await?;
        tick += 1;
        let (pillars, bloodjaws) = population(runtime, boss)?;
        children.extend(pillars.iter().chain(&bloodjaws).copied());
        if pillars.len() == 4 && bloodjaws.len() == 2 {
            assert!(
                !pillars.contains(&removed),
                "refill is a new physical incarnation"
            );
            refilled = true;
            break;
        }
    }
    assert!(
        refilled,
        "the next source population requests replace a missing Pillar"
    );
    runtime
        .remove_test_actor(boss)
        .map_err(super::fixture::debug)?;
    states.monster_melee.creature_ai.remove(boss);
    drive(
        owner,
        runtime,
        states,
        attack,
        room,
        native,
        permit,
        revisions,
        position,
        now + tick * 1_000_000,
    )
    .await?;
    for child in children {
        assert!(
            runtime.read_actor_position(child).is_err(),
            "source minions disappear after their keeper leaves"
        );
        assert!(
            states.monster_melee.creature_ai.state(child).is_none(),
            "orphan cleanup retires native AI"
        );
    }
    eprintln!(
        "M26_VEMIATH_POPULATION production_driver=true pg_map=true fill=true native_ai=true refill=true orphan_cleanup=true"
    );
    Ok(now + (tick + 1) * 1_000_000)
}

fn population(
    runtime: &ChannelRuntimeV1,
    boss: ExactActorRef,
) -> Result<(Vec<ExactActorRef>, Vec<ExactActorRef>), Box<dyn std::error::Error>> {
    let own = runtime
        .read_actor_position(boss)
        .map_err(super::fixture::debug)?
        .position();
    let mut pillars = Vec::new();
    let mut bloodjaws = Vec::new();
    for (actor, at, session) in runtime
        .positioned_melee_census()
        .map_err(super::fixture::debug)?
    {
        let at = at.position();
        if session.is_some()
            || at.floor != own.floor
            || at.x.abs_diff(own.x) > 17
            || at.y.abs_diff(own.y) > 17
        {
            continue;
        }
        let snapshot = runtime
            .companion_snapshot(actor)
            .map_err(super::fixture::debug)?;
        match snapshot.state.policy.definition_key.as_str() {
            "oteryn:creature.pillar_of_dark_energy" => pillars.push(actor),
            "oteryn:creature.elder_bloodjaw" => bloodjaws.push(actor),
            _ => {}
        }
    }
    Ok((pillars, bloodjaws))
}
