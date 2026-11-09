//! Source-family activation without an Ichgahal/Murcion wave owner.
//! Qualification actors use the real source pin/map; this does not open a Rotten quest room.
use super::fixture::debug;
use super::m26_rotten_wave_production::admit_source_actor;
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::{
    ComposedFreshAdmission, actor_spell::ChannelSpellStates, attack::ChannelAttackStates,
};

#[allow(
    clippy::too_many_arguments,
    reason = "the PG case borrows the actual production owners and scope facts"
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
    player: ExactActorRef,
    position: MovementLocalPosition,
    now: u64,
) -> Result<u64, Box<dyn std::error::Error>> {
    assert!(
        !states.rotten_waves.has_attempts(),
        "no prior source wave can arm this case"
    );
    let boss = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now,
        "oteryn:creature.chagorz",
        u16::MAX - 1,
    )
    .await?;
    let bloodjaw = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now,
        "oteryn:creature.elder_bloodjaw",
        u16::MAX - 2,
    )
    .await?;
    let mushroom = admit_source_actor(
        owner,
        runtime,
        states,
        permit,
        room,
        native,
        position,
        now,
        "oteryn:creature.mushroom",
        u16::MAX - 3,
    )
    .await?;
    drive(
        owner, runtime, states, attack, room, native, permit, revisions, position, now,
    )
    .await?;
    assert_eq!(
        states
            .monster_melee
            .creature_ai
            .state(bloodjaw)
            .ok_or("native Bloodjaw")?
            .target(),
        Some(player),
        "Chagorz source attempt enables the actual Bloodjaw onThink"
    );
    assert!(
        runtime.read_actor_position(bloodjaw).is_ok(),
        "Chagorz keeps Bloodjaws alive"
    );
    assert!(
        runtime.read_actor_position(mushroom).is_err(),
        "Chagorz does not keep Mushrooms alive"
    );
    assert!(states.monster_melee.creature_ai.state(mushroom).is_none());
    assert!(
        !states.rotten_waves.has_attempts(),
        "no unrelated wave planner was installed"
    );
    runtime.remove_test_actor(boss).map_err(debug)?;
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
        now + 1_000_000,
    )
    .await?;
    assert!(
        runtime.read_actor_position(bloodjaw).is_err(),
        "source callback remains armed after the parent leaves"
    );
    assert!(states.monster_melee.creature_ai.state(bloodjaw).is_none());
    assert!(!states.rotten_waves.has_attempts());
    // Fresh callback ownership must not require a previously observed boss.
    // The preceding assertions prove there are no live source bosses or wave owners.
    states.rotten_waves = Default::default();
    let mut initial_orphans = Vec::new();
    for (key, source_id) in [
        ("oteryn:creature.elder_bloodjaw", u16::MAX - 4),
        ("oteryn:creature.mushroom", u16::MAX - 5),
    ] {
        initial_orphans.push(
            admit_source_actor(
                owner,
                runtime,
                states,
                permit,
                room,
                native,
                position,
                now + 2_000_000,
                key,
                source_id,
            )
            .await?,
        );
    }
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
        now + 2_000_000,
    )
    .await?;
    for actor in initial_orphans {
        assert!(
            runtime.read_actor_position(actor).is_err(),
            "a source minion's first callback removes it without prior boss observation"
        );
        assert!(
            states.monster_melee.creature_ai.state(actor).is_none(),
            "initial orphan removal clears its native AI registration"
        );
    }
    assert!(!states.rotten_waves.has_attempts());
    eprintln!(
        "M26_ROTTEN_FAMILY production_driver=true chagorz=true no_wave_owner=true bloodjaw_target=true distinct_mushroom_rule=true orphan_cleanup=true initial_orphan_cleanup=true"
    );
    Ok(now + 3_000_000)
}

#[allow(
    clippy::too_many_arguments,
    reason = "same production driver and owned current facts as the source case"
)]
async fn drive(
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
    states.monster_melee.creature_ai.wake(
        position,
        |actor| {
            runtime
                .read_actor_position(actor)
                .ok()
                .map(|row| row.position())
        },
        now,
    );
    owner
        .drive_monster_profiles(
            runtime,
            states,
            attack,
            room,
            native,
            permit,
            revisions,
            &[],
            now,
        )
        .await
        .map_err(|_| "source-family production drive")?;
    Ok(())
}
