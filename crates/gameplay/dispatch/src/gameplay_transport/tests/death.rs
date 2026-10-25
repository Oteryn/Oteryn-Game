/// DEATH-2b: only a cell that is Walkable in the pinned generation's own movement cells of
/// this World, and not a closed door, is admissible; a blocked or absent cell, a door and a
/// foreign World or generation are not, and an undecodable record has no cell.
#[test]
fn a_respawn_cell_must_be_walkable_unblocked_and_of_the_pinned_generation() {
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
        EngineeringStaticCellScope,
    };
    use crate::content::{
        CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
        MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
    };
    use crate::foundation::MovementLocalPosition;
    use std::collections::BTreeSet;
    let world = WorldId::decode(&CHARACTER).expect("world");
    let scope = EngineeringStaticCellScope {
        world_id: world,
        coordinate_frame: CoordinateFrameRef::new("death-2b-frame").expect("frame"),
        map_revision: MapRevisionRef::new("death-2b-map").expect("map"),
        generation_digest: [9; 32],
        content_lock: ContentLockBinding {
            revision_digest_token: ProductionAtom::new("lock", "death-2b-lock").expect("lock"),
            entries: vec![ContentLockEntry::exact(
                ProductionKey::new("engineering:death-2b").expect("key"),
                ProductionAtom::new("revision", "death-2b-r1").expect("revision"),
                Sha256HexDigest::new(&"b".repeat(64)).expect("digest"),
            )],
        },
    };
    let claim = |x, collision| EngineeringStaticCellClaim {
        scope: scope.clone(),
        cell: LogicalCell { x, y: 4, z: 7 },
        collision: EngineeringCollisionClaim::Qualified(collision),
    };
    let index = EngineeringStaticCellIndex::from_claims(vec![
        claim(3, CollisionClass::Walkable),
        claim(5, CollisionClass::Walkable),
        claim(6, CollisionClass::Blocked),
    ])
    .expect("index");
    let doors = BTreeSet::from([LogicalCell { x: 5, y: 4, z: 7 }]);
    let at = |x| MovementLocalPosition { x, y: 4, floor: 7 };
    let admit = |world, digest, cell| {
        respawn_cell_admissible(&index, &scope, world, digest, &doors, cell)
    };
    assert!(admit(world, [9; 32], at(3)));
    assert!(!admit(world, [9; 32], at(6)), "blocked cell");
    assert!(!admit(world, [9; 32], at(4)), "absent cell");
    assert!(!admit(world, [9; 32], at(5)), "closed door");
    assert!(!admit(world, [8; 32], at(3)), "foreign generation");
    let foreign = WorldId::decode(&[
        0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x72, 0x22, 0x92, 0x22, 0x22, 0x22, 0x22, 0x22,
        0x22, 0x22,
    ])
    .expect("foreign world");
    assert!(!admit(foreign, [9; 32], at(3)), "foreign World");
    assert_eq!(
        respawn_cell(&cell_bytes(at(3))[..9]),
        None,
        "undecodable record"
    );
}

/// DEATH-2b: an admission is playable after a pending respawn only when the consumption
/// deleted exactly the placed occurrence; a consumption error, an already consumed row or
/// another occurrence refuses the first entry, so the actor stays input-ineligible and the
/// respawn is not replayed by this admission.
#[test]
fn an_unsettled_respawn_refuses_the_playable_first_entry() {
    use crate::durability::character_death::{ConsumedRespawn, PlayerDeathOccurrence};
    let placed = PlayerDeathOccurrence::from_bytes(CHARACTER).expect("placed");
    let mut other_bytes = CHARACTER;
    other_bytes[15] = 0x12;
    let other = PlayerDeathOccurrence::from_bytes(other_bytes).expect("other");
    let row = |occurrence| {
        Ok::<_, &str>(Some(ConsumedRespawn {
            occurrence,
            respawn_position: vec![0; 10],
        }))
    };
    assert!(admitted_respawn_consumed(placed, row(placed)));
    assert!(!admitted_respawn_consumed(placed, Err("consumption")));
    assert!(!admitted_respawn_consumed(placed, Ok::<_, &str>(None)));
    assert!(!admitted_respawn_consumed(placed, row(other)));
    for entry in [FirstEntryOutcome::Positioned, FirstEntryOutcome::Reconciled] {
        assert_eq!(respawned_first_entry(entry, true), entry);
        assert_eq!(
            respawned_first_entry(entry, false),
            FirstEntryOutcome::RefusedByChannel
        );
    }
}

/// DEATH-ML-LOSS-1 (#2193 Codex P1 4306417846, 4307011952): a live death settles only on a
/// durable or empty training flush; a fenced-out flush keeps the live actor dead, and only
/// the actor-end release settles past it, and only when the fence left no training in
/// memory. An unknown flush never settles.
#[test]
fn a_fenced_out_training_flush_refuses_a_live_death() {
    use super::actor_spell::TrainingSave;
    for releasing in [false, true] {
        for training_left in [false, true] {
            assert!(death_flush_permits_settle(
                TrainingSave::Saved,
                releasing,
                training_left
            ));
            assert!(death_flush_permits_settle(
                TrainingSave::NotApplicable,
                releasing,
                training_left
            ));
            assert!(!death_flush_permits_settle(
                TrainingSave::Unknown,
                releasing,
                training_left
            ));
        }
        // #2193 Codex P1 4307011952: a refused write that keeps training in memory never
        // settles, on the live or the release path.
        assert!(!death_flush_permits_settle(
            TrainingSave::FencedOut,
            releasing,
            true
        ));
    }
    assert!(!death_flush_permits_settle(
        TrainingSave::FencedOut,
        false,
        false
    ));
    assert!(death_flush_permits_settle(
        TrainingSave::FencedOut,
        true,
        false
    ));
}

/// #2199: a familiar Login that did not complete refuses the playable first entry, so the
/// stored familiar and its cooldowns are never silently left unrestored.
#[test]
fn an_incomplete_familiar_login_refuses_the_playable_first_entry() {
    for entry in [FirstEntryOutcome::Positioned, FirstEntryOutcome::Reconciled] {
        assert_eq!(familiar_login_first_entry(entry, true), entry);
        assert_eq!(
            familiar_login_first_entry(entry, false),
            FirstEntryOutcome::RefusedByChannel
        );
    }
}

/// ARCH-PROGRESSION-SOURCE-0 §1.5 (#1964 Codex P1 4243646040): a positioned admission whose
/// progression rounds all ended undecided runs no later admission writer, even when storage
/// recovers right after the last round; the gate reads only the refused outcome. A bound or
/// unbound-but-decided step, and entries refused before the step, keep their quest admission.
#[test]
fn an_exhausted_progression_step_admits_no_later_writer() {
    for entry in [FirstEntryOutcome::Positioned, FirstEntryOutcome::Reconciled] {
        assert!(writes_after_progression(true, entry));
    }
    // Exhausted rounds: the step left the positioned entry `RefusedUnavailable`.
    let exhausted = FirstEntryOutcome::RefusedUnavailable;
    assert!(!writes_after_progression(true, exhausted));
    assert!(!matches!(
        exhausted,
        FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
    ));
    // Entries that never reached the step are unchanged by it.
    for entry in [
        FirstEntryOutcome::NotApplicable,
        FirstEntryOutcome::RefusedStaleAuthority,
        FirstEntryOutcome::RefusedUnavailable,
        FirstEntryOutcome::RefusedByChannel,
    ] {
        assert!(writes_after_progression(false, entry));
    }
    // An unsettled respawn after a bound step keeps the quest admission it had.
    assert!(writes_after_progression(
        true,
        respawned_first_entry(FirstEntryOutcome::Positioned, false)
    ));
}

/// DEATH-2: the durable death intent carries the occurrence, the death cell in this Channel
/// at the pinned map revision, the respawn position and the empty held set, both cells as the
/// big-endian `(x, y, floor)`.
#[test]
fn a_player_death_request_binds_the_cell_the_map_revision_and_the_respawn() {
    use crate::domain::progression::{
        FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
    };
    use crate::foundation::MovementLocalPosition;
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let context = ProgressionRevisionContext {
        profile: "profile-1".to_owned(),
        ruleset: "ruleset-1".to_owned(),
        content: "content-1".to_owned(),
        simulation: "simulation-1".to_owned(),
        evidence: "evidence-1".to_owned(),
        declaration: "declaration-1".to_owned(),
    };
    let progression = crate::combat::RewardProgressionBinding {
        context: context.clone(),
        policy_revision: "policy-1".to_owned(),
        reward_revision: "reward-1".to_owned(),
        policy: FiniteProgressionPolicy {
            context: context.clone(),
            policy_revision: "policy-1".to_owned(),
            reward_revision: "reward-1".to_owned(),
            death_policy_revision: "death-1".to_owned(),
            declared_difference_revision: "declaration-1".to_owned(),
            thresholds: [LevelThreshold {
                level: 1,
                minimum_experience: ExactI64::new(0),
            }],
            terminal_exclusive_experience: ExactI64::new(100),
            death_loss_numerator: 1,
            death_loss_denominator: 1,
            death_loss_rounding: RoundingMode::Floor,
        },
    };
    let occurrence =
        crate::durability::character_death::PlayerDeathOccurrence::from_bytes(CHARACTER)
            .expect("occurrence");
    let world = WorldId::decode(&CHARACTER).expect("world");
    let channel = ChannelId::decode(&CHARACTER).expect("channel");
    let request = player_death_request(
        &progression,
        (world, channel),
        [0xab; 32],
        actor_spell::PlayerDeath {
            occurrence,
            cell: MovementLocalPosition {
                x: -2,
                y: 258,
                floor: 7,
            },
        },
        MovementLocalPosition {
            x: 3,
            y: 4,
            floor: 7,
        },
    );
    assert_eq!(request.occurrence, occurrence);
    assert_eq!(request.policy, progression.policy);
    assert!(request.held_blessings.is_empty());
    assert_eq!(
        (request.death_cell.world_id, request.death_cell.channel_id),
        (world, channel)
    );
    assert_eq!(
        request.death_cell.spatial_position,
        [0xff, 0xff, 0xff, 0xfe, 0, 0, 1, 2, 0, 7]
    );
    assert_eq!(
        request.death_cell.map_revision,
        format!("map:{}", "ab".repeat(32))
    );
    assert_eq!(request.respawn_position, [0, 0, 0, 3, 0, 0, 0, 4, 0, 7]);
    // DEATH-2b: the admission reads the recorded cell back exactly; any other length is no
    // cell, so nothing is placed and the obligation stays.
    let cell = MovementLocalPosition {
        x: -2,
        y: 258,
        floor: -1,
    };
    assert_eq!(respawn_cell(&cell_bytes(cell)), Some(cell));
    assert_eq!(respawn_cell(&request.respawn_position[..9]), None);
    assert_eq!(respawn_cell(&[0; 11]), None);
}

#[test]
fn identifiers_are_fresh_uuid_v7_and_nonzero_transport() {
    let identifiers = SecureIdentifiers;
    let first = identifiers.game_session_id().expect("session");
    let second = identifiers.game_session_id().expect("session");
    assert_ne!(first, second);
    assert_eq!(first.as_bytes()[6] >> 4, 7);
    assert_ne!(identifiers.transport_ref(), identifiers.transport_ref());
    assert!(identifiers.transport_ref().is_some());
    let _ = (
        CharacterId::decode(&CHARACTER),
        AuthenticatedTransportRefV1::decode(&[1; 16]),
        GameSessionId::decode(&CHARACTER),
    );
}
