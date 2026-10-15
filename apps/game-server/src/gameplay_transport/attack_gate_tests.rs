//! CONDITIONS-0 §3.5 (MAGIC-COND-1): `pacified` in the actor's runtime condition slot refuses
//! an `ATTACK_TARGET` with `REJECTED` before every other target check; stopping still works.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::content::native_gameplay::{
    NativeGameplayInput, NativeGameplayMapProfile, PinnedGameplayBytes,
};
use crate::content::qualify_native_source_spell_world_with_gameplay;
use crate::foundation::{
    ActorConditionTransition, ApplicationFacts, ChannelContentPin, ChannelId, ConditionDefinition,
    ConditionSource, ConditionSourceKind, ConditionValues, NodeId, WorldId,
};

fn uuid(raw: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = raw;
    bytes
}

fn at(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

fn pinned(bytes: &[u8]) -> PinnedGameplayBytes {
    PinnedGameplayBytes {
        bytes: bytes.to_vec(),
        sha256: hex(&Sha256::digest(bytes)),
    }
}

fn room() -> QualifiedNativeEntryRoom {
    let input = NativeGameplayInput {
        native_map_profile: NativeGameplayMapProfile::AcceptedEntryR1,
        catalog: pinned(include_bytes!(
            "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
        )),
        source_selection: pinned(include_bytes!(
            "../../../../tools/content-schema/spell-authoring/samples/executable-spell-source-selection.json"
        )),
        creature_profiles: pinned(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/creature_profiles.json"
        )),
        presentation_profiles: pinned(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/presentation_profiles.json"
        )),
        item_profiles: None,
        spell_appearances: None,
        build_training: None,
        familiar_config: None,
        familiar_defenses: None,
        wheel_profile: None,
        source_world: None,
        progression: None,
        item_keys: None,
        loot_tables: None,
    };
    qualify_native_source_spell_world_with_gameplay(
        WorldId::decode(&uuid(1)).unwrap(),
        &input,
        include_bytes!("../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"),
    )
    .unwrap()
}

/// The Thalom world with one entered player, outside a protection zone.
fn owner() -> (
    QualifiedNativeEntryRoom,
    ChannelRuntimeV1,
    ExactActorRef,
    GameSessionId,
) {
    let room = room();
    let pin = ChannelContentPin::from_activation(
        room.movement_cells().scope().world_id,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 11, -7),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        room.movement_cells().scope().world_id,
        ChannelId::decode(&uuid(2)).unwrap(),
        NodeId::decode(&uuid(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        2,
        pin,
    )
    .unwrap();
    let session = GameSessionId::decode(&uuid(4)).unwrap();
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let player = runtime.commit_fresh_session(reservation).unwrap();
    runtime.initialize_first_entry_position(player).unwrap();
    (room, runtime, player, session)
}

/// Commits `gate` for 10 s from time 0 to the player's runtime condition slot.
fn gate(runtime: &mut ChannelRuntimeV1, player: ExactActorRef, session: GameSessionId, key: &str) {
    let root = GameplayDecisionRoot::from_bytes([7; 32]);
    let definition = ConditionDefinition::new(
        "cond.gate",
        1,
        ConditionValues::gate(key, 10_000).expect("gate family"),
    )
    .unwrap();
    let plan = runtime
        .prepare_actor_condition(
            player,
            Some(session),
            ActorConditionTransition::Apply {
                definition: &definition,
                source: ConditionSource {
                    actor: player,
                    session: Some(session),
                    kind: ConditionSourceKind::SelfUse,
                },
                immunities: &[],
                facts: ApplicationFacts {
                    now: 0,
                    base_speed: 220,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([1; 16]),
                },
            },
            at(0),
        )
        .unwrap();
    runtime.commit_actor_condition(&plan, at(0)).unwrap();
}

/// An entity the room does not show: an ungated actor gets `TARGET_NOT_VISIBLE`.
const UNSEEN: EntityRef = EntityRef {
    identity: [9; 16],
    generation: 1,
};

#[test]
fn pacified_refuses_a_target_before_the_visibility_check_and_still_lets_the_actor_stop() {
    let (room, mut runtime, player, session) = owner();
    let mut attack = ChannelAttackStates::default();
    let mut target = |runtime: &ChannelRuntimeV1, now: u64, target: Option<EntityRef>| {
        attack.set_target(runtime, Some(&room), player, session, at(now), target, 1)
    };
    assert_eq!(
        target(&runtime, 1, Some(UNSEEN)),
        AttackIntentDisposition::TargetNotVisible
    );
    gate(&mut runtime, player, session, "pacified");
    assert!(pacified_slot(&runtime, player, session, at(1)));
    assert_eq!(
        target(&runtime, 1, Some(UNSEEN)),
        AttackIntentDisposition::Rejected
    );
    assert_eq!(AttackIntentDisposition::Rejected as u8, 6);
    assert_eq!(target(&runtime, 1, None), AttackIntentDisposition::Ok);
    // Off the gate's duration the refusal ends.
    assert!(!pacified_slot(&runtime, player, session, at(10_000)));
    assert_eq!(
        target(&runtime, 10_000, Some(UNSEEN)),
        AttackIntentDisposition::TargetNotVisible
    );
}

#[test]
fn muted_and_powerless_do_not_gate_an_attack_target() {
    for key in ["muted", "powerless"] {
        let (room, mut runtime, player, session) = owner();
        gate(&mut runtime, player, session, key);
        assert!(!pacified_slot(&runtime, player, session, at(1)));
        assert_eq!(
            ChannelAttackStates::default().set_target(
                &runtime,
                Some(&room),
                player,
                session,
                at(1),
                Some(UNSEEN),
                1,
            ),
            AttackIntentDisposition::TargetNotVisible
        );
    }
}
