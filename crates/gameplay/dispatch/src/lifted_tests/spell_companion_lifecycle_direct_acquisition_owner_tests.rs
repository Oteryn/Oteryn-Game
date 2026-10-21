#![allow(clippy::expect_used)]
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::foundation::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags};
use crate::spell::cast::{CharacterCastFacts, PlayerSpellState};
use crate::spell::companion_lifecycle::*;
use crate::spell::native::CompiledNativeSpell;
use crate::spell::{CasterState, Vocation};
use std::collections::BTreeSet;

fn profile(source: &str) -> CompiledNativeSpell {
    let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/spell-authoring/samples/native-spell-profiles.json"
    )))
    .expect("actual source profiles");
    let row = document["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|r| {
            r["spell"]["execution"]["native_behavior"]["key"] == "acquire_summon"
                && r["spell"]["execution"]["native_behavior"]["parameters"]["source"] == source
        })
        .expect("actual source family");
    crate::spell::native::spell_from_bundle(
        &serde_json::json!({"spell":row["spell"]}),
        &row["dependencies"],
    )
    .expect("complete canonical qualification")
}
fn target_fixture(
    mana: u64,
    infinite: bool,
) -> (
    ChannelRuntimeV1,
    crate::content::QualifiedNativeEntryRoom,
    DirectCompanionReservation,
    AcquisitionOwnerFacts,
) {
    let (mut runtime, owner, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(96);
    runtime
        .initialize_first_entry_position(owner)
        .expect("actual positioned owner");
    // A typed physical owner fixture, not an active source importer or grant.
    let policy = CompiledCreaturePolicy {
        definition_key: "creature:test-direct".into(),
        definition_revision: "test-1".into(),
        display_name: "test direct".into(),
        maximum_health: 812,
        base_speed: 220,
        outfit_look_type: 991,
        object_look_type: None,
        summonable: true,
        convinceable: true,
        mana_cost: Some(250),
        is_familiar: false,
        condition_immunities: vec![],
        defense: Some(1),
        armor: Some(10),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
        preferred_distance: Some(1),
        reward_boss: Some(false),
    };
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                vec![policy],
            )
            .expect("closed fixture policy"),
        )
        .expect("actual owner policy");
    let target = runtime
        .create_companion(
            owner,
            session,
            "creature:test-direct",
            MovementLocalPosition {
                x: 1,
                y: 0,
                floor: 0,
            },
            None,
            None,
            0,
        )
        .expect("real creature incarnation");
    let room = crate::content::qualify_native_entry_room(runtime.binding().world_id())
        .expect("actual map");
    let restrictions = AcquisitionOwnerFacts {
        mana,
        has_infinite_mana: infinite,
        can_summon_all: false,
        can_convince_all: true,
        black_skull: false,
    };
    let character = crate::domain::CharacterId::from_bytes([
        1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16,
    ])
    .expect("native character");
    let reservation = prepare_direct_acquisition(
        &runtime,
        room.movement_cells(),
        owner,
        session,
        character,
        &profile("target_creature"),
        AcquisitionSource::Target(target),
        &restrictions,
    )
    .expect("real target preparation");
    (runtime, room, reservation, restrictions)
}
#[test]
fn direct_reservation_checks_independent_restrictions_and_current_target() {
    let (mut runtime, _, reservation, current) = target_fixture(500, false);
    assert!(reservation.validate_current(&runtime, &current).is_ok());
    for field in 0..4 {
        let mut changed = current.clone();
        match field {
            0 => changed.mana -= 1,
            1 => changed.has_infinite_mana = true,
            2 => changed.can_summon_all = true,
            _ => changed.can_convince_all = false,
        }
        assert_eq!(
            reservation.validate_current(&runtime, &changed),
            Err(CompanionExecutionError::SnapshotMismatch)
        );
    }
    let target = runtime
        .companion_snapshot(reservation.actor())
        .expect("independent actual target");
    let mut changed = target.state.clone();
    changed.creation_speed_delta = 1;
    runtime
        .compare_companion_state(&target, changed)
        .expect("actual unrelated owner mutation");
    assert!(reservation.validate_current(&runtime, &current).is_err());
    assert_eq!(
        runtime
            .companion_snapshot(reservation.actor())
            .expect("target remains live")
            .state
            .master,
        target.state.master
    );
}
#[test]
fn infinite_mana_keeps_exact_source_training_cost_without_debit_or_profile_substitution() {
    let (runtime, _, reservation, current) = target_fixture(0, true);
    assert_eq!(reservation.mana_to_deduct(), 0);
    assert_eq!(reservation.mana_spent(), 250);
    assert!(reservation.consume_rune_charge());
    assert!(reservation.qualifies_payment(&profile("target_creature"), 0));
    assert!(!reservation.qualifies_payment(&profile("named_creature"), 0));
    assert!(!reservation.qualifies_payment(&profile("target_creature"), 1));
    assert!(reservation.validate_current(&runtime, &current).is_ok());
}
#[test]
fn bounded_exclusions_narrow_static_placement_without_granting_absent_cells() {
    let (reference, _, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(97);
    let world = reference.binding().world_id();
    let room = crate::content::qualify_native_entry_room(world).expect("qualified immutable map");
    let start = room.entry_start();
    let node = crate::foundation::NodeId::decode(&[
        1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 17,
    ])
    .expect("native test node");
    // A test runtime uses the actual qualified map/frame digests. This is
    // not a production content activation or a dynamic door/Item grant.
    let pin = crate::foundation::ChannelContentPin::from_activation(
        world,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        reference.binding().channel_id(),
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin,
    )
    .expect("actual test carrier");
    let reserved = runtime
        .reserve_fresh_session(session)
        .expect("actual source session");
    let owner = runtime
        .commit_fresh_session(reserved)
        .expect("actual actor");
    runtime
        .initialize_first_entry_position(owner)
        .expect("qualified positioned actor");
    let anchor = runtime
        .read_actor_position(owner)
        .expect("actual physical source")
        .position();
    let first = placement_excluding(&runtime, room.movement_cells(), anchor, true, &[])
        .expect("bounded static search")
        .expect("first free qualified cell");
    let second = placement_excluding(&runtime, room.movement_cells(), anchor, true, &[first])
        .expect("bounded exclusions")
        .expect("later qualified cell");
    assert_ne!(first, second);
    assert_eq!(
        placement_excluding(
            &runtime,
            room.movement_cells(),
            anchor,
            true,
            &[first, second]
        )
        .expect("blocked candidates"),
        None
    );
    assert_eq!(
        placement_excluding(
            &runtime,
            room.movement_cells(),
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: anchor.floor
            },
            true,
            &[]
        )
        .expect("unknown source terrain"),
        None
    );
}
#[test]
fn dynamic_payment_is_once_and_cooldown_rejection_does_not_mutate_the_owner() {
    let (runtime, room, reservation, mut current) = target_fixture(500, false);
    let p = profile("target_creature");
    let spell = oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle(
        &serde_json::json!({"spell":p.spell()}),
        p.dependencies(),
    )
    .expect("actual complete header");
    let state = PlayerSpellState::new(
        CharacterCastFacts {
            vocation: Vocation::Druid,
            level: 100,
            magic_level: 50,
            max_health: 500,
            max_mana: 500,
            max_soul: 100,
        },
        0,
        0,
    )
    .expect("actual caster state");
    let caster = CasterState {
        vocation: Vocation::Druid,
        level: 100,
        magic_level: 50,
        premium: crate::spell::CasterPremium::Current,
        mana: 500,
        max_mana: 500,
        soul: 100,
        learned: BTreeSet::new(),
        attack_skill: 0,
        attack_value: 0,
        attack_factor: 1.0,
        shielding_skill: 0,
        melee_weapon: false,
        shield_defense: None,
        harmony_multiplier: crate::spell::harmony::HarmonyMultiplier::ONE,
    };
    let operational = crate::spell::OperationalCastFacts {
        caster_position: crate::spell::chain::TilePosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        target_position: Some(crate::spell::chain::TilePosition {
            x: 1,
            y: 0,
            floor: 0,
        }),
        target: Some(crate::spell::target::CastTarget {
            caster: 1,
            creature: 2,
            actor: "actual:creature:2".into(),
            master: Some(1),
        }),
        line_of_sight_clear: Some(true),
        direction_available: false,
        wheel_unlocked: None,
        in_protection_zone: false,
        target_tile_solid: Some(false),
        target_tile_creature: Some(true),
    };
    let paid = crate::spell::cast::prepare_direct_acquisition_owner_cast_with_caster(
        &state,
        &spell,
        &operational,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
        &caster,
        &reservation,
    )
    .expect("source dynamic payment");
    assert_eq!(paid.anchor.paid_mana, 250);
    assert_eq!(paid.next.vitals().mana, 250);
    assert_eq!(state.vitals().mana, 500);
    let mut occupied_solid = operational.clone();
    occupied_solid.target_tile_solid = Some(true);
    assert!(
        crate::spell::cast::prepare_direct_acquisition_owner_cast_with_caster(
            &state,
            &spell,
            &occupied_solid,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
            &caster,
            &reservation
        )
        .is_ok()
    );
    let mut empty_solid = occupied_solid.clone();
    empty_solid.target_tile_creature = Some(false);
    assert!(
        crate::spell::cast::prepare_direct_acquisition_owner_cast_with_caster(
            &state,
            &spell,
            &empty_solid,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
            &caster,
            &reservation
        )
        .is_err()
    );
    occupied_solid.target_tile_solid = None;
    assert!(
        crate::spell::cast::prepare_direct_acquisition_owner_cast_with_caster(
            &state,
            &spell,
            &occupied_solid,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(100),
            &caster,
            &reservation
        )
        .is_err()
    );
    let mut retry = caster.clone();
    retry.mana = 250;
    current.mana = 250;
    let current_reservation = prepare_direct_acquisition(
        &runtime,
        room.movement_cells(),
        reservation.master_actor(),
        reservation.game_session_id(),
        reservation.character_id(),
        &p,
        AcquisitionSource::Target(reservation.actor()),
        &current,
    )
    .expect("independently current target/cost qualification");
    assert_eq!(
        crate::spell::cast::prepare_direct_acquisition_owner_cast_with_caster(
            &paid.next,
            &spell,
            &operational,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(101),
            &retry,
            &current_reservation
        )
        .err(),
        Some(oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::CoolingDown)
    );
    assert_eq!(paid.next.vitals().mana, 250);
}
