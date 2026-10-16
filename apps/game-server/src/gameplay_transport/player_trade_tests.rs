#![allow(clippy::unwrap_used)]
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemTradeRestrictions};
use crate::domain::player_trade::tradeable_flag_allows;
use crate::domain::timed_item::TimedValues;
use crate::domain::{AccountId, CharacterId};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::{OneItemStateV1, OneItemTypedDefinitionRevisionV1};
use crate::durability::item_timed_state::StoredTimedState;
use crate::durability::item_transfer::{
    BackpackEntry, ContainerEntryPosition, CurrentCharacterItemFence, InventoryItem,
};
use crate::foundation::{
    ChannelId, CommandId, ConnectionGeneration, ScopeOwnershipGeneration, WorldId,
};

fn uuid(tag: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = tag;
    id
}
fn command(side: Side, number: u64) -> CommandRef {
    CommandRef::new(
        GameSessionId::decode(&uuid(31 + side.index() as u8)).unwrap(),
        CommandId::new(number).unwrap(),
    )
}
fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item/{key}"),
        revision_ref: "r1".into(),
    }
}
fn data(counter: bool) -> TradeData {
    let traders = [0_u8, 1].map(|side| Trader {
        account_id: AccountId::from_bytes(uuid(41 + side)).unwrap(),
        fence: CurrentCharacterItemFence {
            character_id: CharacterId::from_bytes(uuid(1 + side)).unwrap(),
            game_session_id: GameSessionId::decode(&uuid(31 + side)).unwrap(),
            connection_generation: ConnectionGeneration::new(1).unwrap(),
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(&uuid(50)).unwrap(),
                ChannelId::decode(&uuid(51)).unwrap(),
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
        },
    });
    let backpacks = [0_u8, 1].map(|side| CharacterBackpack {
        backpack: InventoryItem {
            item_instance_id: uuid(10 + side),
            definition: definition("backpack"),
            quantity: 1,
        },
        entries: vec![BackpackEntry {
            item: InventoryItem {
                item_instance_id: uuid(20 + side),
                definition: definition("leaf"),
                quantity: 2,
            },
            placement_ordinal: 1,
        }],
    });
    let offers = [0_usize, 1].map(|side| {
        if side == 1 && !counter {
            return None;
        }
        let entry = &backpacks[side].entries[0];
        Some(LeafOffer {
            state_revision: 1,
            placed_transaction_id: uuid(80 + side as u8),
            state: OneItemStateV1 {
                item_instance_id: entry.item.item_instance_id.to_vec(),
                world_id: uuid(50).to_vec(),
                definition: Some(OneItemTypedDefinitionRevisionV1 {
                    family: "Item".into(),
                    production_key: "oteryn:item/leaf".into(),
                    revision_ref: "r1".into(),
                }),
                quantity: 2,
                lifecycle: 1,
                key_number: 0,
            },
            timed: None,
            position: ContainerEntryPosition {
                parent_item_instance_id: backpacks[side].backpack.item_instance_id,
                placement_ordinal: 1,
            },
        })
    });
    TradeData {
        traders,
        backpack_versions: backpacks.each_ref().map(|backpack| {
            std::iter::once(&backpack.backpack)
                .chain(backpack.entries.iter().map(|entry| &entry.item))
                .map(|item| BackpackItemVersion {
                    item: item.item_instance_id,
                    state_revision: 1,
                    placed_transaction_id: uuid(item.item_instance_id[15] + 60),
                    key_number: 0,
                    timed: None,
                })
                .collect()
        }),
        backpacks,
        offers,
    }
}
fn ready() -> TradeSession {
    let mut trade =
        TradeSession::record_revalidated_start(data(false), command(Side::First, 1), 0).unwrap();
    trade
        .record_revalidated_counter_offer(data(true), command(Side::Second, 2), 10)
        .unwrap();
    trade
}
fn transferring() -> TradeSession {
    let mut trade = ready();
    trade
        .record_revalidated_accept(
            Side::First,
            command(Side::Second, 2),
            command(Side::First, 3),
            &data(true),
            20,
        )
        .unwrap();
    trade
        .record_revalidated_accept(
            Side::Second,
            command(Side::Second, 2),
            command(Side::Second, 4),
            &data(true),
            30,
        )
        .unwrap();
    trade
}

#[test]
fn native_unknown_tradeable_is_allowed_independently_of_marketable() {
    let mut facts = ReferenceItemTradeRestrictions {
        tradeable: F::Unknown,
        marketable: F::Known(false),
        vocations: F::Unknown,
        account_binding_policy: F::Unknown,
        character_binding_policy: F::Unknown,
    };
    assert!(tradeable_flag_allows(&facts));
    facts.tradeable = F::Known(true);
    assert!(tradeable_flag_allows(&facts));
    for blocked in [F::Known(false), F::Conflict, F::NotApplicable] {
        facts.tradeable = blocked;
        assert!(!tradeable_flag_allows(&facts));
    }
}

#[test]
fn start_refuses_self_same_account_cross_channel_and_instance_scope() {
    assert!(
        TradeSession::record_revalidated_start(data(false), command(Side::First, 1), 0).is_ok()
    );
    for operator in 0..4 {
        let mut current = data(false);
        match operator {
            0 => current.traders[1].fence.character_id = current.traders[0].fence.character_id,
            1 => current.traders[1].account_id = current.traders[0].account_id,
            2 => {
                current.traders[1].fence.runtime_scope = RuntimeScopeRefV1::channel(
                    WorldId::decode(&uuid(50)).unwrap(),
                    ChannelId::decode(&uuid(52)).unwrap(),
                )
            }
            _ => {
                let scope =
                    RuntimeScopeRefV1::instance(WorldId::decode(&uuid(50)).unwrap(), uuid(60))
                        .unwrap();
                current.traders[0].fence.runtime_scope = scope;
                current.traders[1].fence.runtime_scope = scope;
            }
        }
        assert!(matches!(
            TradeSession::record_revalidated_start(current, command(Side::First, 1), 0),
            Err(TradeRefusal::Rejected)
        ));
    }
}

#[test]
fn counter_offer_and_two_accepts_preserve_the_exact_native_pair_and_trigger() {
    for order in [[Side::First, Side::Second], [Side::Second, Side::First]] {
        let mut trade = ready();
        assert_eq!(trade.phase(), Phase::Ready);
        trade
            .record_revalidated_accept(
                order[0],
                command(Side::Second, 2),
                command(order[0], 3),
                &data(true),
                20,
            )
            .unwrap();
        assert_eq!(trade.phase(), Phase::Ready);
        trade
            .record_revalidated_accept(
                order[1],
                command(Side::Second, 2),
                command(order[1], 4),
                &data(true),
                20,
            )
            .unwrap();
        let binding = trade.transfer().unwrap();
        assert_eq!(trade.phase(), Phase::Transferring);
        assert_eq!(binding.pair, command(Side::Second, 2));
        assert_eq!(binding.trigger, command(order[1], 4));
        assert_eq!(binding.accepts[order[0].index()], command(order[0], 3));
    }
}

#[test]
fn a_later_accept_from_the_same_side_cannot_replace_the_terminal_first_accept() {
    let mut trade = ready();
    for number in [3, 4] {
        trade
            .record_revalidated_accept(
                Side::First,
                command(Side::Second, 2),
                command(Side::First, number),
                &data(true),
                20,
            )
            .unwrap();
    }
    trade
        .record_revalidated_accept(
            Side::Second,
            command(Side::Second, 2),
            command(Side::Second, 5),
            &data(true),
            30,
        )
        .unwrap();
    assert_eq!(
        trade.transfer().unwrap().accepts[0],
        command(Side::First, 3)
    );
}

#[test]
fn stale_pair_command_session_and_native_fence_substitution_cannot_accept() {
    for operator in 0..5 {
        let mut trade = ready();
        let mut current = data(true); // Independent fixture read, not cloned from captured binding.
        let mut pair = command(Side::Second, 2);
        let mut accept = command(Side::First, 3);
        match operator {
            0 => pair = command(Side::Second, 9),
            1 => accept = command(Side::Second, 3),
            2 => {
                current.traders[0].fence.connection_generation =
                    ConnectionGeneration::new(2).unwrap()
            }
            3 => current.traders[0].fence.character_lease_generation = 2,
            _ => {
                current.traders[0].fence.scope_ownership_generation =
                    ScopeOwnershipGeneration::new(2).unwrap()
            }
        }
        assert_eq!(
            trade.record_revalidated_accept(Side::First, pair, accept, &current, 20),
            Err(TradeRefusal::Stale)
        );
        assert_eq!(
            trade.phase(),
            if operator < 2 {
                Phase::Ready
            } else {
                Phase::Closed(CloseReason::Stale)
            }
        );
        assert!(trade.transfer().is_none());
    }
}

#[test]
fn changed_native_state_position_or_destination_contents_cancels_acceptance() {
    for operator in 0..4 {
        let mut trade = ready();
        let mut current = data(true);
        match operator {
            0 => current.offers[0].as_mut().unwrap().state.key_number = 3,
            1 => {
                current.offers[0].as_mut().unwrap().timed = Some(StoredTimedState {
                    revision: 1,
                    values: Some(TimedValues::new(Some(1), None).unwrap()),
                    deadline_at: None,
                })
            }
            2 => {
                current.offers[0]
                    .as_mut()
                    .unwrap()
                    .position
                    .placement_ordinal = 2;
                current.backpacks[0].entries[0].placement_ordinal = 2;
            }
            _ => current.backpacks[0].entries.push(BackpackEntry {
                item: InventoryItem {
                    item_instance_id: uuid(70),
                    definition: definition("reward"),
                    quantity: 1,
                },
                placement_ordinal: 2,
            }),
        }
        assert_eq!(
            trade.record_revalidated_accept(
                Side::First,
                command(Side::Second, 2),
                command(Side::First, 3),
                &current,
                20
            ),
            Err(TradeRefusal::Stale)
        );
    }
}

#[test]
fn issued_change_cancel_and_reconnect_close_pre_transfer_sessions() {
    let mut offered =
        TradeSession::record_revalidated_start(data(false), command(Side::First, 1), 0).unwrap();
    offered.invalidate().unwrap();
    assert_eq!(offered.phase(), Phase::Closed(CloseReason::Cancelled));
    let mut trade = ready();
    trade
        .reconnect(Side::First, command(Side::First, 1).game_session_id())
        .unwrap();
    assert_eq!(trade.phase(), Phase::Closed(CloseReason::Cancelled));
}

#[test]
fn idle_deadline_is_exact_and_a_valid_accept_refreshes_it() {
    let mut trade = ready();
    trade.expire_idle(120_009).unwrap();
    assert_eq!(trade.phase(), Phase::Ready);
    trade.expire_idle(120_010).unwrap();
    assert_eq!(trade.phase(), Phase::Closed(CloseReason::Cancelled));
    let mut trade = ready();
    trade
        .record_revalidated_accept(
            Side::First,
            command(Side::Second, 2),
            command(Side::First, 3),
            &data(true),
            100,
        )
        .unwrap();
    trade.expire_idle(120_099).unwrap();
    assert_eq!(trade.phase(), Phase::Ready);
    trade.expire_idle(120_100).unwrap();
    assert_eq!(trade.phase(), Phase::Closed(CloseReason::Cancelled));
    let mut trade = ready();
    assert_eq!(trade.expire_idle(9), Err(TradeRefusal::Rejected));
    assert_eq!(trade.phase(), Phase::Ready);
}

#[test]
fn ambiguous_transfer_ignores_cancel_timeout_and_preserves_same_session_reconnect() {
    let mut trade = transferring();
    let binding = trade.transfer().unwrap().clone();
    trade
        .record_outcome_data(&binding, ObservedSwap::Unknown)
        .unwrap();
    assert_eq!(trade.invalidate(), Err(TradeRefusal::Transferring));
    trade.expire_idle(u64::MAX).unwrap();
    trade
        .reconnect(Side::First, command(Side::First, 1).game_session_id())
        .unwrap();
    assert_eq!(
        trade.reconnect(Side::First, GameSessionId::decode(&uuid(80)).unwrap()),
        Err(TradeRefusal::Stale)
    );
    assert_eq!(trade.phase(), Phase::Transferring);
    assert_eq!(trade.transfer(), Some(&binding));
    // Policy retention only: no actual item/entry reservation or DB outcome is authenticated.
}

#[test]
fn only_the_exact_transfer_binding_can_record_a_terminal_outcome() {
    let mut trade = transferring();
    let binding = trade.transfer().unwrap().clone();
    let mut other = binding.clone();
    other.trigger = command(Side::Second, 99);
    assert_eq!(
        trade.record_outcome_data(&other, ObservedSwap::Committed),
        Err(TradeRefusal::Stale)
    );
    assert_eq!(trade.phase(), Phase::Transferring);
    trade
        .record_outcome_data(&binding, ObservedSwap::Committed)
        .unwrap();
    assert_eq!(trade.phase(), Phase::Closed(CloseReason::Completed));
    for (outcome, reason) in [
        (ObservedSwap::NoRoom, CloseReason::NoRoom),
        (ObservedSwap::Stale, CloseReason::Stale),
        (ObservedSwap::Rejected, CloseReason::Rejected),
    ] {
        let mut trade = transferring();
        let binding = trade.transfer().unwrap().clone();
        trade.record_outcome_data(&binding, outcome).unwrap();
        assert_eq!(trade.phase(), Phase::Closed(reason));
    }
}

// Unchanged genuine native MINT/TRANSFER/recovery/session fixture. Its original standalone
// target allows expect_used at this same test-only boundary; no production lint is relaxed.
#[allow(clippy::expect_used)]
#[path = "../../tests/support/item_transfer_postgres_cases.rs"]
mod native;
use native::TestResult;
use std::time::Duration;

const OPEN_A: crate::foundation::MovementLocalPosition = crate::foundation::MovementLocalPosition {
    x: 2,
    y: 12,
    floor: -7,
};
const OPEN_B: crate::foundation::MovementLocalPosition = crate::foundation::MovementLocalPosition {
    x: 1,
    y: 12,
    floor: -7,
};

fn actual_runtime_positions(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    node: crate::foundation::NodeId,
    identities: [(GameSessionId, crate::foundation::CharacterId); 2],
    positions: [crate::foundation::MovementLocalPosition; 2],
    capacity: usize,
    assignment_revisions: (u64, u64),
) -> (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
    ChannelRuntimeV1,
    LocalObjectRuntime,
    [TradeActor; 2],
) {
    let (controller, pin) = crate::content::native_gameplay::tests::activated_with_loot_tables(
        world,
        Some(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    );
    let room = pin.qualified_room();
    let channel_pin = crate::foundation::ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (OPEN_A.x, OPEN_A.y, OPEN_A.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        assignment_revisions.0,
        1,
        assignment_revisions.1,
        &format!("runtime-scope-assignment:{}", assignment_revisions.1),
        capacity,
        channel_pin,
    )
    .unwrap();
    let actors = [0_usize, 1].map(|side| {
        let (session, character) = identities[side];
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(character, 1).unwrap(),
            )
            .unwrap();
        runtime
            .initialize_pinned_test_position(actor, positions[side])
            .unwrap();
        TradeActor {
            actor,
            fence: CurrentCharacterItemFence {
                character_id: CharacterId::from_bytes(*character.as_bytes()).unwrap(),
                game_session_id: session,
                connection_generation: ConnectionGeneration::new(1).unwrap(),
                character_lease_generation: 1,
                runtime_scope: RuntimeScopeRefV1::channel(world, channel),
                scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
            },
        }
    });
    let scope = RuntimeScopeRefV1::channel(world, channel);
    let door_content =
        crate::world_runtime::ReferenceContentGeneration::from_content(room.door()).unwrap();
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
        scope,
        ScopeOwnershipGeneration::new(1).unwrap(),
        door_content,
    );
    let objects = crate::world_runtime::bind_native_entry_door(
        room.door(),
        &door_fence,
        scope,
        ScopeOwnershipGeneration::new(1).unwrap(),
    )
    .unwrap();
    (controller, pin, runtime, objects, actors)
}
fn actual_runtime(
    world: crate::foundation::WorldId,
    channel: crate::foundation::ChannelId,
    node: crate::foundation::NodeId,
    identities: [(GameSessionId, crate::foundation::CharacterId); 2],
) -> (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
    ChannelRuntimeV1,
    LocalObjectRuntime,
    [TradeActor; 2],
) {
    actual_runtime_positions(
        world,
        channel,
        node,
        identities,
        [OPEN_A, OPEN_B],
        8,
        (1, 1),
    )
}
fn geometry_fixture() -> (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
    ChannelRuntimeV1,
    LocalObjectRuntime,
    [TradeActor; 2],
) {
    actual_runtime(
        WorldId::decode(&uuid(50)).unwrap(),
        ChannelId::decode(&uuid(51)).unwrap(),
        crate::foundation::NodeId::decode(&uuid(52)).unwrap(),
        [0_u8, 1].map(|side| {
            (
                GameSessionId::decode(&uuid(31 + side)).unwrap(),
                crate::foundation::CharacterId::decode(&uuid(1 + side)).unwrap(),
            )
        }),
    )
}

#[test]
fn runtime_pair_uses_real_actor_control_lease_and_current_native_map() {
    let (_generation, pin, mut runtime, objects, actors) = geometry_fixture();
    assert!(qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, actors).is_ok());
    for field in 0..4 {
        let mut bad = actors;
        match field {
            0 => bad[0].fence.game_session_id = GameSessionId::decode(&uuid(99)).unwrap(),
            1 => bad[0].fence.character_id = CharacterId::from_bytes(uuid(99)).unwrap(),
            2 => bad[0].fence.character_lease_generation += 1,
            _ => {
                bad[0].fence.scope_ownership_generation = ScopeOwnershipGeneration::new(2).unwrap()
            }
        }
        assert!(matches!(
            qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, bad),
            Err(OwnerError::Authority)
        ));
        assert!(qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, actors).is_ok());
    }
    runtime
        .record_control_loss(
            actors[0].actor,
            actors[0].fence.game_session_id,
            crate::foundation::ControlLossMark {
                epoch: 1,
                grace_deadline: 999999,
            },
        )
        .unwrap();
    assert!(matches!(
        qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, actors),
        Err(OwnerError::Authority)
    ));
}

#[test]
fn runtime_retired_owner_and_unknown_foreign_map_refuse_identical_actor_names() {
    let (_generation, pin, mut runtime, objects, actors) = geometry_fixture();
    let wrong =
        crate::content::qualify_native_entry_room(WorldId::decode(&uuid(90)).unwrap()).unwrap();
    assert!(matches!(
        qualify_runtime_pair(&runtime, &wrong, &objects, actors),
        Err(OwnerError::UnknownMap)
    ));
    assert!(qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, actors).is_ok());
    runtime.retire_owner_cycle();
    assert!(matches!(
        qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, actors),
        Err(OwnerError::Authority)
    ));
    assert!(matches!(
        TradeOwner::new(&runtime, Instant::now()),
        Err(OwnerError::Authority)
    ));
}

#[test]
fn offered_item_query_rejects_foreign_binding_and_retired_runtime_before_empty_book() {
    let (_, _, mut runtime, _, _) = geometry_fixture();
    let owner = TradeOwner::new(&runtime, Instant::now()).unwrap();
    assert!(!owner.item_reserved_by_trade(&runtime, uuid(240)).unwrap());
    let (_, _, foreign, _, _) = actual_runtime(
        WorldId::decode(&uuid(50)).unwrap(),
        ChannelId::decode(&uuid(53)).unwrap(),
        crate::foundation::NodeId::decode(&uuid(52)).unwrap(),
        [0_u8, 1].map(|side| {
            (
                GameSessionId::decode(&uuid(31 + side)).unwrap(),
                crate::foundation::CharacterId::decode(&uuid(1 + side)).unwrap(),
            )
        }),
    );
    assert!(matches!(
        owner.item_reserved_by_trade(&foreign, uuid(240)),
        Err(OwnerError::Authority)
    ));
    runtime.retire_owner_cycle();
    assert!(matches!(
        owner.item_reserved_by_trade(&runtime, uuid(240)),
        Err(OwnerError::Authority)
    ));
}

#[test]
fn native_state_and_placement_versions_cancel_same_looking_leaf_and_backpack() {
    for operator in 0..4 {
        let mut trade = ready();
        let mut fresh = data(true);
        match operator {
            0 => fresh.offers[0].as_mut().unwrap().state_revision += 1,
            1 => fresh.offers[0].as_mut().unwrap().placed_transaction_id = uuid(99),
            2 => fresh.backpack_versions[1][0].state_revision += 1,
            _ => {
                fresh.backpack_versions[1][1].timed = Some(StoredTimedState {
                    revision: 1,
                    values: Some(TimedValues::new(Some(1), None).unwrap()),
                    deadline_at: None,
                })
            }
        }
        assert_eq!(
            trade.record_revalidated_accept(
                Side::First,
                command(Side::Second, 2),
                command(Side::First, 3),
                &fresh,
                20
            ),
            Err(TradeRefusal::Stale)
        );
        assert_eq!(trade.phase(), Phase::Closed(CloseReason::Stale));
    }
}

#[test]
fn owner_local_index_expiry_invalidation_and_ambiguity_preserve_physical_gap() {
    let (_generation, _pin, runtime, _objects, actors) = geometry_fixture();
    let origin = Instant::now();
    let mut owner = TradeOwner::new(&runtime, origin).unwrap();
    let key = trade_key(command(Side::First, 1));
    // Pure book fixture, NOT a successful native offer/read or physical reservation.
    owner.trades.insert(
        key,
        OwnedTrade {
            actors,
            session: ready(),
            intent: None,
        },
    );
    for actor in actors {
        owner.by_actor.insert(actor.actor.placement_identity(), key);
    }
    owner
        .expire(origin + Duration::from_millis(120_009))
        .unwrap();
    assert_eq!(owner.phase(actors[0].actor), Some(Phase::Ready));
    owner
        .expire(origin + Duration::from_millis(120_010))
        .unwrap();
    assert!(owner.trades.is_empty() && owner.by_actor.is_empty());
    owner.trades.insert(
        key,
        OwnedTrade {
            actors,
            session: transferring(),
            intent: None,
        },
    );
    for actor in actors {
        owner.by_actor.insert(actor.actor.placement_identity(), key);
    }
    let binding = owner.pending_transfer(actors[0].actor).unwrap().clone();
    owner
        .note_ambiguous_outcome(actors[0].actor, &binding)
        .unwrap();
    owner.expire(origin + Duration::from_secs(999)).unwrap();
    assert!(matches!(
        owner.invalidate(actors[0].actor),
        Err(OwnerError::Policy(TradeRefusal::Transferring))
    ));
    assert!(matches!(
        owner.invalidate_item(uuid(20)),
        Err(OwnerError::Policy(TradeRefusal::Transferring))
    ));
    owner
        .reconnect(actors[0].actor, command(Side::First, 1).game_session_id())
        .unwrap();
    assert!(matches!(
        owner.reconnect(actors[0].actor, GameSessionId::decode(&uuid(99)).unwrap()),
        Err(OwnerError::Policy(TradeRefusal::Stale))
    ));
    assert_eq!(owner.pending_transfer(actors[0].actor), Some(&binding));
    assert_eq!(owner.by_actor.len(), 2);
}

async fn seed_second_character_data(h: &native::Harness) -> TestResult {
    // Independent privileged fixture DATA, prepared before Root integrity reconciliation.
    // Native creation binding is encoded by the real producer decoder; no authority object
    // is constructed from these rows, and every production predicate remains unchanged.
    let uid = |seed: u8| native::uuid_text(native::id(seed));
    let raw = serde_json::to_vec(&serde_json::json!({
        "contract_version":2,"variant":"OPERATOR_CONTROL_PLANE_BOOTSTRAP","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":uid(133),"source_revision":"2","operation_id":uid(130),"operation":"INITIAL_CHARACTER_BOOTSTRAP","account_id":uid(70),"target_world_id":uid(native::WORLD),"requested_name":"Trade Other",
        "interpretation_context":{"profile_revision":"profile-1","ruleset_revision":"ruleset-1","content_revision":"content-1","starter_template_revision":"starter-1"},"issued_at_source":"1","expires_at_source":"120","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"
    }))?;
    let intent = crate::character_bootstrap_intent::decode_producer_response(&raw, native::id(130))
        .map_err(native::debug)?;
    sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
        .bind(native::id(70).as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'profile-1','ruleset-1','content-1','starter-1','Trade Other')").bind(native::id(60).as_slice()).bind(native::id(70).as_slice()).bind(native::id(native::WORLD).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_character_operation_receipts(operation_id,command_binding,account_id,character_id,world_id,character_revision,event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,intent_source_revision,issued_at_source,expires_at_source) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',encode($7,'hex')::uuid,encode($8,'hex')::uuid,2,1,120)")
        .bind(native::id(130).as_slice()).bind(intent.binding()).bind(native::id(70).as_slice()).bind(native::id(60).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(native::id(131).as_slice()).bind(native::id(132).as_slice()).bind(native::id(133).as_slice()).execute(&h.pool).await?;
    sqlx::query("UPDATE game_character_bootstrap_intent_floors SET source_revision=2,issuer_decision_id=encode($1,'hex')::uuid,intent_binding=$2").bind(native::id(133).as_slice()).bind(intent.binding()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_reconnect_sessions(game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,999999,1,1,1,1,$6,1)")
        .bind(native::id(61).as_slice()).bind(native::id(70).as_slice()).bind(native::id(60).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).bind([8_u8;16].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_account_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,'test',1,'trade-second-account',1,0,'{}')")
        .bind(native::id(70).as_slice()).bind(native::id(60).as_slice()).bind(native::id(61).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_durability_admission_character_guards VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,true,1,encode($4,'hex')::uuid,1,'test',1,'trade-second-character',1,0,'{}')")
        .bind(native::id(60).as_slice()).bind(native::id(70).as_slice()).bind(native::id(native::WORLD).as_slice()).bind(native::id(61).as_slice()).execute(&h.pool).await?;
    Ok(())
}
fn native_command(side: usize, number: u64) -> CommandRef {
    CommandRef::new(
        GameSessionId::decode(&native::id([native::SESSION, 61][side])).unwrap(),
        CommandId::new(number).unwrap(),
    )
}
fn native_fences() -> [CurrentCharacterItemFence; 2] {
    let mut second = native::fence().unwrap();
    second.character_id = CharacterId::from_bytes(native::id(60)).unwrap();
    second.game_session_id = GameSessionId::decode(&native::id(61)).unwrap();
    [native::fence().unwrap(), second]
}
fn source_item_facts(
    generation: &ActiveGeneration,
    source: u32,
) -> TestResult<crate::durability::item_transfer::ItemDefinitionFacts> {
    let record = generation
        .native_gameplay()
        .unwrap()
        .item_policy_for_source_id(source)
        .unwrap()
        .record();
    let ReferenceItemField::Known(stack) = &record.semantics.stack else {
        return Err("unknown stack fixture".into());
    };
    let capacity = match &record.semantics.container {
        ReferenceItemField::Known(container) => match container.capacity {
            ReferenceItemField::Known(n) => Some(u32::from(n)),
            _ => None,
        },
        _ => None,
    };
    let equip = match &record.semantics.equipment {
        ReferenceItemField::Known(equipment) => match &equipment.patterns {
            ReferenceItemField::Known(patterns) => patterns.iter().any(|p| {
                p.primary_slot
                    == ReferenceItemField::Known(crate::content::ReferenceEquipmentSlot::Container)
                    && p.additional_reserved_slots == ReferenceItemField::Known(vec![])
            }),
            _ => false,
        },
        _ => false,
    };
    Ok(crate::durability::item_transfer::ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: record.production_definition.family.clone(),
            production_key: record.production_definition.production_key.clone(),
            revision_ref: record.production_definition.revision_ref.clone(),
        },
        stack: match stack.stackable {
            ReferenceItemField::Known(true) => {
                crate::durability::item_transfer::ItemStackClass::Stackable {
                    proven_maximum: match stack.stack_max {
                        ReferenceItemField::Known(n) => Some(u32::from(n)),
                        ReferenceItemField::Unknown => None,
                        _ => return Err("missing stack maximum fixture".into()),
                    },
                }
            }
            ReferenceItemField::Known(false) => {
                crate::durability::item_transfer::ItemStackClass::NonStackable
            }
            _ => return Err("unknown stack class fixture".into()),
        },
        container_capacity: capacity,
        container_slot_equip_pattern: equip,
    })
}
async fn native_inventory(
    h: &native::Harness,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    generation: &ActiveGeneration,
) -> TestResult<[[u8; 16]; 2]> {
    let bag = source_item_facts(generation, 2854)?;
    let leaf = source_item_facts(generation, 3031)?;
    let mut leaves = Vec::new();
    for side in [0, 1] {
        let backpack = h
            .mint_definition(recovery, bag.definition.clone(), 1)
            .await?;
        let request = native::to_slot(native_command(side, 1), backpack, bag.clone());
        assert!(matches!(
            h.transfer(recovery, native_fences()[side], request)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        let item = h
            .mint_definition(recovery, leaf.definition.clone(), 1)
            .await?;
        let mut request = native::to_backpack(native_command(side, 2), item, leaf.clone());
        request.backpack = Some(bag.clone());
        assert!(matches!(
            h.transfer(recovery, native_fences()[side], request)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        leaves.push(item);
    }
    Ok(leaves.try_into().unwrap())
}
async fn activate_native_content(
    h: &native::Harness,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,NULL,$4,$5,$6)")
        .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).bind(pin.activation_sequence().to_string()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    Ok(())
}
fn pg_trade<F>(label: &str, body: F) -> TestResult
where
    F: AsyncFnOnce(&native::Harness) -> TestResult,
{
    let Some(admin) = native::configured_admin() else {
        return Ok(());
    };
    native::runtime()?.block_on(async move {
        let h = native::Harness::create(admin, label).await?;
        let result = async {
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&h.pool)
                .await?;
            if version != "170006" {
                return Err(format!("PostgreSQL17.6 required, observed {version}").into());
            }
            eprintln!("TRADE-NATIVE-PG server_version_num={version} case={label}");
            body(&h).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}
fn pg_runtime() -> (
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
    ChannelRuntimeV1,
    LocalObjectRuntime,
    [TradeActor; 2],
) {
    actual_runtime(
        WorldId::decode(&native::id(native::WORLD)).unwrap(),
        ChannelId::decode(&native::id(native::CHANNEL)).unwrap(),
        crate::foundation::NodeId::decode(&native::id(1)).unwrap(),
        [0_usize, 1].map(|side| {
            (
                native_fences()[side].game_session_id,
                crate::foundation::CharacterId::decode(
                    native_fences()[side].character_id.as_bytes(),
                )
                .unwrap(),
            )
        }),
    )
}

#[test]
fn native_root_inventory_and_owner_offer_counter_accept_retain_only_intent_data() -> TestResult {
    pg_trade("tradeowner", async |h| {
        seed_second_character_data(h).await?;
        let (controller, pin, runtime, objects, actors) = pg_runtime();
        let generation = controller.active().unwrap();
        activate_native_content(h, &pin).await?;
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let items = native_inventory(h, &recovery, generation).await?;
        let before = h.footprint().await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let now = Instant::now();
        let mut owner = TradeOwner::new(&runtime, now).map_err(native::debug)?;
        assert_eq!(
            owner
                .offer(&context, actors, items[0], native_command(0, 3), now)
                .await
                .map_err(native::debug)?,
            Phase::Offered
        );
        assert!(
            owner
                .item_reserved_by_trade(&runtime, items[0])
                .map_err(native::debug)?
        );
        assert!(
            !owner
                .item_reserved_by_trade(&runtime, items[1])
                .map_err(native::debug)?
        );
        assert!(
            !owner
                .item_reserved_by_trade(&runtime, uuid(240))
                .map_err(native::debug)?
        );
        let backpack = h
            .root
            .read_character_backpack(&recovery, actors[0].fence.character_id)
            .await
            .map_err(native::debug)?
            .unwrap();
        assert!(
            !owner
                .item_reserved_by_trade(&runtime, backpack.backpack.item_instance_id)
                .map_err(native::debug)?
        );
        // Cancellation releases membership. Restart through the actual native read path.
        owner.invalidate(actors[0].actor).map_err(native::debug)?;
        assert!(
            !owner
                .item_reserved_by_trade(&runtime, items[0])
                .map_err(native::debug)?
        );
        owner
            .offer(&context, actors, items[0], native_command(0, 3), now)
            .await
            .map_err(native::debug)?;
        // No Character may start another trade in this same book, including with another
        // named actor; Busy is a refusal, never a grant for the other unvalidated name.
        assert!(matches!(
            owner
                .offer(&context, actors, items[0], native_command(0, 4), now)
                .await,
            Err(OwnerError::Busy)
        ));
        assert_eq!(
            owner
                .offer(
                    &context,
                    [actors[1], actors[0]],
                    items[1],
                    native_command(1, 3),
                    now
                )
                .await
                .map_err(native::debug)?,
            Phase::Ready
        );
        for item in items {
            assert!(
                owner
                    .item_reserved_by_trade(&runtime, item)
                    .map_err(native::debug)?
            );
        }
        let pair = native_command(1, 3);
        assert_eq!(
            owner
                .accept(&context, actors[0], pair, native_command(0, 4), now)
                .await
                .map_err(native::debug)?,
            Phase::Ready
        );
        assert_eq!(
            owner
                .accept(&context, actors[0], pair, native_command(0, 5), now)
                .await
                .map_err(native::debug)?,
            Phase::Ready
        );
        assert_eq!(
            owner
                .accept(&context, actors[1], pair, native_command(1, 4), now)
                .await
                .map_err(native::debug)?,
            Phase::Transferring
        );
        let intent = owner.reservation_intent(actors[0].actor).unwrap();
        assert_eq!(intent.binding.accepts[0], native_command(0, 4));
        assert_eq!(intent.binding.trigger, native_command(1, 4));
        assert_eq!(intent.destinations.map(|p| p.placement_ordinal), [2, 2]);
        let binding = intent.binding.clone();
        owner
            .note_ambiguous_outcome(actors[0].actor, &binding)
            .map_err(native::debug)?;
        assert!(matches!(
            owner.invalidate_item(items[0]),
            Err(OwnerError::Policy(TradeRefusal::Transferring))
        ));
        assert_eq!(h.footprint().await?, before);
        assert_eq!(owner.phase(actors[0].actor), Some(Phase::Transferring));
        for item in items {
            assert!(
                owner
                    .item_reserved_by_trade(&runtime, item)
                    .map_err(native::debug)?
            );
        }
        assert_eq!(
            h.root
                .read_character_backpack(&recovery, actors[0].fence.character_id)
                .await
                .map_err(native::debug)?
                .unwrap()
                .entries[0]
                .item
                .item_instance_id,
            items[0]
        );
        Ok(())
    })
}

#[test]
fn native_root_rejects_each_changed_current_fact_with_immutable_expected_inputs() -> TestResult {
    pg_trade("tradecurrent", async |h| {
        seed_second_character_data(h).await?;
        let (controller, pin, runtime, objects, actors) = pg_runtime();
        let generation = controller.active().unwrap();
        activate_native_content(h, &pin).await?;
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let items = native_inventory(h, &recovery, generation).await?;
        let fences = native_fences();
        let digest = pin.identity().server_artifact_digest();
        let before = h
            .root
            .read_player_trade_data(
                &recovery,
                &h.node,
                &runtime,
                fences,
                items.map(Some),
                digest,
            )
            .await
            .map_err(native::debug)?;
        for (table, field, bad, good, key) in [
            (
                "game_durability_reconnect_sessions",
                "current_generation",
                "2",
                "1",
                "game_session_id",
            ),
            (
                "game_durability_reconnect_sessions",
                "character_lease_generation",
                "2",
                "1",
                "game_session_id",
            ),
            (
                "game_durability_reconnect_sessions",
                "scope_ownership_generation",
                "2",
                "1",
                "game_session_id",
            ),
            (
                "game_durability_admission_character_guards",
                "eligible",
                "FALSE",
                "TRUE",
                "character_id",
            ),
        ] {
            let id = if key == "game_session_id" {
                native::id(61)
            } else {
                native::id(60)
            };
            // Exercise every owner read boundary, not only the shared Root helper. Each
            // stage starts one independent book under the restored actual current facts.
            // These sequential books are not installed concurrent global owners.
            for stage in 0..3 {
                let context = TradeReadContext {
                    runtime: &runtime,
                    root: &h.root,
                    recovery: &recovery,
                    node: &h.node,
                    room: pin.qualified_room(),
                    objects: &objects,
                    generation,
                };
                let now = Instant::now();
                let mut owner = TradeOwner::new(&runtime, now).map_err(native::debug)?;
                if stage > 0 {
                    owner
                        .offer(&context, actors, items[0], native_command(0, 3), now)
                        .await
                        .map_err(native::debug)?;
                }
                if stage > 1 {
                    owner
                        .offer(
                            &context,
                            [actors[1], actors[0]],
                            items[1],
                            native_command(1, 3),
                            now,
                        )
                        .await
                        .map_err(native::debug)?;
                }
                // Independent privileged current-row DATA mutation, NOT publication. All
                // native SQL guards stay enabled; expected actors/fences/book stay fixed.
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {field}={bad} WHERE {key}=encode($1,'hex')::uuid"
                )))
                .bind(id.as_slice())
                .execute(&h.pool)
                .await?;
                let footprint = h.footprint().await?;
                assert!(matches!(
                    h.root
                        .read_player_trade_data(
                            &recovery,
                            &h.node,
                            &runtime,
                            fences,
                            items.map(Some),
                            digest
                        )
                        .await,
                    Err(OwnerError::Authority)
                ));
                let refused = match stage {
                    0 => {
                        owner
                            .offer(&context, actors, items[0], native_command(0, 3), now)
                            .await
                    }
                    1 => {
                        owner
                            .offer(
                                &context,
                                [actors[1], actors[0]],
                                items[1],
                                native_command(1, 3),
                                now,
                            )
                            .await
                    }
                    _ => {
                        owner
                            .accept(
                                &context,
                                actors[0],
                                native_command(1, 3),
                                native_command(0, 4),
                                now,
                            )
                            .await
                    }
                };
                assert!(
                    matches!(refused, Err(OwnerError::Authority)),
                    "stage={stage} {table}.{field}: {refused:?}"
                );
                assert!(owner.pending_transfer(actors[0].actor).is_none());
                assert_eq!(h.footprint().await?, footprint);
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE {table} SET {field}={good} WHERE {key}=encode($1,'hex')::uuid"
                )))
                .bind(id.as_slice())
                .execute(&h.pool)
                .await?;
                assert_eq!(
                    h.root
                        .read_player_trade_data(
                            &recovery,
                            &h.node,
                            &runtime,
                            fences,
                            items.map(Some),
                            digest
                        )
                        .await
                        .map_err(native::debug)?,
                    before
                );
            }
        }
        let foreign = native::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .read_player_trade_data(
                    &recovery,
                    &foreign,
                    &runtime,
                    fences,
                    items.map(Some),
                    digest
                )
                .await,
            Err(OwnerError::Authority)
        ));
        assert_eq!(
            h.root
                .read_player_trade_data(
                    &recovery,
                    &h.node,
                    &runtime,
                    fences,
                    items.map(Some),
                    digest
                )
                .await
                .map_err(native::debug)?,
            before
        );
        // Independently valid current DB facts remain unchanged. Only native carrier
        // expected provenance is varied via its real constructor, without manufacturing a
        // current assignment or weakening its immutable/monotonic database guards.
        for (node, revisions) in [
            (
                crate::foundation::NodeId::decode(&native::id(2)).unwrap(),
                (1, 1),
            ),
            (h.node.fact().node_id(), (2, 1)),
            (h.node.fact().node_id(), (1, 2)),
        ] {
            let (_controller, _pin, foreign_runtime, _objects, _actors) = actual_runtime_positions(
                WorldId::decode(&native::id(native::WORLD)).unwrap(),
                ChannelId::decode(&native::id(native::CHANNEL)).unwrap(),
                node,
                [0_usize, 1].map(|side| {
                    (
                        fences[side].game_session_id,
                        crate::foundation::CharacterId::decode(
                            fences[side].character_id.as_bytes(),
                        )
                        .unwrap(),
                    )
                }),
                [OPEN_A, OPEN_B],
                8,
                revisions,
            );
            let expected = runtime_key(&foreign_runtime);
            assert!(matches!(
                h.root
                    .read_player_trade_data(
                        &recovery,
                        &h.node,
                        &foreign_runtime,
                        fences,
                        items.map(Some),
                        digest
                    )
                    .await,
                Err(OwnerError::Authority)
            ));
            assert_eq!(runtime_key(&foreign_runtime), expected);
            assert_eq!(
                h.root
                    .read_player_trade_data(
                        &recovery,
                        &h.node,
                        &runtime,
                        fences,
                        items.map(Some),
                        digest
                    )
                    .await
                    .map_err(native::debug)?,
                before
            );
        }
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let now = Instant::now();
        let mut owner = TradeOwner::new(&runtime, now).map_err(native::debug)?;
        owner
            .offer(&context, actors, items[0], native_command(0, 3), now)
            .await
            .map_err(native::debug)?;
        owner
            .offer(
                &context,
                [actors[1], actors[0]],
                items[1],
                native_command(1, 3),
                now,
            )
            .await
            .map_err(native::debug)?;
        let expected_binding = owner.binding;
        // Genuine native Content-record function commits a separately compiled generation;
        // immutable book/runtime/expected digest do not become current merely by matching.
        let (_other_controller, other) =
            crate::content::native_gameplay::tests::activated_with_loot_tables(
                WorldId::decode(&native::id(native::WORLD)).unwrap(),
                None,
            );
        assert_ne!(other.identity().server_artifact_digest(), digest);
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2::numeric,1::numeric,$3,$4,$5)")
            .bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).bind(other.identity().server_artifact_digest().as_slice()).bind(other.identity().client_artifact_digest().as_slice()).bind(other.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        assert!(matches!(
            owner
                .accept(
                    &context,
                    actors[0],
                    native_command(1, 3),
                    native_command(0, 4),
                    now
                )
                .await,
            Err(OwnerError::Authority)
        ));
        assert_eq!(owner.binding, expected_binding);
        assert!(owner.pending_transfer(actors[0].actor).is_none());
        Ok(())
    })
}

#[test]
fn native_mint_transfer_changes_current_inventory_and_cancels_accept_without_swap() -> TestResult {
    pg_trade("tradeitems", async |h| {
        seed_second_character_data(h).await?;
        let (controller, pin, runtime, objects, actors) = pg_runtime();
        let generation = controller.active().unwrap();
        activate_native_content(h, &pin).await?;
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let items = native_inventory(h, &recovery, generation).await?;
        let fences = native_fences();
        let digest = pin.identity().server_artifact_digest();
        // An existing real foreign item or the real main backpack is not a direct offer.
        assert!(matches!(
            h.root
                .read_player_trade_data(
                    &recovery,
                    &h.node,
                    &runtime,
                    fences,
                    [Some(items[1]), None],
                    digest
                )
                .await,
            Err(OwnerError::NotTradeable)
        ));
        let backpack = h
            .root
            .read_character_backpack(&recovery, fences[0].character_id)
            .await
            .map_err(native::debug)?
            .unwrap();
        assert!(matches!(
            h.root
                .read_player_trade_data(
                    &recovery,
                    &h.node,
                    &runtime,
                    fences,
                    [Some(backpack.backpack.item_instance_id), None],
                    digest
                )
                .await,
            Err(OwnerError::NotTradeable)
        ));
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let now = Instant::now();
        let mut owner = TradeOwner::new(&runtime, now).map_err(native::debug)?;
        owner
            .offer(&context, actors, items[0], native_command(0, 3), now)
            .await
            .map_err(native::debug)?;
        owner
            .offer(
                &context,
                [actors[1], actors[0]],
                items[1],
                native_command(1, 3),
                now,
            )
            .await
            .map_err(native::debug)?;
        let captured = h
            .root
            .read_player_trade_data(
                &recovery,
                &h.node,
                &runtime,
                fences,
                items.map(Some),
                digest,
            )
            .await
            .map_err(native::debug)?;
        let leaf = source_item_facts(generation, 3031)?;
        let bag = source_item_facts(generation, 2854)?;
        let extra = h
            .mint_definition(&recovery, leaf.definition.clone(), 1)
            .await?;
        let mut request = native::to_backpack(native_command(0, 9), extra, leaf);
        request.backpack = Some(bag);
        assert!(matches!(
            h.transfer(&recovery, fences[0], request)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        let current = h
            .root
            .read_player_trade_data(
                &recovery,
                &h.node,
                &runtime,
                fences,
                items.map(Some),
                digest,
            )
            .await
            .map_err(native::debug)?;
        assert_ne!(current, captured);
        assert_eq!(current.offers[0].as_ref().unwrap().state.quantity, 2);
        let post_transfer = h.footprint().await?;
        assert!(matches!(
            owner
                .accept(
                    &context,
                    actors[0],
                    native_command(1, 3),
                    native_command(0, 4),
                    now
                )
                .await,
            Err(OwnerError::Policy(TradeRefusal::Stale))
        ));
        assert_eq!(h.footprint().await?, post_transfer);
        assert!(owner.pending_transfer(actors[0].actor).is_none());
        // Start a new real current binding after the known item change, then revoke the
        // assignment through its genuine native writer. Never regenerate expected facts
        // from the revoked assignment; the retained book still has the same immutable key.
        owner
            .offer(&context, actors, items[0], native_command(0, 10), now)
            .await
            .map_err(native::debug)?;
        owner
            .offer(
                &context,
                [actors[1], actors[0]],
                items[1],
                native_command(1, 10),
                now,
            )
            .await
            .map_err(native::debug)?;
        let expected_binding = owner.binding;
        use crate::durability::runtime_scope_assignment::{
            AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
            RuntimeScopeAssignmentWriter,
        };
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)").bind(native::id(native::WORLD).as_slice()).bind(native::id(native::CHANNEL).as_slice()).execute(&h.pool).await?;
        let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "item-transfer-writer")
            .await
            .map_err(native::debug)?;
        let scope = fences[0].runtime_scope;
        let predecessor = h
            .root
            .read_runtime_scope_predecessor(scope)
            .await
            .map_err(native::debug)?
            .ok_or("current assignment")?;
        let outcome = writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([0x54; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(native::debug)?,
                command: AssignmentCommand::Revoke { scope, predecessor },
            })
            .await
            .map_err(native::debug)?;
        assert!(matches!(outcome, AssignmentOutcome::Committed(_)));
        assert!(matches!(
            owner
                .accept(
                    &context,
                    actors[0],
                    native_command(1, 10),
                    native_command(0, 11),
                    now
                )
                .await,
            Err(OwnerError::Authority)
        ));
        assert_eq!(owner.binding, expected_binding);
        assert!(owner.pending_transfer(actors[0].actor).is_none());
        Ok(())
    })
}

#[test]
fn native_range_two_floor_and_actual_blocking_ray_boundaries() {
    let (_generation, pin, runtime, objects, expected) = geometry_fixture();
    assert!(qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, expected).is_ok());
    let identities = [0_u8, 1].map(|side| {
        (
            GameSessionId::decode(&uuid(31 + side)).unwrap(),
            crate::foundation::CharacterId::decode(&uuid(1 + side)).unwrap(),
        )
    });
    let pos = |x, y, floor| crate::foundation::MovementLocalPosition { x, y, floor };
    for (positions, reason) in [
        ([OPEN_A, pos(0, 12, -7)], 0), // exact2, native ground-only cells on the actual source map
        ([OPEN_A, pos(-1, 12, -7)], 1), //3
        ([OPEN_A, pos(1, 12, -6)], 1), // sameXY reach, differentfloor
        ([pos(-13, 13, -7), pos(-11, 13, -7)], 2), // actual Source wall at(-12,13,-7)
    ] {
        let (_controller, pin, changed, objects, names) = actual_runtime_positions(
            WorldId::decode(&uuid(50)).unwrap(),
            ChannelId::decode(&uuid(51)).unwrap(),
            crate::foundation::NodeId::decode(&uuid(52)).unwrap(),
            identities,
            positions,
            8,
            (1, 1),
        );
        assert_eq!(names, expected); // expected identity/fences NOT regenerated from changed pose
        let result = qualify_runtime_pair(&changed, pin.qualified_room(), &objects, expected);
        match reason {
            0 => assert!(result.is_ok()),
            1 => assert!(matches!(result, Err(OwnerError::TooFar))),
            _ => assert!(matches!(result, Err(OwnerError::UnknownMap))),
        }
    }
}

#[test]
fn current_native_visibility_cutoff_refuses_same_nearby_partner_without_a_flag() {
    let identities = [0_u8, 1].map(|side| {
        (
            GameSessionId::decode(&uuid(31 + side)).unwrap(),
            crate::foundation::CharacterId::decode(&uuid(1 + side)).unwrap(),
        )
    });
    let (_generation, pin, mut runtime, objects, expected) = actual_runtime_positions(
        WorldId::decode(&uuid(50)).unwrap(),
        ChannelId::decode(&uuid(51)).unwrap(),
        crate::foundation::NodeId::decode(&uuid(52)).unwrap(),
        identities,
        [OPEN_A, OPEN_B],
        512,
        (1, 1),
    );
    assert!(qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, expected).is_ok());
    for n in 0_u32..256 {
        let mut bytes = uuid(90);
        bytes[12..16].copy_from_slice(&(n + 1000).to_be_bytes());
        let session = GameSessionId::decode(&bytes).unwrap();
        let reservation = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reservation).unwrap();
        runtime
            .initialize_pinned_test_position(actor, OPEN_A)
            .unwrap();
    }
    assert!(matches!(
        qualify_runtime_pair(&runtime, pin.qualified_room(), &objects, expected),
        Err(OwnerError::NotVisible)
    ));
    // Committed carrier placements exercise the real256result cutoff; this is no claim of
    //256 authentic client admissions or of production memory/latency qualification.
}
