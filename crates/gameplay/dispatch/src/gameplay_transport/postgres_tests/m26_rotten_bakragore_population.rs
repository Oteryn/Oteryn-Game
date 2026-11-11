//! Source Bakragore population requests through current PG/map/native owners.
//! Dense close placement may refuse requests, as source createMonster does.
use super::m26_rotten_family_production::drive;
use super::m26_rotten_wave_production::admit_source_actor;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::{
    ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates,
};

#[allow(
    clippy::too_many_arguments,
    reason = "actual current PG, map and native driver owners remain explicit"
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
) -> super::TestResult {
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
        "oteryn:creature.bakragore",
        u16::MAX - 10,
    )
    .await?;
    let mut tick = 0;
    let mut tracked = Vec::new();
    let mut original_mushrooms = Vec::new();
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
        let kinds = population(runtime, boss)?;
        assert!(
            kinds[0].len() <= 2 && kinds[1].len() <= 4 && kinds[2].len() <= 4,
            "untainted source population bounds"
        );
        for child in kinds.iter().flatten() {
            assert!(
                states.monster_melee.creature_ai.state(*child).is_some(),
                "every accepted source kind enters native AI"
            );
            assert!(
                runtime.native_summon_master(*child).is_none(),
                "source createMonster is masterless"
            );
            tracked.push(*child);
        }
        if kinds.iter().all(|kind| !kind.is_empty()) {
            original_mushrooms = kinds[1].clone();
            break;
        }
    }
    assert!(
        !original_mushrooms.is_empty(),
        "the actual source caller creates Bloodjaws, Mushrooms and Pillars"
    );
    let removed = original_mushrooms[0];
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
        let kinds = population(runtime, boss)?;
        tracked.extend(kinds.iter().flatten().copied());
        if kinds[1]
            .iter()
            .any(|actor| !original_mushrooms.contains(actor))
        {
            assert!(
                !kinds[1].contains(&removed),
                "replacement has a new physical generation"
            );
            refilled = true;
            break;
        }
    }
    assert!(
        refilled,
        "source population retries create a replacement Mushroom"
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
    for actor in tracked {
        assert!(
            runtime.read_actor_position(actor).is_err(),
            "all source kinds orphan after Bakragore leaves"
        );
        assert!(
            states.monster_melee.creature_ai.state(actor).is_none(),
            "orphan cleanup retires native AI"
        );
    }
    eprintln!(
        "M26_BAKRAGORE_POPULATION production_driver=true pg_map=true native_three_kinds=true native_ai=true refill=true orphan_cleanup=true"
    );
    Ok(())
}

fn population(
    runtime: &ChannelRuntimeV1,
    boss: ExactActorRef,
) -> Result<[Vec<ExactActorRef>; 3], Box<dyn std::error::Error>> {
    let own = runtime
        .read_actor_position(boss)
        .map_err(super::fixture::debug)?
        .position();
    let mut kinds = [Vec::new(), Vec::new(), Vec::new()];
    for (actor, at, session) in runtime
        .positioned_melee_census()
        .map_err(super::fixture::debug)?
    {
        let at = at.position();
        if session.is_some()
            || at.floor != own.floor
            || at.x.abs_diff(own.x) > 20
            || at.y.abs_diff(own.y) > 20
        {
            continue;
        }
        let snapshot = runtime
            .companion_snapshot(actor)
            .map_err(super::fixture::debug)?;
        let index = match snapshot.state.policy.definition_key.as_str() {
            "oteryn:creature.elder_bloodjaw" => 0,
            "oteryn:creature.mushroom" => 1,
            "oteryn:creature.pillar_of_dark_energy" => 2,
            _ => continue,
        };
        kinds[index].push(actor);
    }
    Ok(kinds)
}
