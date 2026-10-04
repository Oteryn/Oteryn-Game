//! ATTACK-1b (ATTACK-0 §3, §4, §9): the Channel owner's attack entries on a real
//! `ChannelRuntimeV1` with a native qualification rat, the two intent windows and the domain 10
//! continuity. The swing commit itself is tested in `runtime_actor_carrier::attacker_fence_tests`
//! and the composed kill in `tests/support/attack_kill_reward_postgres_cases.rs`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::content::native_gameplay::{
    NativeGameplayInput, NativeGameplayMapProfile, PinnedGameplayBytes,
};
use crate::content::qualify_native_source_spell_world_with_gameplay;
use crate::foundation::{
    ChannelContentPin, ChannelId, CompiledCreaturePolicies, CompiledCreaturePolicy,
    ControlLossMark, CreatureFlags, NodeId, WorldId,
};
use oteryn_protocol_oteryn::attack::ChaseMode;

fn uuid(raw: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = raw;
    bytes
}

fn at(micros: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(micros)
}

const T0: u64 = 10_000_000;

fn rat_policy(digest: [u8; 32]) -> CompiledCreaturePolicies {
    CompiledCreaturePolicies::from_active_artifact(
        digest,
        vec![CompiledCreaturePolicy {
            definition_key: "fixture:rat".into(),
            definition_revision: "fixture:1".into(),
            display_name: "fixture Rat".into(),
            maximum_health: 20,
            base_speed: 110,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: vec![],
            armor: Some(1),
            mitigation: None,
            resistances: vec![],
            damage_immunities: vec![],
            preferred_distance: Some(1),
            reward_boss: Some(false),
            flags: CreatureFlags {
                attackable: true,
                illusionable: false,
                health_hidden: false,
            },
        }],
    )
    .expect("qualified fixture table")
}

fn pinned(bytes: &[u8]) -> PinnedGameplayBytes {
    PinnedGameplayBytes {
        bytes: bytes.to_vec(),
        sha256: hex(&Sha256::digest(bytes)),
    }
}

/// The source-qualified Thalom spell world: its tiles carry the protection-zone flags.
fn source_world() -> QualifiedNativeEntryRoom {
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
    };
    qualify_native_source_spell_world_with_gameplay(
        WorldId::decode(&uuid(1)).unwrap(),
        &input,
        include_bytes!("../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"),
    )
    .unwrap()
}

/// Thalom cells around the qualification spawn: (1,10) is in a protection zone, (1,11) and
/// (2,11) are not.
const OPEN_PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const OPEN_RAT: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};
const PROTECTED: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 10,
    floor: -7,
};

/// The Thalom world with a player entering at `player_at` and a rat realized through the
/// qualification spawn path at `rat_at`.
struct Owner {
    room: QualifiedNativeEntryRoom,
    runtime: ChannelRuntimeV1,
    player: ExactActorRef,
    session: GameSessionId,
    rat: EntityRef,
    attack: ChannelAttackStates,
}

fn owner() -> Owner {
    owner_at(OPEN_PLAYER, OPEN_RAT)
}

fn owner_at(player_at: MovementLocalPosition, rat_at: MovementLocalPosition) -> Owner {
    let room = source_world();
    let pin = ChannelContentPin::from_activation(
        room.movement_cells().scope().world_id,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (player_at.x, player_at.y, player_at.floor),
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
        pin.clone(),
    )
    .unwrap();
    runtime
        .install_companion_policies(rat_policy(pin.server_artifact_digest()))
        .unwrap();
    let session = GameSessionId::decode(&uuid(4)).unwrap();
    let reservation = runtime.reserve_fresh_session(session).unwrap();
    let player = runtime.commit_fresh_session(reservation).unwrap();
    runtime.initialize_first_entry_position(player).unwrap();
    assert_eq!(
        runtime.read_actor_position(player).unwrap().position(),
        player_at
    );
    runtime
        .realize_native_qualification_spawn(pin, "fixture:rat", rat_at)
        .expect("the rat spawns through the qualification path");
    let creature = runtime
        .visible_entities()
        .creatures
        .first()
        .copied()
        .expect("the rat is visible");
    let rat = EntityRef {
        identity: creature.actor.placement_identity(),
        generation: creature.generation,
    };
    Owner {
        room,
        runtime,
        player,
        session,
        rat,
        attack: ChannelAttackStates::default(),
    }
}

impl Owner {
    fn target(
        &mut self,
        now: u64,
        target: Option<EntityRef>,
        command: u64,
    ) -> AttackIntentDisposition {
        self.attack.set_target(
            &self.runtime,
            Some(&self.room),
            self.player,
            self.session,
            at(now),
            target,
            command,
        )
    }

    fn held(&self) -> Option<EntityRef> {
        self.attack
            .combat_state(self.player, self.session, at(T0))
            .target
    }

    fn entry(&self) -> &ActorAttack {
        &self.attack.entries[self.attack.index(self.player, self.session).unwrap()]
    }
}

#[test]
fn a_visible_rat_is_held_and_a_new_target_starts_its_own_lineage_at_ordinal_zero() {
    let mut owner = owner();
    let rat = owner.rat;
    assert_eq!(owner.target(T0, Some(rat), 7), AttackIntentDisposition::Ok);
    assert_eq!(owner.held(), Some(rat));
    let entry = owner.entry();
    assert_eq!(entry.next_ordinal, 0);
    // Holding a target is not a fight: the deadline starts with the first swing or hit.
    assert!(
        !owner
            .attack
            .combat_state(owner.player, owner.session, at(T0))
            .in_fight
    );
    assert_eq!(
        owner
            .attack
            .in_fight_until(owner.player, owner.session, at(T0)),
        None
    );
    owner.attack.entries[0].next_ordinal = 3;
    assert_eq!(owner.target(T0, Some(rat), 8), AttackIntentDisposition::Ok);
    assert_eq!(
        owner.entry().next_ordinal,
        0,
        "a new lineage restarts the ordinal"
    );
    // `None` stops attacking.
    assert_eq!(owner.target(T0, None, 9), AttackIntentDisposition::Ok);
    assert_eq!(owner.held(), None);
}

#[test]
fn a_player_an_unknown_or_stale_entity_and_a_missing_room_are_refused_and_keep_the_held_target() {
    let mut owner = owner();
    let rat = owner.rat;
    assert_eq!(owner.target(T0, Some(rat), 1), AttackIntentDisposition::Ok);
    let player = owner
        .runtime
        .visible_entities()
        .players
        .first()
        .copied()
        .unwrap();
    let player_ref = EntityRef {
        identity: player.actor.placement_identity(),
        generation: player.generation,
    };
    // PvP is out of scope: a player target is refused, the attacker itself included.
    assert_eq!(
        owner.target(T0, Some(player_ref), 2),
        AttackIntentDisposition::TargetNotACreature
    );
    let stale = EntityRef {
        generation: rat.generation + 1,
        ..rat
    };
    assert_eq!(
        owner.target(T0, Some(stale), 3),
        AttackIntentDisposition::TargetNotVisible
    );
    let unknown = EntityRef {
        identity: [0xAB; 16],
        generation: 1,
    };
    assert_eq!(
        owner.target(T0, Some(unknown), 4),
        AttackIntentDisposition::TargetNotVisible
    );
    assert_eq!(
        owner.attack.set_target(
            &owner.runtime,
            None,
            owner.player,
            owner.session,
            at(T0),
            Some(rat),
            5,
        ),
        AttackIntentDisposition::Rejected
    );
    assert_eq!(
        owner.target(T0, Some(rat), 0),
        AttackIntentDisposition::Rejected,
        "command 0"
    );
    assert_eq!(
        owner.held(),
        Some(rat),
        "a refusal leaves the held target unchanged"
    );
}

#[test]
fn a_session_that_does_not_control_the_actor_is_rejected_and_holds_no_entry() {
    let mut owner = owner();
    let other = GameSessionId::decode(&uuid(9)).unwrap();
    assert_eq!(
        owner.attack.set_target(
            &owner.runtime,
            Some(&owner.room),
            owner.player,
            other,
            at(T0),
            Some(owner.rat),
            1,
        ),
        AttackIntentDisposition::Rejected
    );
    assert!(
        !owner
            .attack
            .set_modes(&owner.runtime, owner.player, other, FightModes::DEFAULT)
    );
    assert!(owner.attack.entries.is_empty());
}

#[test]
fn reentry_protection_refuses_a_target_for_four_seconds() {
    let mut owner = owner();
    owner
        .runtime
        .record_control_loss(
            owner.player,
            owner.session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 1,
            },
        )
        .unwrap();
    owner
        .runtime
        .restore_control_after_committed_reentry(owner.player, owner.session, 1, T0)
        .unwrap();
    let rat = owner.rat;
    assert_eq!(
        owner.target(T0 + 3_999_999, Some(rat), 1),
        AttackIntentDisposition::ReentryProtected
    );
    assert_eq!(owner.held(), None);
    assert_eq!(
        owner.target(T0 + 4_000_000, Some(rat), 2),
        AttackIntentDisposition::Ok
    );
}

#[test]
fn a_rat_in_a_protection_zone_and_an_attacker_in_one_are_refused() {
    let mut owner = owner_at(OPEN_PLAYER, PROTECTED);
    let rat = owner.rat;
    assert_eq!(
        owner.target(T0, Some(rat), 1),
        AttackIntentDisposition::ProtectionZone
    );
    assert_eq!(owner.held(), None);
    let mut owner = owner_at(PROTECTED, OPEN_RAT);
    let rat = owner.rat;
    assert_eq!(
        owner.target(T0, Some(rat), 1),
        AttackIntentDisposition::ProtectionZone
    );
    assert_eq!(owner.held(), None);
}

#[test]
fn every_target_refusal_has_its_wire_disposition() {
    use AttackIntentDisposition as D;
    for (refusal, disposition) in [
        (TargetRefusal::NotVisible, D::TargetNotVisible),
        (TargetRefusal::Dead, D::TargetNotVisible),
        (TargetRefusal::NotAttackableKind, D::TargetNotACreature),
        (TargetRefusal::TargetInProtectionZone, D::ProtectionZone),
        (TargetRefusal::AttackerInProtectionZone, D::ProtectionZone),
        (TargetRefusal::ReentryProtected, D::ReentryProtected),
        (TargetRefusal::TargetReentryProtected, D::ReentryProtected),
    ] {
        assert_eq!(refusal_disposition(refusal), disposition, "{refusal:?}");
    }
}

#[test]
fn the_qualification_rat_is_attackable_outside_a_protection_zone() {
    let owner = owner();
    let rat = owner.runtime.visible_entities().creatures[0].actor;
    let facts = target_facts(
        &owner.runtime,
        &owner.room,
        owner.player,
        owner.session,
        rat,
        at(T0),
    )
    .expect("the attacker's facts are readable");
    assert!(facts.present_and_visible && facts.attackable_kind && facts.alive && facts.same_floor);
    assert_eq!(facts.distance, 1);
    assert!(!facts.attacker_in_protection_zone && !facts.target_in_protection_zone);
    assert!(!facts.attacker_reentry_protected);
}

#[test]
fn a_hit_taken_starts_the_sixty_second_deadline_and_clearing_the_target_keeps_it() {
    let mut owner = owner();
    let rat = owner.rat;
    assert_eq!(owner.target(T0, Some(rat), 1), AttackIntentDisposition::Ok);
    owner
        .attack
        .record_hit_taken(&owner.runtime, owner.player, owner.session, at(T0));
    let deadline = at(T0 + 60_000_000);
    let (player, session) = (owner.player, owner.session);
    // 59.999 s after the hit the logout is refused and a closed client is held.
    assert_eq!(
        owner
            .attack
            .in_fight_until(player, session, at(T0 + 59_999_000)),
        Some(deadline)
    );
    assert!(
        owner
            .attack
            .combat_state(player, session, at(T0 + 59_999_000))
            .in_fight
    );
    // A reconnect stops attacking but keeps the deadline.
    owner.attack.clear_target(player, session);
    assert_eq!(owner.held(), None);
    assert_eq!(
        owner
            .attack
            .in_fight_until(player, session, at(T0 + 59_999_999)),
        Some(deadline)
    );
    // At 60 s the actor is released.
    assert_eq!(owner.attack.in_fight_until(player, session, deadline), None);
    assert!(
        !owner
            .attack
            .combat_state(player, session, deadline)
            .in_fight
    );
    // A later hit runs the deadline from that hit.
    owner
        .attack
        .record_hit_taken(&owner.runtime, player, session, at(T0 + 70_000_000));
    assert_eq!(
        owner
            .attack
            .in_fight_until(player, session, at(T0 + 70_000_000)),
        Some(at(T0 + 130_000_000))
    );
}

#[test]
fn fight_modes_are_runtime_state_with_the_admission_defaults() {
    let mut owner = owner();
    let (player, session) = (owner.player, owner.session);
    assert_eq!(
        owner.attack.combat_state(player, session, at(T0)),
        ActorCombatState {
            target: None,
            modes: FightModes::DEFAULT,
            in_fight: false,
        }
    );
    let modes = FightModes {
        fight_mode: WireFightMode::Offensive,
        chase: ChaseMode::Stand,
        secure: false,
    };
    assert!(
        owner
            .attack
            .set_modes(&owner.runtime, player, session, modes)
    );
    assert_eq!(
        owner.attack.combat_state(player, session, at(T0)).modes,
        modes
    );
    assert_eq!(fight_mode(WireFightMode::Offensive), FightMode::Offensive);
    assert_eq!(fight_mode(WireFightMode::Balanced), FightMode::Balanced);
    assert_eq!(fight_mode(WireFightMode::Defensive), FightMode::Defensive);
}

#[test]
fn each_intent_window_admits_twenty_five_per_second_and_survives_a_reconnect() {
    let start = Instant::now();
    let mut continuity = CombatContinuity::FRESH;
    for _ in 0..25 {
        assert!(continuity.target_intents.admit(start));
    }
    assert!(
        !continuity.target_intents.admit(start),
        "the 26th intent in one second"
    );
    // The two limits are independent.
    assert!(continuity.mode_intents.admit(start));
    // A reconnect resumes the same continuity: the window is not reset.
    let mut resumed = continuity;
    assert!(
        !resumed
            .target_intents
            .admit(start + Duration::from_millis(999))
    );
    assert!(resumed.target_intents.admit(start + ATTACK_INTENT_WINDOW));
    assert!(
        !CombatContinuity::FRESH
            .target_intents
            .0
            .iter()
            .any(Option::is_some)
    );
}

#[test]
fn domain_ten_sends_a_delta_only_for_a_changed_state_and_never_reuses_a_revision() {
    let mut continuity = CombatContinuity::FRESH;
    let idle = ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    };
    let (revision, payload) = continuity.snapshot(&idle).expect("snapshot");
    assert_eq!(revision, 1);
    assert_eq!(payload, encode_actor_combat_state(&idle).unwrap());
    // Building a snapshot records nothing until it is sent.
    assert_eq!(continuity.revision, 0);
    continuity.snapshot_sent(revision, idle);
    assert_eq!(continuity.delta(1, 10, idle), Some(None));
    let fighting = ActorCombatState {
        in_fight: true,
        ..idle
    };
    let (sequence, frame) = continuity
        .delta(1, 10, fighting)
        .expect("no fault")
        .expect("a changed state");
    assert_eq!(sequence, 11);
    assert!(!frame.is_empty());
    assert_eq!((continuity.revision, continuity.sent), (2, Some(fighting)));
    assert_eq!(continuity.delta(1, 11, fighting), Some(None));
    // A later snapshot is above every revision this session has seen.
    assert_eq!(
        continuity.snapshot(&fighting).map(|(revision, _)| revision),
        Some(3)
    );
}

#[test]
fn a_source_world_floor_is_seen_on_its_own_plane_only() {
    let at = |x, y, floor| MovementLocalPosition { x, y, floor };
    assert!(sees(at(2, 11, -7), at(1, 11, -7)));
    assert!(sees(at(0, 0, -7), at(9, 6, -7)));
    assert!(
        !sees(at(0, 0, -7), at(-9, 0, -7)),
        "outside the 18 x 14 view"
    );
    assert!(!sees(at(2, 11, -7), at(2, 11, -6)), "another native floor");
    // Visibility floors keep the reference rules, other floors included.
    assert!(sees(at(0, 0, 7), at(1, 0, 7)));
    assert!(sees(at(5, 5, 7), at(4, 4, 6)));
}
