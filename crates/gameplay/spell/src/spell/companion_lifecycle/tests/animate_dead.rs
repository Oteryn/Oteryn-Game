use super::*;
use crate::content::{
    ProjectV2AuthoringProfileData, native_gameplay::test_support::activated_with_loot_tables,
};
use crate::durability::native_world_items::{
    DurableCorpseReservation, QualifiedItemDefinition, SpellGroundTarget,
};
use crate::foundation::{ChannelId, NodeId, WorldId};
use crate::spell::Execution;

fn id(value: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = value;
    bytes
}

#[test]
fn animate_dead_prepares_the_production_skeleton_with_both_namesakes_installed()
-> Result<(), Box<dyn std::error::Error>> {
    let debug = |error: &dyn std::fmt::Debug| format!("{error:?}");
    let world = WorldId::decode(&id(1)).map_err(|error| debug(&error))?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let room = pin.qualified_room().clone();
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("active native gameplay")?;
    let spell = native.spell_book().rune(3203).ok_or("Animate Dead Rune")?;
    let Execution::NativeProfile(profile) = &spell.execution else {
        return Err("Animate Dead native profile".into());
    };
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).map_err(|error| debug(&error))?,
        NodeId::decode(&id(3)).map_err(|error| debug(&error))?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin.into_channel_pin(),
    )
    .map_err(|error| debug(&error))?;
    runtime
        .install_companion_policies(native.policies().clone())
        .map_err(|error| debug(&error))?;
    assert!(matches!(
        runtime.companion_policy("Skeleton"),
        Err(CarrierError::InvalidCreatureTarget)
    ));
    let session = GameSessionId::decode(&id(4)).map_err(|error| debug(&error))?;
    let reserved = runtime
        .reserve_fresh_session(session)
        .map_err(|error| debug(&error))?;
    let actor = runtime
        .commit_fresh_session(reserved)
        .map_err(|error| debug(&error))?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(|error| debug(&error))?;
    let caster = runtime
        .read_actor_position(actor)
        .map_err(|error| debug(&error))?;
    let target = placement(&runtime, room.movement_cells(), caster.position(), true)
        .map_err(|error| debug(&error))?
        .ok_or("a free tile next to the caster")?;
    let address = SpellGroundTarget::from_native_owner(&room, &runtime, target)?;
    let creature = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == "oteryn:creature.goldhanded_cultist")
        .ok_or("corpse Creature profile")?;
    let ProjectV2AuthoringProfileData::Creature(creature) = &creature.profile.data else {
        return Err("expected Creature profile".into());
    };
    let corpse_ref = creature
        .details
        .as_ref()
        .and_then(|details| details.corpse_item.as_ref())
        .ok_or("Creature corpse definition")?;
    let item = native
        .item_policy(&corpse_ref.key, &corpse_ref.revision)
        .ok_or("corpse Item policy")?;
    let item = QualifiedItemDefinition::from_native_policy(item)?;
    let corpse = DurableCorpseReservation {
        item_instance_id: id(5),
        definition: item.definition,
        state_revision: 1,
        top_down_ordinal: 1,
        placement: address.placement(id(6).to_vec()),
        mint_transaction_id: id(7),
        quantity: 1,
        contents: Vec::new(),
    };
    let spawn = prepare_animate_dead(
        &runtime,
        room.movement_cells(),
        actor,
        session,
        crate::domain::CharacterId::from_bytes(id(8)).map_err(|error| debug(&error))?,
        profile,
        &corpse,
        &AcquisitionOwnerFacts {
            mana: 0,
            has_infinite_mana: false,
            can_summon_all: false,
            can_convince_all: false,
            black_skull: false,
        },
    )
    .map_err(|error| debug(&error))?;
    assert_eq!(spawn.creature_definition_key(), "oteryn:creature.skeleton");
    assert_eq!(spawn.creature_definition_revision(), "definition-r1");
    assert_eq!(spawn.spawn.policy().display_name, "Skeleton");
    Ok(())
}
