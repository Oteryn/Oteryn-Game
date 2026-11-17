//! M26: source Pillar onThink through the production server driver and real PG map.
use super::m26_rotten_family_production::drive;
use super::m26_rotten_wave_production::admit_source_actor;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::{
    ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates,
};

#[allow(
    clippy::too_many_arguments,
    reason = "real current runtime, map and PG owner remain explicit"
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
    const KEY: &str = "oteryn:creature.pillar_of_dark_energy";
    // The preceding wave case proves every source child has been removed.
    // Start independent callback ownership without altering its deterministic actor history.
    states.rotten_waves = Default::default();
    assert!(!states.rotten_waves.has_attempts());
    let orphan = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now,
        KEY,
        u16::MAX - 6,
    )
    .await?;
    assert!(oteryn_gameplay_encounters::bosses::rotten_blood::native_pillar::NativePillarThinkOwner::for_actor(runtime, orphan, [0; 32]).is_err(),
        "an unrelated content owner cannot authorize this native Pillar");
    drive(
        owner, runtime, states, attack, room, native, permit, revisions, position, now,
    )
    .await?;
    assert!(
        runtime.read_actor_position(orphan).is_err(),
        "a Pillar's actual first source think removes it without a nearby source boss"
    );
    assert!(states.monster_melee.creature_ai.state(orphan).is_none());
    let boss = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now + 1_000_000,
        "oteryn:creature.chagorz",
        u16::MAX - 7,
    )
    .await?;
    assert!(oteryn_gameplay_encounters::bosses::rotten_blood::native_pillar::NativePillarThinkOwner::for_actor(runtime, boss, native.source_digest()).is_err(),
        "a boss identity cannot be consumed as a Pillar callback");
    let pillar = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now + 1_000_000,
        KEY,
        u16::MAX - 8,
    )
    .await?;
    for tick in 1..=10 {
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
        if tick < 10 {
            assert!(
                runtime.read_actor_position(pillar).is_ok(),
                "Chagorz keeps the source Pillar alive until its tenth native think"
            );
            assert!(states.monster_melee.creature_ai.state(pillar).is_some());
        } else {
            assert!(
                runtime.read_actor_position(pillar).is_err(),
                "the tenth native source callback expires the Pillar even while Chagorz lives"
            );
            assert!(states.monster_melee.creature_ai.state(pillar).is_none());
        }
        assert!(runtime.read_actor_position(boss).is_ok());
    }
    runtime
        .remove_test_actor(boss)
        .map_err(super::fixture::debug)?;
    states.monster_melee.creature_ai.remove(boss);
    assert!(!states.rotten_waves.has_attempts());
    eprintln!(
        "M26_ROTTEN_PILLAR production_driver=true pg_map=true initial_orphan=true ten_thinks=true ai_cleanup=true"
    );
    Ok(now + 11_000_000)
}
