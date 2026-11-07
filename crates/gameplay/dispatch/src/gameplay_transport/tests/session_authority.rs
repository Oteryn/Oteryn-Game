#[test]
fn reconciled_session_must_still_be_the_untouched_fresh_admission() {
    use crate::foundation::{
        CharacterLease, ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
        ScopeOwnershipGeneration,
    };
    let session = GameSessionId::decode(&uuid_v7(0x21)).expect("session");
    let other = GameSessionId::decode(&uuid_v7(0x22)).expect("session");
    let character = CharacterId::decode(&CHARACTER).expect("character");
    let world = WorldId::decode(&uuid_v7(0x23)).expect("world");
    let channel = ChannelId::decode(&uuid_v7(0x24)).expect("channel");
    let facts =
        FreshAdmissionFacts::new([7; 32], character, world, channel, 1, 1).expect("facts");
    let commit = FreshAdmissionCommit::from_facts(session, facts, 9_u64).expect("commit");
    let snapshot = |state, generation, transport| {
        GameSessionAuthoritySnapshot::new(
            commit,
            state,
            ConnectionGeneration::new(generation).expect("generation"),
            transport,
            CharacterLease::new(character, 1).expect("lease"),
            ScopeOwnershipGeneration::new(1).expect("scope"),
        )
    };
    let admitted = snapshot(GameSessionState::Active, 1, Some(9));
    assert!(owns_fresh_session(admitted, session, 9));
    assert!(!owns_fresh_session(admitted, session, 8));
    assert!(!owns_fresh_session(admitted, other, 9));
    let rebound = snapshot(GameSessionState::Active, 2, Some(8));
    assert!(!owns_fresh_session(rebound, session, 9));
    let lost = snapshot(GameSessionState::Reconnectable, 1, None);
    assert!(!owns_fresh_session(lost, session, 9));
    let terminal = snapshot(GameSessionState::Terminal, 1, None);
    assert!(!owns_fresh_session(terminal, session, 9));
}

/// C2: the item fence is exactly the current session's Character, GameSession, connection
/// generation, current Character lease generation, runtime scope and scope generation, so
/// a resume (a new connection generation) or a new lease yields a different fence.
#[test]
fn item_fence_is_the_current_session_binding() {
    use crate::foundation::{
        CharacterLease, ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
    };
    let session = GameSessionId::decode(&uuid_v7(0x41)).expect("session");
    let character = CharacterId::decode(&CHARACTER).expect("character");
    let world = WorldId::decode(&uuid_v7(0x43)).expect("world");
    let channel = ChannelId::decode(&uuid_v7(0x44)).expect("channel");
    let transport = AuthenticatedTransportRefV1::decode(&[9; 16]).expect("transport");
    let facts =
        FreshAdmissionFacts::new([7; 32], character, world, channel, 3, 5).expect("facts");
    let commit = FreshAdmissionCommit::from_facts(session, facts, transport).expect("commit");
    let scope = RuntimeScopeRefV1::channel(world, channel);
    let snapshot = |connection, lease| {
        GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            GameSessionState::Active,
            ConnectionGeneration::new(connection).expect("generation"),
            Some(transport),
            CharacterLease::new(character, lease).expect("lease"),
            Some(CharacterWorldEligibilityClaimV1::new(character, world)),
            scope,
            ScopeOwnershipGeneration::new(5).expect("scope"),
        )
        .expect("snapshot")
    };
    let fence = item_fence_of(snapshot(1, 3)).expect("fence");
    assert_eq!(
        fence,
        CurrentCharacterItemFence {
            character_id: domain::CharacterId::from_bytes(CHARACTER).expect("character"),
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).expect("generation"),
            character_lease_generation: 3,
            runtime_scope: scope,
            scope_ownership_generation: ScopeOwnershipGeneration::new(5).expect("scope"),
        }
    );
    let resumed = item_fence_of(snapshot(2, 3)).expect("fence");
    assert_eq!(resumed.connection_generation.get(), 2);
    assert_ne!(resumed, fence);
    assert_eq!(
        item_fence_of(snapshot(1, 4))
            .expect("fence")
            .character_lease_generation,
        4
    );
}

/// C2 routing over the real activated entry room: only the injected chest placement is
/// routed to the chest `USE`; the door and any other placement keep the door path. The
/// injection leaves the activated Content (and so its digests) unchanged, the chest stands
/// on the non-walkable `entry-north` cell in reach of the start cell, its reward and backpack
/// resolve with known stack classes (D82) and its claim names no achievement.
#[test]
fn use_routes_only_the_injected_entry_chest_away_from_the_door_path()
-> Result<(), Box<dyn Error>> {
    use crate::combat_pickup::resolve_item_definition_facts;
    use crate::content::{LogicalCell, accepted};
    use crate::durability::item_transfer::ItemStackClass;
    use crate::interaction_chest_use::{entry_chest, resolve_chest, with_entry_chest};
    use crate::world_runtime::ReferenceContentGeneration;

    let world = WorldId::decode(&uuid_v7(0x45))?;
    let room = crate::content::qualify_native_entry_room(world)
        .map_err(|error| format!("{error:?}"))?;
    let activated = room.door().clone();
    let chest = with_entry_chest(&activated).map_err(|error| format!("{error:?}"))?;
    assert_eq!(&activated, room.door());
    assert_eq!(
        ReferenceContentGeneration::from_content(&activated)?,
        ReferenceContentGeneration::from_content(room.door())?
    );

    let placement =
        ComposedFreshAdmission::chest_target(&chest, entry_chest::PLACEMENT.as_bytes())
            .ok_or("chest not routed")?;
    for other in [accepted::DOOR_CELL.0, "oteryn:cell/unknown", ""] {
        assert!(
            ComposedFreshAdmission::chest_target(&chest, other.as_bytes()).is_none(),
            "{other}"
        );
        assert!(ComposedFreshAdmission::chest_target(&activated, other.as_bytes()).is_none());
    }

    let north = accepted::CELLS[2];
    assert!(!north.4, "the chest cell must not be walkable");
    assert_eq!(
        placement.address.cell,
        LogicalCell {
            x: north.1,
            y: north.2,
            z: i32::from(north.3)
        }
    );
    let cells = std::collections::BTreeSet::from([placement.address.cell]);
    let start = accepted::CELLS[0];
    assert!(ComposedFreshAdmission::use_object_reachable(
        start.1,
        start.2,
        i32::from(start.3),
        &cells
    ));
    assert_eq!(entry_chest::CONTENT_REVISION, accepted::REVISIONS[0]);
    assert_eq!(entry_chest::MAP_REVISION, accepted::REVISIONS[1]);
    assert_eq!(entry_chest::RULESET_REVISION, accepted::REVISIONS[2]);
    assert_eq!(entry_chest::SIM_REVISION, accepted::REVISIONS[6]);

    let resolved = resolve_chest(&chest, &placement.key).map_err(|error| format!("{error}"))?;
    assert_eq!(resolved.claim.production_key, entry_chest::CLAIM);
    assert_eq!(resolved.quantity, entry_chest::REWARD_COUNT);
    assert_eq!(resolved.achievement, None);
    let reward = resolve_item_definition_facts(&chest, &resolved.reward_item)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        reward.stack,
        ItemStackClass::Stackable {
            proven_maximum: Some(100)
        }
    );
    let backpack = resolve_item_definition_facts(
        &chest,
        &crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: entry_chest::BACKPACK_ITEM.into(),
            revision_ref: entry_chest::DEFINITION_REVISION.into(),
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    assert_eq!(backpack.stack, ItemStackClass::NonStackable);
    assert_eq!(
        backpack.container_capacity,
        Some(u32::from(entry_chest::BACKPACK_CAPACITY))
    );
    assert!(backpack.container_slot_equip_pattern);
    let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded()
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        achievements
            .unbound_reward_claim_achievements(&chest)
            .is_empty()
    );
    Ok(())
}

/// #935 no-write matrix: each independently changed binding alone refuses
/// the first-entry write while every other binding stays valid.
#[test]
fn first_entry_refuses_every_single_changed_current_binding() {
    use crate::foundation::{
        CharacterLease, ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
    };
    let session = GameSessionId::decode(&uuid_v7(0x31)).expect("session");
    let other_session = GameSessionId::decode(&uuid_v7(0x32)).expect("session");
    let character = CharacterId::decode(&CHARACTER).expect("character");
    let world = WorldId::decode(&uuid_v7(0x33)).expect("world");
    let other_world = WorldId::decode(&uuid_v7(0x35)).expect("world");
    let channel = ChannelId::decode(&uuid_v7(0x34)).expect("channel");
    let transport = AuthenticatedTransportRefV1::decode(&[9; 16]).expect("transport");
    let facts =
        FreshAdmissionFacts::new([7; 32], character, world, channel, 3, 5).expect("facts");
    let commit = FreshAdmissionCommit::from_facts(session, facts, transport).expect("commit");
    let scope = RuntimeScopeRefV1::channel(world, channel);
    let generation = |value| ScopeOwnershipGeneration::new(value).expect("scope");
    let snapshot = |state, lease, eligibility, scope_generation| {
        GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            state,
            ConnectionGeneration::new(1).expect("generation"),
            Some(transport),
            CharacterLease::new(character, lease).expect("lease"),
            eligibility,
            scope,
            generation(scope_generation),
        )
        .expect("snapshot")
    };
    let eligible = Some(CharacterWorldEligibilityClaimV1::new(character, world));
    let valid = snapshot(GameSessionState::Active, 3, eligible, 5);
    let guard = |account: &str, guard_world, eligible, lease, holder| {
        AdmissionAuthorityGuardStateV1::Character {
            account_id: account.to_owned(),
            world_id: guard_world,
            eligible,
            lease_generation: lease,
            holder,
        }
    };
    let valid_guard = guard("account-1", world, true, 3, Some(session));
    let expected = FirstEntryExpectation {
        game_session_id: session,
        transport,
        world_id: world,
        channel_id: channel,
        account_id: "account-1",
        scope_generation: generation(5),
    };
    assert!(first_entry_authority_is_current(
        &expected,
        valid,
        Some(&valid_guard)
    ));

    // Session-side changes.
    for (label, current) in [
        (
            "not active",
            snapshot(GameSessionState::Reconnectable, 3, eligible, 5),
        ),
        ("ineligible", snapshot(GameSessionState::Active, 3, None, 5)),
        ("lease", snapshot(GameSessionState::Active, 4, eligible, 5)),
        (
            "scope generation",
            snapshot(GameSessionState::Active, 3, eligible, 6),
        ),
    ] {
        assert!(
            !first_entry_authority_is_current(&expected, current, Some(&valid_guard)),
            "{label}"
        );
    }
    // Current Character guard changes.
    for (label, changed) in [
        ("owner", guard("account-2", world, true, 3, Some(session))),
        (
            "guard world",
            guard("account-1", other_world, true, 3, Some(session)),
        ),
        (
            "guard ineligible",
            guard("account-1", world, false, 3, Some(session)),
        ),
        (
            "guard lease",
            guard("account-1", world, true, 4, Some(session)),
        ),
        (
            "holder",
            guard("account-1", world, true, 3, Some(other_session)),
        ),
        ("no holder", guard("account-1", world, true, 3, None)),
    ] {
        assert!(
            !first_entry_authority_is_current(&expected, valid, Some(&changed)),
            "{label}"
        );
    }
    assert!(!first_entry_authority_is_current(&expected, valid, None));
    // Expectation-side changes (another socket, World, Channel or runtime).
    let other_channel = ChannelId::decode(&uuid_v7(0x36)).expect("channel");
    let other_transport = AuthenticatedTransportRefV1::decode(&[8; 16]).expect("transport");
    for (label, changed) in [
        (
            "session",
            FirstEntryExpectation {
                game_session_id: other_session,
                ..expected
            },
        ),
        (
            "transport",
            FirstEntryExpectation {
                transport: other_transport,
                ..expected
            },
        ),
        (
            "world",
            FirstEntryExpectation {
                world_id: other_world,
                ..expected
            },
        ),
        (
            "channel",
            FirstEntryExpectation {
                channel_id: other_channel,
                ..expected
            },
        ),
        (
            "runtime generation",
            FirstEntryExpectation {
                scope_generation: generation(4),
                ..expected
            },
        ),
    ] {
        assert!(
            !first_entry_authority_is_current(&changed, valid, Some(&valid_guard)),
            "{label}"
        );
    }
}
