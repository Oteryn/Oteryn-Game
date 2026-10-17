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
    BackpackEntry, CharacterBackpack, ContainerEntryPosition, CurrentCharacterItemFence,
    InventoryItem,
};
use crate::durability::player_trade::{BackpackItemVersion, LeafOffer, Trader};
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
    let binding = runtime.binding();
    let original_node = crate::foundation::NodeId::decode(&uuid(52)).unwrap();
    // These are genuine native carriers with one assignment component changed.
    // Node/incarnation cases retain the same scope, source revision and Content;
    // an empty local book must not turn either mismatch into an unreserved answer.
    for (changed, channel, node, registration_revision) in [
        (
            "channel",
            ChannelId::decode(&uuid(53)).unwrap(),
            original_node,
            1,
        ),
        (
            "node",
            binding.channel_id(),
            crate::foundation::NodeId::decode(&uuid(53)).unwrap(),
            1,
        ),
        (
            "node registration revision",
            binding.channel_id(),
            original_node,
            2,
        ),
    ] {
        let foreign = ChannelRuntimeV1::from_committed_assignment(
            binding.world_id(),
            channel,
            node,
            registration_revision,
            binding.scope_generation().get(),
            binding.source_revision(),
            "runtime-scope-assignment:1",
            runtime.actor_slot_capacity(),
            runtime.content_pin().clone(),
        )
        .unwrap();
        assert!(
            matches!(
                owner.item_reserved_by_trade(&foreign, uuid(240)),
                Err(OwnerError::Authority)
            ),
            "changed {changed} must refuse before the empty-book answer"
        );
    }
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
            issuance: None,
            retained_swap: None,
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
            issuance: None,
            retained_swap: None,
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
    native_inventory_with_ingress(h, recovery, generation, None).await
}

/// The issuer case uses actual FND reservations around the real fixture item writers,
/// so its accept commands follow the committed pickup commands in the same ingress.
async fn native_inventory_with_ingress(
    h: &native::Harness,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    generation: &ActiveGeneration,
    mut ingress: Option<&mut [CommandIngress; 2]>,
) -> TestResult<[[u8; 16]; 2]> {
    let bag = source_item_facts(generation, 2854)?;
    let leaf = source_item_facts(generation, 3031)?;
    let mut leaves = Vec::new();
    for side in [0, 1] {
        let backpack = h
            .mint_definition(recovery, bag.definition.clone(), 1)
            .await?;
        let request = native::to_slot(native_command(side, 1), backpack, bag.clone());
        if let Some(ingress) = ingress.as_mut() {
            reserve_fixture_command(
                &mut ingress[side],
                native_command(side, 1),
                "pickup",
                backpack,
            )?;
        }
        assert!(matches!(
            h.transfer(recovery, native_fences()[side], request)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        if let Some(ingress) = ingress.as_mut() {
            terminal_fixture_command(
                &mut ingress[side],
                native_command(side, 1),
                "COMMITTED",
                backpack,
            )?;
        }
        let item = h
            .mint_definition(recovery, leaf.definition.clone(), 1)
            .await?;
        let mut request = native::to_backpack(native_command(side, 2), item, leaf.clone());
        request.backpack = Some(bag.clone());
        if let Some(ingress) = ingress.as_mut() {
            reserve_fixture_command(&mut ingress[side], native_command(side, 2), "pickup", item)?;
        }
        assert!(matches!(
            h.transfer(recovery, native_fences()[side], request)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        if let Some(ingress) = ingress.as_mut() {
            terminal_fixture_command(
                &mut ingress[side],
                native_command(side, 2),
                "COMMITTED",
                item,
            )?;
        }
        leaves.push(item);
    }
    Ok(leaves.try_into().unwrap())
}

fn reserve_fixture_command(
    ingress: &mut CommandIngress,
    command: CommandRef,
    family: &str,
    item: [u8; 16],
) -> TestResult {
    let semantic = CommandSemanticIdentity::new(
        NormalizedSemanticIntentIdentity::new(&trade_hex(&item), 1, family, 1)?,
        RetainedBindingIdentity::new(
            "native-trade-fixture",
            &trade_hex(command.game_session_id().as_bytes()),
        )?,
    );
    assert_eq!(
        ingress.reserve(command.command_id(), semantic),
        crate::foundation::IngressDecision::Reserved(command.command_id()),
    );
    Ok(())
}

/// Only called after the actual native operation has returned the named outcome.
fn terminal_fixture_command(
    ingress: &mut CommandIngress,
    command: CommandRef,
    disposition: &str,
    item: [u8; 16],
) -> TestResult {
    ingress.terminalize(
        command.command_id(),
        TerminalSemanticOutcome::new(disposition, &trade_hex(&item), 1)?,
        || {},
    )?;
    Ok(())
}

async fn native_ready_with_ingress(
    context: &TradeReadContext<'_, '_, '_>,
    actors: [TradeActor; 2],
    items: [[u8; 16]; 2],
    ingress: &mut [CommandIngress; 2],
    now: Instant,
) -> TestResult<TradeOwner> {
    let mut owner = TradeOwner::new(context.runtime, now).map_err(native::debug)?;
    for side in [0, 1] {
        let command = CommandRef::new(
            actors[side].fence.game_session_id,
            ingress[side].next_command_id().unwrap(),
        );
        reserve_fixture_command(
            &mut ingress[side],
            command,
            "player_trade_offer",
            items[side],
        )?;
        assert_eq!(
            owner
                .offer(
                    context,
                    [actors[side], actors[1 - side]],
                    items[side],
                    command,
                    now
                )
                .await
                .map_err(native::debug)?,
            if side == 0 {
                Phase::Offered
            } else {
                Phase::Ready
            },
        );
        terminal_fixture_command(&mut ingress[side], command, "OFFERED", items[side])?;
    }
    Ok(owner)
}

fn reserve_native_accept(
    owner: &TradeOwner,
    actor: TradeActor,
    trade_id: [u8; 16],
    ingress: &mut CommandIngress,
) -> TestResult<CommandRef> {
    let command = CommandRef::new(
        actor.fence.game_session_id,
        ingress.next_command_id().unwrap(),
    );
    let semantic = owner
        .accept_semantic(command, trade_id)
        .map_err(native::debug)?;
    assert_eq!(
        ingress.reserve(command.command_id(), semantic),
        crate::foundation::IngressDecision::Reserved(command.command_id())
    );
    Ok(command)
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
        result?;
        eprintln!("native Trade PG PASS: {label}");
        Ok(())
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
                    Err(TradeReadError::Authority)
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
            Err(TradeReadError::Authority)
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
                Err(TradeReadError::Authority)
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
            Err(TradeReadError::NotTradeable)
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
            Err(TradeReadError::NotTradeable)
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

#[test]
fn native_trade_id_and_occurrence_preserve_full_command_identity() {
    let counter = command(Side::Second, 3);
    let first = command(Side::First, 4);
    let second = command(Side::Second, 4);
    let id = issued_trade_id(counter);
    let occurrence = command_identity(b"oteryn:player-trade-occurrence:v1", second);
    assert_eq!(id.len(), 16);
    assert_eq!(occurrence.len(), 16);
    assert_eq!(id[6] >> 4, 7);
    assert_eq!(occurrence[8] >> 6, 2);
    assert_eq!(issued_trade_id(counter), id);
    assert_ne!(occurrence, id);
    assert_ne!(
        command_identity(b"oteryn:player-trade-occurrence:v1", first),
        occurrence
    );
    assert_ne!(
        command_identity(
            b"oteryn:player-trade-occurrence:v1",
            command(Side::Second, 5)
        ),
        occurrence
    );
    assert_ne!(
        issued_trade_id(command(Side::Second, u64::MAX)),
        issued_trade_id(command(Side::Second, u32::MAX.into()))
    );
}

#[test]
fn native_pending_trade_accept_issues_once_after_terminal_partner() -> TestResult {
    pg_trade("tradeissuer", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
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
        let mut owner =
            native_ready_with_ingress(&context, actors, items, &mut ingress, now).await?;
        let id = owner.trade_id(actors[0].actor).unwrap();
        assert_eq!(owner.trade_id(actors[1].actor), Some(id));
        let before = h.footprint().await?;

        // Change only pending ingress: all native actors, fences, item state and id stay valid.
        assert!(matches!(
            owner
                .accept_pending(
                    &context,
                    actors[0],
                    id,
                    native_command(0, 4),
                    &mut ingress[0],
                    now
                )
                .await,
            Err(OwnerError::Ingress(TradeIngressRefusal::MissingPending)),
        ));
        let first = reserve_native_accept(&owner, actors[0], id, &mut ingress[0])?;
        let later_first = reserve_native_accept(&owner, actors[0], id, &mut ingress[0])?;
        // Real FND terminal preflight refuses the later command; no acceptance was installed.
        assert!(matches!(
            owner
                .accept_pending(&context, actors[0], id, later_first, &mut ingress[0], now)
                .await,
            Err(OwnerError::CommandLifecycle(
                CommandLifecycleError::TerminalOutOfOrder { .. }
            )),
        ));
        assert_eq!(
            owner.trades.values().next().unwrap().session.accepts,
            [None, None]
        );
        assert!(owner.trades.values().next().unwrap().issuance.is_none());
        assert_eq!(ingress[0].outstanding(), 2);

        let mut changed_id = id;
        changed_id[15] ^= 1;
        assert!(matches!(
            owner
                .accept_pending(&context, actors[0], changed_id, first, &mut ingress[0], now)
                .await,
            Err(OwnerError::Ingress(
                TradeIngressRefusal::ConflictChangedInput
            )),
        ));
        let wrong_session = CommandRef::new(actors[1].fence.game_session_id, first.command_id());
        assert!(matches!(
            owner
                .accept_pending(&context, actors[0], id, wrong_session, &mut ingress[0], now)
                .await,
            Err(OwnerError::Authority),
        ));
        assert!(
            owner
                .issued_transfer(&runtime, actors[0].actor)
                .map_err(native::debug)?
                .is_none()
        );

        assert_eq!(
            owner
                .accept_pending(&context, actors[0], id, first, &mut ingress[0], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::Accepted
        );
        assert_eq!(owner.phase(actors[0].actor), Some(Phase::Ready));
        let first_semantic = owner.accept_semantic(first, id).map_err(native::debug)?;
        assert!(matches!(
            ingress[0].classify_duplicate(first.command_id(), &first_semantic),
            DuplicateDisposition::ReplayRetainedOutcome(_)
        ));
        assert_eq!(
            owner
                .accept_pending(&context, actors[0], id, first, &mut ingress[0], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::ReplayedAccepted
        );
        assert_eq!(
            owner
                .accept_pending(&context, actors[0], id, later_first, &mut ingress[0], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::Accepted
        );
        // The original terminal accept survives native retained-record eviction in the book.
        assert!(matches!(
            ingress[0].classify_duplicate(first.command_id(), &first_semantic),
            DuplicateDisposition::OutcomeExpired
        ));

        let second = reserve_native_accept(&owner, actors[1], id, &mut ingress[1])?;
        let later_second = reserve_native_accept(&owner, actors[1], id, &mut ingress[1])?;
        assert!(matches!(
            owner
                .accept_pending(&context, actors[1], id, later_second, &mut ingress[1], now)
                .await,
            Err(OwnerError::Ingress(TradeIngressRefusal::PendingOutOfOrder)),
        ));
        assert_eq!(owner.phase(actors[0].actor), Some(Phase::Ready));
        assert!(
            owner
                .issued_transfer(&runtime, actors[0].actor)
                .map_err(native::debug)?
                .is_none()
        );

        assert_eq!(
            owner
                .accept_pending(&context, actors[1], id, second, &mut ingress[1], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::Transferring
        );
        let (issued, intent) = owner
            .issued_transfer(&runtime, actors[0].actor)
            .map_err(native::debug)?
            .unwrap();
        assert_eq!(*issued.trade_id(), id);
        assert_eq!(issued.first_accept(), first);
        assert_eq!(issued.first_fence(), actors[0].fence);
        assert_eq!(issued.trigger(), second);
        assert_eq!(intent.binding.accepts, [first, second]);
        assert_eq!(
            intent
                .destinations
                .map(|destination| destination.placement_ordinal),
            [2, 2]
        );
        let occurrence = *issued.occurrence();
        let issued_address: *const IssuedTradeOccurrence = issued;
        let binding = intent.binding.clone();
        let second_semantic = owner.accept_semantic(second, id).map_err(native::debug)?;
        assert!(matches!(
            ingress[1].classify_duplicate(second.command_id(), &second_semantic),
            DuplicateDisposition::PendingOriginal
        ));
        owner
            .note_ambiguous_outcome(actors[0].actor, &binding)
            .map_err(native::debug)?;
        assert_eq!(
            owner
                .accept_pending(&context, actors[1], id, second, &mut ingress[1], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::Transferring
        );
        owner
            .expire(now + Duration::from_secs(121))
            .map_err(native::debug)?;
        let (same, _) = owner
            .issued_transfer(&runtime, actors[1].actor)
            .map_err(native::debug)?
            .unwrap();
        assert!(
            std::ptr::eq(same, issued_address),
            "retry must retain the issued record"
        );
        assert_eq!(*same.occurrence(), occurrence);
        assert!(matches!(
            owner.invalidate_item(items[0]),
            Err(OwnerError::Policy(TradeRefusal::Transferring))
        ));
        assert!(matches!(
            ingress[1].classify_duplicate(second.command_id(), &second_semantic),
            DuplicateDisposition::PendingOriginal
        ));
        assert_eq!(
            h.footprint().await?,
            before,
            "issuance is not a durable swap or reservation"
        );
        Ok(())
    })
}

#[test]
fn native_issuer_refuses_data_accept_and_terminal_enum_as_consent() -> TestResult {
    pg_trade("tradeconsent", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
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
        let mut owner =
            native_ready_with_ingress(&context, actors, items, &mut ingress, now).await?;
        let id = owner.trade_id(actors[0].actor).unwrap();
        // A successful old DATA accept still did not pass through native terminalization.
        assert_eq!(
            owner
                .accept(
                    &context,
                    actors[0],
                    native_command(1, 3),
                    native_command(0, 4),
                    now
                )
                .await
                .map_err(native::debug)?,
            Phase::Ready
        );
        let second = reserve_native_accept(&owner, actors[1], id, &mut ingress[1])?;
        assert!(matches!(
            owner
                .accept_pending(&context, actors[1], id, second, &mut ingress[1], now)
                .await,
            Err(OwnerError::Ingress(TradeIngressRefusal::UnissuedAcceptance))
        ));
        assert!(
            owner
                .issued_transfer(&runtime, actors[1].actor)
                .map_err(native::debug)?
                .is_none()
        );
        terminal_fixture_command(&mut ingress[1], second, "REJECTED", id)?;
        owner.invalidate(actors[0].actor).map_err(native::debug)?;
        drop(owner);

        let mut owner =
            native_ready_with_ingress(&context, actors, items, &mut ingress, now).await?;
        let id = owner.trade_id(actors[0].actor).unwrap();
        let first = reserve_native_accept(&owner, actors[0], id, &mut ingress[0])?;
        // Deliberately terminalize outside the TradeOwner mutation. Even a real FND retained
        // ACCEPTED outcome must not be promoted into the missing owner-issued consent marker.
        terminal_fixture_command(&mut ingress[0], first, "ACCEPTED", id)?;
        assert_eq!(
            owner
                .accept_pending(&context, actors[0], id, first, &mut ingress[0], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::ReplayedAccepted
        );
        assert!(owner.trades.values().next().unwrap().issuance.is_none());
        let second = reserve_native_accept(&owner, actors[1], id, &mut ingress[1])?;
        assert_eq!(
            owner
                .accept_pending(&context, actors[1], id, second, &mut ingress[1], now)
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::Accepted
        );
        assert_eq!(owner.phase(actors[1].actor), Some(Phase::Ready));
        assert!(
            owner
                .issued_transfer(&runtime, actors[0].actor)
                .map_err(native::debug)?
                .is_none()
        );
        assert!(matches!(
            owner.trades.values().next().unwrap().issuance,
            Some(TradeIssuance::FirstAccepted(TerminalTradeAccept {
                side: Side::Second,
                ..
            }))
        ));
        // The legacy adapter cannot change a lifecycle after a genuine native first accept.
        let pair = owner.trades.values().next().unwrap().session.pair.unwrap();
        assert!(matches!(
            owner
                .accept(&context, actors[0], pair, native_command(0, 6), now)
                .await,
            Err(OwnerError::Ingress(TradeIngressRefusal::UnissuedAcceptance))
        ));
        // Historical replay can outlive the closed book. Its separate outcome does not
        // revalidate current actor/DB state, recreate consent or imply a current trade.
        owner.invalidate(actors[0].actor).map_err(native::debug)?;
        assert_eq!(owner.phase(actors[0].actor), None);
        assert_eq!(
            owner
                .accept_pending(
                    &context,
                    actors[1],
                    id,
                    second,
                    &mut ingress[1],
                    now + Duration::from_secs(121)
                )
                .await
                .map_err(native::debug)?,
            NativeAcceptOutcome::ReplayedAccepted
        );
        assert!(owner.trades.is_empty());
        assert!(
            owner
                .issued_transfer(&runtime, actors[1].actor)
                .map_err(native::debug)?
                .is_none()
        );
        Ok(())
    })
}

#[test]
fn native_issuer_rechecks_item_changed_by_actual_transfer() -> TestResult {
    pg_trade("tradeissuerstale", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
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
        let mut owner =
            native_ready_with_ingress(&context, actors, items, &mut ingress, now).await?;
        let id = owner.trade_id(actors[0].actor).unwrap();
        let first = reserve_native_accept(&owner, actors[0], id, &mut ingress[0])?;
        owner
            .accept_pending(&context, actors[0], id, first, &mut ingress[0], now)
            .await
            .map_err(native::debug)?;
        let leaf = source_item_facts(generation, 3031)?;
        let extra = h
            .mint_definition(&recovery, leaf.definition.clone(), 1)
            .await?;
        let pickup = CommandRef::new(
            actors[0].fence.game_session_id,
            ingress[0].next_command_id().unwrap(),
        );
        reserve_fixture_command(&mut ingress[0], pickup, "pickup", extra)?;
        let mut request = native::to_backpack(pickup, extra, leaf);
        request.backpack = Some(source_item_facts(generation, 2854)?);
        assert!(matches!(
            h.transfer(&recovery, actors[0].fence, request)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        terminal_fixture_command(&mut ingress[0], pickup, "COMMITTED", extra)?;
        let before = h.footprint().await?;
        let second = reserve_native_accept(&owner, actors[1], id, &mut ingress[1])?;
        assert!(matches!(
            owner
                .accept_pending(&context, actors[1], id, second, &mut ingress[1], now)
                .await,
            Err(OwnerError::Policy(TradeRefusal::Stale))
        ));
        assert!(
            owner
                .issued_transfer(&runtime, actors[0].actor)
                .map_err(native::debug)?
                .is_none()
        );
        assert_eq!(
            owner.phase(actors[0].actor),
            Some(Phase::Closed(CloseReason::Stale))
        );
        assert_eq!(h.footprint().await?, before);
        Ok(())
    })
}

async fn root_lock_is_available(pool: &sqlx::PgPool, character: CharacterId) -> TestResult<bool> {
    let mut observer = pool.begin().await?;
    let result = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM game_character_roots \
         WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut *observer)
    .await;
    observer.rollback().await?;
    match result {
        Ok(1) => Ok(true),
        Err(error)
            if error
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("55P03")) =>
        {
            Ok(false)
        }
        Err(error) => Err(error.into()),
        Ok(_) => Err("unexpected root-lock observation".into()),
    }
}

// These writer fixtures use the real native component issuance, Root, recovery,
// node, fences, Content, MINT and TRANSFER. The session-owned installation of the
// two FND ingresses is still external; constructing these fixture ingresses does
// not qualify that missing transport boundary.
async fn native_transferring_with_ingress(
    context: &TradeReadContext<'_, '_, '_>,
    actors: [TradeActor; 2],
    items: [[u8; 16]; 2],
    ingress: &mut [CommandIngress; 2],
    now: Instant,
) -> TestResult<TradeOwner> {
    let mut owner = native_ready_with_ingress(context, actors, items, ingress, now).await?;
    let id = owner.trade_id(actors[0].actor).unwrap();
    let first = reserve_native_accept(&owner, actors[0], id, &mut ingress[0])?;
    assert_eq!(
        owner
            .accept_pending(context, actors[0], id, first, &mut ingress[0], now)
            .await
            .map_err(native::debug)?,
        NativeAcceptOutcome::Accepted,
    );
    let second = reserve_native_accept(&owner, actors[1], id, &mut ingress[1])?;
    assert_eq!(
        owner
            .accept_pending(context, actors[1], id, second, &mut ingress[1], now)
            .await
            .map_err(native::debug)?,
        NativeAcceptOutcome::Transferring,
    );
    Ok(owner)
}

fn native_frozen_trade(
    context: &TradeReadContext<'_, '_, '_>,
    owner: &TradeOwner,
    actor: TradeActor,
    identity: crate::durability::player_trade::PlayerTradeEventIdentity,
) -> TestResult<super::FrozenPlayerTrade> {
    let (issued, _) = owner
        .issued_transfer(context.runtime, actor.actor)
        .map_err(native::debug)?
        .ok_or("missing native trade occurrence")?;
    Ok(super::FrozenPlayerTrade::from_issued(
        issued,
        context.runtime,
        context.generation,
        identity,
    )?)
}

async fn native_commit_trade(
    context: &TradeReadContext<'_, '_, '_>,
    owner: &TradeOwner,
    actor: TradeActor,
    request: &super::FrozenPlayerTrade,
    first_current_fence: CurrentCharacterItemFence,
) -> Result<
    crate::durability::player_trade::PlayerTradeOutcome,
    crate::durability::player_trade::PlayerTradeError,
> {
    use super::PlayerTradeCommitAuthority;
    use crate::durability::player_trade::PlayerTradeError;
    let (issued, _) = owner
        .issued_transfer(context.runtime, actor.actor)
        .map_err(|_| PlayerTradeError::AuthorityRejected)?
        .ok_or(PlayerTradeError::AuthorityRejected)?;
    context
        .root
        .commit_player_trade(
            PlayerTradeCommitAuthority {
                recovery: context.recovery,
                node: context.node,
                runtime: context.runtime,
                issued,
                first_current_fence,
            },
            request,
        )
        .await
}

/// Full row snapshots are bounded by the disposable fixture, not a production
/// scan. A rejected swap must preserve values as well as relation counts.
async fn native_trade_snapshot(h: &native::Harness) -> TestResult<Vec<Vec<String>>> {
    let mut snapshot = Vec::new();
    for relation in [
        "game_player_trades",
        "game_player_trade_participants",
        "game_player_trade_receipts",
        "game_item_instances",
        "game_item_container_slots",
        "game_item_container_entries",
        "game_item_ground_locations",
        "game_item_mint_receipts",
        "game_item_transfer_reservations",
        "game_item_transfer_receipts",
        "game_item_transfer_quantity_evidence",
        "game_item_audit_outbox",
        "game_item_keys",
        "game_item_timed_states",
        "game_item_timed_state_writes",
        "game_character_roots",
    ] {
        snapshot.push(
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT row_to_json(t)::text FROM {relation} t ORDER BY 1"
            )))
            .fetch_all(&h.pool)
            .await?,
        );
    }
    Ok(snapshot)
}

fn native_trade_event_identity() -> crate::durability::player_trade::PlayerTradeEventIdentity {
    crate::durability::player_trade::PlayerTradeEventIdentity {
        transaction_id: native::id(240),
        event_id: native::id(241),
        occurred_at_unix_ms: 1_700_000_000_000,
    }
}

#[test]
fn native_trade_writer_swaps_same_definition_ids_once_and_reconciles_history() -> TestResult {
    use crate::durability::player_trade::{
        PlayerTradeError, PlayerTradeOutcome, decode_player_trade_envelope,
    };
    pg_trade("tradewriterswap", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let (issued, _) = owner
            .issued_transfer(&runtime, actors[0].actor)
            .map_err(native::debug)?
            .unwrap();
        let captured = issued.intent().binding.data.clone();
        let request =
            native_frozen_trade(&context, &owner, actors[0], native_trade_event_identity())?;
        let mut expected_counts = h.footprint().await?;
        let root_revisions: Vec<(String, String)> = sqlx::query_as(
            "SELECT character_id::text,character_revision::text FROM game_character_roots ORDER BY character_id",
        ).fetch_all(&h.pool).await?;
        let PlayerTradeOutcome::Committed(committed) =
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await?
        else {
            return Err("expected fresh native swap".into());
        };
        assert_eq!(
            committed.transaction_id,
            native_trade_event_identity().transaction_id
        );
        assert_eq!(committed.event_id, native_trade_event_identity().event_id);
        assert_eq!(committed.occurrence, *issued.occurrence());
        assert_eq!(committed.trade_id, *issued.trade_id());
        for side in [0, 1] {
            let bag = h
                .root
                .read_character_backpack(&recovery, actors[side].fence.character_id)
                .await
                .map_err(native::debug)?
                .unwrap();
            assert_eq!(bag.backpack, captured.backpacks[side].backpack);
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.item_instance_id, items[1 - side]);
            assert_eq!(
                bag.entries[0].item.quantity, 1,
                "equal definitions must not merge"
            );
            assert_eq!(bag.entries[0].placement_ordinal, 2);
            assert_eq!(h.item_state(items[side]).await?, (1, 1));
            let state: (String, String) = sqlx::query_as(
                "SELECT state_revision::text,last_transaction_id::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(items[side].as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                state.0.parse::<u64>()?,
                captured.offers[side].as_ref().unwrap().state_revision + 1
            );
            assert_eq!(state.1, native::uuid_text(committed.transaction_id));
        }
        // The four MINT/TRANSFER fixture outcomes remain intact; only one swap
        // event is added. Both pre-existing item identities survive unchanged.
        expected_counts[5] += 1;
        assert_eq!(h.footprint().await?, expected_counts);
        assert_eq!(h.count("game_player_trades").await?, 1);
        assert_eq!(h.count("game_player_trade_participants").await?, 2);
        assert_eq!(h.count("game_player_trade_receipts").await?, 2);
        let after_revisions: Vec<(String, String)> = sqlx::query_as(
            "SELECT character_id::text,character_revision::text FROM game_character_roots ORDER BY character_id",
        ).fetch_all(&h.pool).await?;
        assert_eq!(root_revisions, after_revisions);
        let same_transaction: bool = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM game_player_trade_participants p WHERE p.occurrence_id=c.occurrence_id AND p.created_xact_id=c.created_xact_id)=2 \
             AND (SELECT count(*) FROM game_player_trade_receipts r WHERE r.occurrence_id=c.occurrence_id AND r.created_xact_id=c.created_xact_id)=2 \
             AND EXISTS(SELECT 1 FROM game_item_audit_outbox a WHERE a.transaction_id=c.transaction_id AND a.created_xact_id=c.created_xact_id AND a.publication_state=1 AND a.published_at IS NULL) \
             FROM game_player_trades c WHERE occurrence_id=encode($1,'hex')::uuid",
        ).bind(committed.occurrence.as_slice()).fetch_one(&h.pool).await?;
        assert!(same_transaction);
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
        )
        .bind(committed.event_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let (envelope, audit) = decode_player_trade_envelope(&bytes).map_err(native::debug)?;
        assert_eq!(
            envelope.command_id,
            Some(issued.trigger().command_id().get())
        );
        assert_eq!(audit.lines.len(), 2);
        assert!(audit.lines.iter().all(|line| line.before == line.after));
        assert_eq!(audit.occurrence_id, committed.occurrence);

        // Historical outcome lookup is independent of current actor/session
        // validity and never re-applies placements or reissues an acceptance.
        h.tamper("UPDATE game_durability_reconnect_sessions SET current_generation=2")
            .await?;
        let mut probe = h.pool.begin().await?;
        for side in [0, 1] {
            assert!(
                crate::durability::item_transfer::character_item_fence_without_root_lock(
                    &mut probe,
                    &h.node,
                    &actors[side].fence,
                    native_command(side, 4),
                    actors[side].fence.character_id,
                    native::id(native::WORLD),
                    native::id(native::CHANNEL),
                )
                .await
                .map_err(native::debug)?
                .is_none()
            );
        }
        probe.rollback().await?;
        let after = native_trade_snapshot(h).await?;
        assert_eq!(
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await?,
            PlayerTradeOutcome::AlreadyCommitted(committed.clone())
        );
        assert_eq!(
            h.root.reconcile_player_trade(&recovery, &request).await?,
            Some(committed)
        );
        let mut different_identity = native_trade_event_identity();
        different_identity.event_id = native::id(242);
        let different = native_frozen_trade(&context, &owner, actors[0], different_identity)?;
        assert!(matches!(
            native_commit_trade(&context, &owner, actors[0], &different, actors[0].fence).await,
            Err(PlayerTradeError::ConflictingOccurrence)
        ));
        assert!(matches!(
            h.root.reconcile_player_trade(&recovery, &different).await,
            Err(PlayerTradeError::ConflictingOccurrence)
        ));
        assert_eq!(native_trade_snapshot(h).await?, after);
        Ok(())
    })
}

#[test]
fn native_trade_writer_rejects_item_changed_after_issuance_without_partial_swap() -> TestResult {
    use crate::durability::player_trade::PlayerTradeError;
    pg_trade("tradewriterstale", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let request =
            native_frozen_trade(&context, &owner, actors[0], native_trade_event_identity())?;
        let leaf = source_item_facts(generation, 3031)?;
        let extra = h
            .mint_definition(&recovery, leaf.definition.clone(), 1)
            .await?;
        let command = CommandRef::new(
            actors[0].fence.game_session_id,
            ingress[0].next_command_id().unwrap(),
        );
        reserve_fixture_command(&mut ingress[0], command, "pickup", extra)?;
        let mut pickup = native::to_backpack(command, extra, leaf);
        pickup.backpack = Some(source_item_facts(generation, 2854)?);
        assert!(matches!(
            h.transfer(&recovery, actors[0].fence, pickup)
                .await
                .map_err(native::debug)?,
            crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
        ));
        terminal_fixture_command(&mut ingress[0], command, "COMMITTED", extra)?;
        assert_eq!(h.item_state(items[0]).await?, (2, 1));
        // This direct native fixture bypasses the still-uninstalled physical
        // command hook, so it also proves that the DB writer cannot trust an
        // earlier in-memory reservation when another actual writer has changed it.
        let before = native_trade_snapshot(h).await?;
        assert!(matches!(
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await,
            Err(PlayerTradeError::Stale)
        ));
        assert_eq!(
            h.root.reconcile_player_trade(&recovery, &request).await?,
            None
        );
        assert_eq!(native_trade_snapshot(h).await?, before);
        assert_eq!(owner.phase(actors[0].actor), Some(Phase::Transferring));
        Ok(())
    })
}

#[test]
fn native_trade_writer_checks_real_owners_and_only_allows_first_connection_resume() -> TestResult {
    use super::PlayerTradeCommitAuthority;
    use crate::durability::player_trade::{
        PlayerTradeError, PlayerTradeOutcome, decode_player_trade_envelope,
    };
    pg_trade("tradewriterauthority", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let request =
            native_frozen_trade(&context, &owner, actors[0], native_trade_event_identity())?;
        let (issued, _) = owner
            .issued_transfer(&runtime, actors[0].actor)
            .map_err(native::debug)?
            .unwrap();
        // Only the already-terminal first accept resumes after TRANSFERRING.
        // The native current session row independently records generation 2.
        h.tamper(&format!(
            "UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id='{}'::uuid",
            native::uuid_text(native::id(native::SESSION)),
        )).await?;
        let mut current = actors[0].fence;
        current.connection_generation = ConnectionGeneration::new(2).map_err(native::debug)?;
        let before = native_trade_snapshot(h).await?;
        for component in 0..5 {
            let mut wrong = current;
            match component {
                0 => {
                    wrong.game_session_id =
                        GameSessionId::decode(&native::id(62)).map_err(native::debug)?
                }
                1 => wrong.character_lease_generation += 1,
                2 => {
                    wrong.scope_ownership_generation =
                        ScopeOwnershipGeneration::new(2).map_err(native::debug)?
                }
                3 => wrong.connection_generation = actors[0].fence.connection_generation,
                4 => wrong.character_id = actors[1].fence.character_id,
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    native_commit_trade(&context, &owner, actors[0], &request, wrong).await,
                    Err(PlayerTradeError::AuthorityRejected)
                ),
                "fence component {component}"
            );
            assert_eq!(native_trade_snapshot(h).await?, before);
        }
        // A genuine second registered node is still not this Channel's holder.
        let foreign_node = native::register(&h.root, 2).await?;
        assert!(matches!(
            h.root
                .commit_player_trade(
                    PlayerTradeCommitAuthority {
                        recovery: &recovery,
                        node: &foreign_node,
                        runtime: &runtime,
                        issued,
                        first_current_fence: current,
                    },
                    &request
                )
                .await,
            Err(PlayerTradeError::AuthorityRejected)
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        // Even another ready native Root connected to the same database cannot
        // consume the original Root's borrowed recovery authority.
        let other_root =
            DurabilityRoot::connect_test_runtime(&h.database.url).map_err(native::debug)?;
        assert!(
            other_root
                .maintain_ready_once()
                .await
                .map_err(native::debug)?
        );
        assert!(matches!(
            other_root
                .commit_player_trade(
                    PlayerTradeCommitAuthority {
                        recovery: &recovery,
                        node: &h.node,
                        runtime: &runtime,
                        issued,
                        first_current_fence: current,
                    },
                    &request
                )
                .await,
            Err(PlayerTradeError::AuthorityRejected)
        ));
        drop(other_root);
        assert_eq!(native_trade_snapshot(h).await?, before);
        // Vary only the trigger's current generation. That acceptance is still
        // pending and cannot acquire the first accept's continuity exception.
        h.tamper(&format!(
            "UPDATE game_durability_reconnect_sessions SET current_generation=2 WHERE game_session_id='{}'::uuid",
            native::uuid_text(native::id(61)),
        )).await?;
        assert!(matches!(
            native_commit_trade(&context, &owner, actors[0], &request, current).await,
            Err(PlayerTradeError::AuthorityRejected)
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        h.tamper(&format!(
            "UPDATE game_durability_reconnect_sessions SET current_generation=1 WHERE game_session_id='{}'::uuid",
            native::uuid_text(native::id(61)),
        )).await?;
        let PlayerTradeOutcome::Committed(committed) =
            native_commit_trade(&context, &owner, actors[0], &request, current).await?
        else {
            return Err("same-session first accept must resume".into());
        };
        let generations: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT game_session_id::text,connection_generation::text,current_connection_generation::text FROM game_player_trade_participants ORDER BY participant_ordinal",
        ).fetch_all(&h.pool).await?;
        assert_eq!(
            generations,
            vec![
                (
                    native::uuid_text(native::id(native::SESSION)),
                    "1".into(),
                    "2".into()
                ),
                (native::uuid_text(native::id(61)), "1".into(), "1".into()),
            ]
        );
        let bytes: Vec<u8> = sqlx::query_scalar(
            "SELECT envelope FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
        )
        .bind(committed.event_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        let (envelope, audit) = decode_player_trade_envelope(&bytes).map_err(native::debug)?;
        assert_eq!(audit.participants[0].current_connection_generation, 2);
        assert_eq!(audit.participants[1].current_connection_generation, 1);
        assert_eq!(
            envelope.connection_generation,
            Some(1),
            "the event is caused by the pending trigger"
        );
        Ok(())
    })
}

#[test]
fn native_trade_writer_rolls_back_second_placement_failure_and_retries_frozen_occurrence()
-> TestResult {
    use crate::durability::player_trade::{PlayerTradeError, PlayerTradeOutcome};
    pg_trade("tradewriterrollback", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let request =
            native_frozen_trade(&context, &owner, actors[0], native_trade_event_identity())?;
        let before = native_trade_snapshot(h).await?;
        let second_item = *items.iter().max().unwrap();
        // This test-owned failure occurs after both old entries were deleted,
        // both revisions changed and the first new entry was inserted.
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE FUNCTION trade_test_second_placement_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_instance_id='{}'::uuid AND NEW.placed_transaction_id='{}'::uuid THEN RAISE EXCEPTION 'injected second swap placement failure' USING ERRCODE='23514'; END IF; RETURN NEW; END; $$",
            native::uuid_text(second_item), native::uuid_text(native_trade_event_identity().transaction_id),
        ))).execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER trade_test_second_placement_failure BEFORE INSERT ON game_item_container_entries FOR EACH ROW EXECUTE FUNCTION trade_test_second_placement_failure()")
            .execute(&h.pool).await?;
        assert!(matches!(
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await,
            Err(PlayerTradeError::Storage(_))
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        assert_eq!(
            h.root.reconcile_player_trade(&recovery, &request).await?,
            None
        );
        sqlx::query(
            "DROP TRIGGER trade_test_second_placement_failure ON game_item_container_entries",
        )
        .execute(&h.pool)
        .await?;
        sqlx::query("DROP FUNCTION trade_test_second_placement_failure()")
            .execute(&h.pool)
            .await?;
        let PlayerTradeOutcome::Committed(committed) =
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await?
        else {
            return Err("frozen occurrence retry did not commit".into());
        };
        assert_eq!(
            h.root.reconcile_player_trade(&recovery, &request).await?,
            Some(committed.clone())
        );
        assert_eq!(
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await?,
            PlayerTradeOutcome::AlreadyCommitted(committed)
        );
        assert_eq!(h.count("game_player_trades").await?, 1);
        Ok(())
    })
}

#[test]
fn native_trade_migration_rejects_wrong_old_receipt_missing_audit_and_third_item_write()
-> TestResult {
    use crate::durability::player_trade::{PlayerTradeError, PlayerTradeOutcome};
    pg_trade("tradeguardshape", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let request =
            native_frozen_trade(&context, &owner, actors[0], native_trade_event_identity())?;
        // The third item stays on Ground and is unrelated to either captured
        // backpack. Its real MINT does not itself make the swap stale.
        let extra = h
            .mint_definition(
                &recovery,
                source_item_facts(generation, 3031)?.definition,
                1,
            )
            .await?;
        let before = native_trade_snapshot(h).await?;
        let faults = [
            ("game_player_trade_receipts", "BEFORE", "INSERT", "IF NEW.line_ordinal=2 THEN NEW.state_revision_before:=NEW.state_revision_before+1; END IF; RETURN NEW;".to_string()),
            ("game_player_trade_receipts", "BEFORE", "INSERT", "IF NEW.line_ordinal=2 THEN NEW.source_placement_ordinal:=NEW.source_placement_ordinal+1; END IF; RETURN NEW;".to_string()),
            ("game_item_audit_outbox", "BEFORE", "INSERT", format!("IF NEW.transaction_id='{}'::uuid THEN RETURN NULL; END IF; RETURN NEW;", native::uuid_text(native_trade_event_identity().transaction_id))),
            ("game_player_trades", "AFTER", "INSERT", format!("UPDATE game_item_instances SET last_transaction_id=NEW.transaction_id WHERE item_instance_id='{}'::uuid; RETURN NEW;", native::uuid_text(extra))),
        ];
        for (index, (relation, timing, operation, body)) in faults.into_iter().enumerate() {
            // Admin adds only a test-owned fault hook. The production writer,
            // all native guards and the candidate migration remain installed.
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE FUNCTION trade_test_shape_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN {body} END; $$",
            ))).execute(&h.pool).await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE TRIGGER trade_test_shape_fault {timing} {operation} ON {relation} FOR EACH ROW EXECUTE FUNCTION trade_test_shape_fault()",
            ))).execute(&h.pool).await?;
            assert!(
                matches!(
                    native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence)
                        .await,
                    Err(PlayerTradeError::Storage(
                        crate::durability::DurabilityError::CommitRejected
                    ))
                ),
                "independent deferred fault {index}"
            );
            assert_eq!(
                native_trade_snapshot(h).await?,
                before,
                "fault {index} leaked a row"
            );
            assert_eq!(
                h.root.reconcile_player_trade(&recovery, &request).await?,
                None
            );
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP TRIGGER trade_test_shape_fault ON {relation}",
            )))
            .execute(&h.pool)
            .await?;
            sqlx::query("DROP FUNCTION trade_test_shape_fault()")
                .execute(&h.pool)
                .await?;
        }
        // The exact same real issuance/request commits after each independently
        // injected invariant violation has been removed.
        assert!(matches!(
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await?,
            PlayerTradeOutcome::Committed(_)
        ));
        assert_eq!(h.item_state(extra).await?, (1, 1));
        assert!(h.on_ground(extra).await?);
        Ok(())
    })
}

#[test]
fn native_trade_writer_requires_capacity_before_removing_either_source() -> TestResult {
    use crate::durability::player_trade::PlayerTradeError;
    pg_trade("tradewritercapacity", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let request =
            native_frozen_trade(&context, &owner, actors[0], native_trade_event_identity())?;
        let bag = source_item_facts(generation, 2854)?;
        assert_eq!(bag.container_capacity, Some(20));
        // Fill only the first participant's destination through real MINT and
        // TRANSFER of 19 empty nonstackable containers. No synthetic placement
        // or changed Content capacity is used to manufacture a full backpack.
        for _ in 1..20 {
            let extra = h
                .mint_definition(&recovery, bag.definition.clone(), 1)
                .await?;
            let command = CommandRef::new(
                actors[0].fence.game_session_id,
                ingress[0].next_command_id().unwrap(),
            );
            reserve_fixture_command(&mut ingress[0], command, "pickup", extra)?;
            let mut pickup = native::to_backpack(command, extra, bag.clone());
            pickup.backpack = Some(bag.clone());
            assert!(matches!(
                h.transfer(&recovery, actors[0].fence, pickup)
                    .await
                    .map_err(native::debug)?,
                crate::durability::item_transfer::ItemTransferOutcome::Committed(_)
            ));
            terminal_fixture_command(&mut ingress[0], command, "COMMITTED", extra)?;
        }
        let full = h
            .root
            .read_character_backpack(&recovery, actors[0].fence.character_id)
            .await
            .map_err(native::debug)?
            .unwrap();
        assert_eq!(full.entries.len(), 20);
        let before = native_trade_snapshot(h).await?;
        assert!(matches!(
            native_commit_trade(&context, &owner, actors[0], &request, actors[0].fence).await,
            Err(PlayerTradeError::NoRoom)
        ));
        assert_eq!(
            h.root.reconcile_player_trade(&recovery, &request).await?,
            None
        );
        assert_eq!(native_trade_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn native_trade_migration_grants_only_fact_inserts_and_bounded_ordered_slot_locks() -> TestResult {
    pg_trade("tradeguardgrants", async |h| {
        seed_second_character_data(h).await?;
        let (controller, pin, _runtime, _objects, actors) = pg_runtime();
        activate_native_content(h, &pin).await?;
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        native_inventory(h, &recovery, controller.active().unwrap()).await?;
        for relation in [
            "game_player_trades",
            "game_player_trade_participants",
            "game_player_trade_receipts",
        ] {
            let privileges: bool = sqlx::query_scalar(
                "SELECT has_table_privilege('oteryn_game_runtime',$1,'SELECT') \
                 AND has_table_privilege('oteryn_game_runtime',$1,'INSERT') \
                 AND NOT has_table_privilege('oteryn_game_runtime',$1,'UPDATE,DELETE,TRUNCATE') \
                 AND has_table_privilege('oteryn_game_control',$1,'SELECT') \
                 AND NOT has_table_privilege('oteryn_game_control',$1,'INSERT,UPDATE,DELETE,TRUNCATE')",
            ).bind(relation).fetch_one(&h.pool).await?;
            assert!(privileges, "{relation}");
        }
        let privileges: bool = sqlx::query_scalar(
            "SELECT has_function_privilege('oteryn_game_runtime','game_player_trade_lock_slots(uuid,uuid)','EXECUTE') \
             AND NOT has_function_privilege('oteryn_game_control','game_player_trade_lock_slots(uuid,uuid)','EXECUTE') \
             AND NOT has_function_privilege('oteryn_game_runtime','game_player_trade_complete(uuid)','EXECUTE') \
             AND NOT has_table_privilege('oteryn_game_runtime','game_item_container_slots','UPDATE')",
        ).fetch_one(&h.pool).await?;
        assert!(privileges);
        let before = native_trade_snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let locked: i16 = sqlx::query_scalar(
            "SELECT game_player_trade_lock_slots(encode($1,'hex')::uuid,encode($2,'hex')::uuid)",
        )
        .bind(actors[0].fence.character_id.as_bytes().as_slice())
        .bind(actors[1].fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut *tx)
        .await?;
        assert_eq!(locked, 2);
        for actor in actors {
            // A separate connection observes the slot row lock, while the
            // helper has acquired no Character root or session authority.
            let mut observer = h.pool.begin().await?;
            let result = sqlx::query("SELECT character_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT")
                .bind(actor.fence.character_id.as_bytes().as_slice()).fetch_one(&mut *observer).await;
            assert!(
                result
                    .as_ref()
                    .err()
                    .and_then(|e| e.as_database_error())
                    .is_some_and(|e| e.code().as_deref() == Some("55P03"))
            );
            observer.rollback().await?;
            assert!(root_lock_is_available(&h.pool, actor.fence.character_id).await?);
        }
        tx.rollback().await?;
        for pair in [[actors[1], actors[0]], [actors[0], actors[0]]] {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            let result = sqlx::query("SELECT game_player_trade_lock_slots(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(pair[0].fence.character_id.as_bytes().as_slice())
                .bind(pair[1].fence.character_id.as_bytes().as_slice())
                .execute(&mut *tx).await;
            assert!(
                result
                    .as_ref()
                    .err()
                    .and_then(|e| e.as_database_error())
                    .is_some_and(|e| e.code().as_deref() == Some("23514"))
            );
            tx.rollback().await?;
        }
        // The helper is needed: a runtime caller still cannot take arbitrary
        // slot locks with SELECT FOR UPDATE or change a slot through UPDATE.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        let direct = sqlx::query("SELECT character_id FROM game_item_container_slots FOR UPDATE")
            .fetch_all(&mut *tx)
            .await;
        assert!(
            direct
                .as_ref()
                .err()
                .and_then(|e| e.as_database_error())
                .is_some_and(|e| e.code().as_deref() == Some("42501"))
        );
        tx.rollback().await?;
        assert_eq!(native_trade_snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn native_trade_fence_phase_precedes_both_root_locks_and_preserves_wrapper() -> TestResult {
    use crate::durability::item_transfer::{
        character_item_fence_is_current, character_item_fence_without_root_lock,
        lock_character_item_root,
    };
    pg_trade("tradefencelocks", async |h| {
        seed_second_character_data(h).await?;
        let seal = h.recovery.seal_current().map_err(native::debug)?;
        let _recovery = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(native::debug)?;
        let fences = native_fences();
        let (world, channel) = (native::id(native::WORLD), native::id(native::CHANNEL));
        let before = h.footprint().await?;
        let revisions: Vec<(String, String)> = sqlx::query_as(
            "SELECT character_id::text,character_revision::text \
             FROM game_character_roots ORDER BY character_id",
        )
        .fetch_all(&h.pool)
        .await?;
        let mut transaction = h.pool.begin().await?;
        let mut accounts = Vec::with_capacity(2);
        for side in [0, 1] {
            accounts.push(
                character_item_fence_without_root_lock(
                    &mut transaction,
                    &h.node,
                    &fences[side],
                    native_command(side, 1),
                    fences[side].character_id,
                    world,
                    channel,
                )
                .await
                .map_err(native::debug)?
                .ok_or("native fence rejected")?,
            );
            // Independent connection: after either native fence check, neither root
            // is locked. NOWAIT bounds the observation without a timeout/sleep race.
            for fence in fences {
                assert!(root_lock_is_available(&h.pool, fence.character_id).await?);
            }
        }
        let order = if fences[0].character_id.as_bytes() < fences[1].character_id.as_bytes() {
            [0, 1]
        } else {
            [1, 0]
        };
        for side in order {
            assert!(
                lock_character_item_root(&mut transaction, &fences[side], &accounts[side])
                    .await
                    .map_err(native::debug)?
            );
            assert!(!root_lock_is_available(&h.pool, fences[side].character_id).await?);
        }
        transaction.rollback().await?;

        // Legacy callers still validate and lock the one Character in the original order.
        let mut legacy = h.pool.begin().await?;
        assert!(
            character_item_fence_is_current(
                &mut legacy,
                &h.node,
                &fences[0],
                native_command(0, 1),
                fences[0].character_id,
                world,
                channel,
            )
            .await
            .map_err(native::debug)?
        );
        assert!(!root_lock_is_available(&h.pool, fences[0].character_id).await?);
        assert!(root_lock_is_available(&h.pool, fences[1].character_id).await?);
        legacy.rollback().await?;

        // Change only connection generation. Both entry points refuse before a root lock.
        let mut stale = fences[0];
        stale.connection_generation = ConnectionGeneration::new(2).map_err(native::debug)?;
        let mut transaction = h.pool.begin().await?;
        assert!(
            character_item_fence_without_root_lock(
                &mut transaction,
                &h.node,
                &stale,
                native_command(0, 1),
                stale.character_id,
                world,
                channel,
            )
            .await
            .map_err(native::debug)?
            .is_none()
        );
        assert!(
            !character_item_fence_is_current(
                &mut transaction,
                &h.node,
                &stale,
                native_command(0, 1),
                stale.character_id,
                world,
                channel,
            )
            .await
            .map_err(native::debug)?
        );
        assert!(root_lock_is_available(&h.pool, fences[0].character_id).await?);
        transaction.rollback().await?;

        // The separate root phase retains the original account comparison.
        let mut transaction = h.pool.begin().await?;
        assert_ne!(accounts[0], accounts[1]);
        assert!(
            !lock_character_item_root(&mut transaction, &fences[0], &accounts[1])
                .await
                .map_err(native::debug)?
        );
        transaction.rollback().await?;
        assert_eq!(h.footprint().await?, before);
        let after: Vec<(String, String)> = sqlx::query_as(
            "SELECT character_id::text,character_revision::text \
             FROM game_character_roots ORDER BY character_id",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            after, revisions,
            "root locks never increment CharacterRevision"
        );
        Ok(())
    })
}

#[test]
fn native_trade_owner_freezes_before_storage_and_terminalizes_only_the_trigger() -> TestResult {
    pg_trade("tradeownercomplete", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let mut owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let (first, trigger, trade_id) = {
            let (issued, _) = owner
                .issued_transfer(&runtime, actors[0].actor)
                .map_err(native::debug)?
                .unwrap();
            (issued.first_accept(), issued.trigger(), *issued.trade_id())
        };
        let first_semantic = owner
            .accept_semantic(first, trade_id)
            .map_err(native::debug)?;
        let trigger_semantic = owner
            .accept_semantic(trigger, trade_id)
            .map_err(native::debug)?;
        let before = native_trade_snapshot(h).await?;
        // An otherwise valid pending trigger cannot touch storage before the exact
        // immutable native request has been retained in the owner.
        assert!(matches!(
            owner
                .drive_transfer(&context, actors[1].actor, &mut ingress[1], actors[0].fence)
                .await,
            Err(OwnerError::Ingress(TradeIngressRefusal::UnfrozenTransfer))
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        let identity = native_trade_event_identity();
        owner.freeze_transfer(&runtime, generation, actors[1].actor, identity)?;
        owner.freeze_transfer(&runtime, generation, actors[1].actor, identity)?;
        let mut changed = identity;
        changed.event_id = native::id(242);
        assert!(matches!(
            owner.freeze_transfer(&runtime, generation, actors[1].actor, changed),
            Err(OwnerError::Ingress(
                TradeIngressRefusal::ConflictChangedInput
            ))
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        // These are independent refusals: the first actor has genuine terminal
        // consent but does not own the pending trigger; the second borrow has no
        // pending command. Neither check is a substitute for session installation.
        assert!(matches!(
            owner
                .drive_transfer(&context, actors[0].actor, &mut ingress[0], actors[0].fence)
                .await,
            Err(OwnerError::Authority)
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        let mut empty = CommandIngress::new();
        assert!(matches!(
            owner
                .drive_transfer(&context, actors[1].actor, &mut empty, actors[0].fence)
                .await,
            Err(OwnerError::Ingress(TradeIngressRefusal::MissingPending))
        ));
        assert_eq!(native_trade_snapshot(h).await?, before);
        assert_eq!(owner.phase(actors[1].actor), Some(Phase::Transferring));
        assert_eq!(owner.by_actor.len(), 2);

        let NativeTradeCompletion::Completed(committed) = owner
            .drive_transfer(&context, actors[1].actor, &mut ingress[1], actors[0].fence)
            .await?
        else {
            return Err("native owner did not complete the swap".into());
        };
        assert_eq!(committed.transaction_id, identity.transaction_id);
        assert_eq!(committed.event_id, identity.event_id);
        assert!(owner.trades.is_empty() && owner.by_actor.is_empty());
        assert!(matches!(
            ingress[0].classify_duplicate(first.command_id(), &first_semantic),
            DuplicateDisposition::ReplayRetainedOutcome(outcome) if outcome.disposition() == "ACCEPTED"
        ));
        assert!(matches!(
            ingress[1].classify_duplicate(trigger.command_id(), &trigger_semantic),
            DuplicateDisposition::ReplayRetainedOutcome(outcome)
                if outcome.disposition() == "COMPLETED" && outcome.state() == trade_hex(&trade_id)
        ));
        assert_eq!(ingress[1].outstanding(), 0);
        assert_eq!(h.count("game_player_trades").await?, 1);
        assert_eq!(h.count("game_player_trade_participants").await?, 2);
        assert_eq!(h.count("game_player_trade_receipts").await?, 2);
        for side in [0, 1] {
            let bag = h
                .root
                .read_character_backpack(&recovery, actors[side].fence.character_id)
                .await
                .map_err(native::debug)?
                .unwrap();
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.item_instance_id, items[1 - side]);
        }
        Ok(())
    })
}

#[test]
fn native_trade_owner_reconciles_committed_history_before_checking_current_fences() -> TestResult {
    pg_trade("tradeownerhistory", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let mut owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        owner.freeze_transfer(
            &runtime,
            generation,
            actors[1].actor,
            native_trade_event_identity(),
        )?;
        let key = owner.transferring_key(actors[1].actor)?;
        // Commit the actual retained request, deliberately leaving its continuation
        // unfinished. No caller-provided outcome is installed into the owner.
        let request = &owner
            .trades
            .get(&key)
            .unwrap()
            .retained_swap
            .as_ref()
            .unwrap()
            .request;
        let PlayerTradeOutcome::Committed(committed) =
            native_commit_trade(&context, &owner, actors[0], request, actors[0].fence).await?
        else {
            return Err("expected fresh native commit".into());
        };
        assert!(
            owner
                .trades
                .get(&key)
                .unwrap()
                .retained_swap
                .as_ref()
                .unwrap()
                .resolution
                .is_none()
        );
        h.tamper("UPDATE game_durability_reconnect_sessions SET current_generation=2")
            .await?;
        let after_commit = native_trade_snapshot(h).await?;
        assert_eq!(
            owner
                .drive_transfer(&context, actors[1].actor, &mut ingress[1], actors[0].fence)
                .await?,
            NativeTradeCompletion::Completed(Box::new(committed))
        );
        assert!(owner.trades.is_empty() && owner.by_actor.is_empty());
        assert_eq!(native_trade_snapshot(h).await?, after_commit);
        assert_eq!(h.count("game_player_trades").await?, 1);
        Ok(())
    })
}

#[test]
fn native_trade_owner_retains_native_result_when_output_refuses_and_retries_without_storage()
-> TestResult {
    pg_trade("tradeowneroutput", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let mut owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        owner.freeze_transfer(
            &runtime,
            generation,
            actors[1].actor,
            native_trade_event_identity(),
        )?;
        let key = owner.transferring_key(actors[1].actor)?;
        let (trigger, semantic) = {
            let (issued, _) = owner
                .issued_transfer(&runtime, actors[1].actor)
                .map_err(native::debug)?
                .unwrap();
            (
                issued.trigger(),
                owner.accept_semantic(issued.trigger(), *issued.trade_id())?,
            )
        };
        // This executes the real writer and caches its receipt before any FND output.
        owner
            .resolve_transfer(&context, actors[1].actor, &ingress[1], actors[0].fence)
            .await?;
        let Some(ResolvedTradeSwap::Committed(committed)) = owner
            .trades
            .get(&key)
            .unwrap()
            .retained_swap
            .as_ref()
            .unwrap()
            .resolution
            .as_ref()
        else {
            return Err("native committed result was not retained".into());
        };
        let committed = committed.clone();
        assert!(matches!(
            ingress[1].classify_duplicate(trigger.command_id(), &semantic),
            DuplicateDisposition::PendingOriginal
        ));
        let after_commit = native_trade_snapshot(h).await?;
        let mut missing = CommandIngress::new();
        assert!(matches!(
            owner.terminalize_resolved_transfer(actors[1].actor, &mut missing),
            Err(OwnerError::Ingress(TradeIngressRefusal::MissingPending))
        ));
        assert_eq!(owner.phase(actors[1].actor), Some(Phase::Transferring));
        assert_eq!(owner.by_actor.len(), 2);
        assert!(matches!(
            owner.trades.get(&key).unwrap().retained_swap.as_ref().unwrap().resolution.as_ref(),
            Some(ResolvedTradeSwap::Committed(value)) if value == &committed
        ));

        // Hold the exact native occurrence lock from a separate transaction. A
        // second reader/writer call would block; cached output must finish while
        // this lock is held, without another swap or even a storage lookup.
        let mut lock = h.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:player-trade:' || encode($1,'hex'),0))")
            .bind(committed.occurrence.as_slice()).execute(&mut *lock).await?;
        let result = tokio::time::timeout(
            Duration::from_millis(200),
            owner.drive_transfer(&context, actors[1].actor, &mut ingress[1], actors[0].fence),
        )
        .await;
        lock.rollback().await?;
        assert_eq!(result??, NativeTradeCompletion::Completed(committed));
        assert!(owner.trades.is_empty() && owner.by_actor.is_empty());
        assert_eq!(native_trade_snapshot(h).await?, after_commit);
        assert!(matches!(
            ingress[1].classify_duplicate(trigger.command_id(), &semantic),
            DuplicateDisposition::ReplayRetainedOutcome(outcome) if outcome.disposition() == "COMPLETED"
        ));
        Ok(())
    })
}

#[test]
fn native_trade_owner_closes_rejection_only_after_actual_commit_rejection() -> TestResult {
    pg_trade("tradeownercommitreject", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let mut owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        owner.freeze_transfer(
            &runtime,
            generation,
            actors[1].actor,
            native_trade_event_identity(),
        )?;
        let (trigger, semantic) = {
            let (issued, _) = owner
                .issued_transfer(&runtime, actors[1].actor)
                .map_err(native::debug)?
                .unwrap();
            (
                issued.trigger(),
                owner.accept_semantic(issued.trigger(), *issued.trade_id())?,
            )
        };
        // Fault only the real COMMIT boundary. The nontransactional sequence proves
        // execution of this initially deferred trigger despite rollback; a pre-TX
        // rejection or an unexecuted writer cannot satisfy the observation.
        sqlx::raw_sql(
            "CREATE SEQUENCE trade_owner_commit_seen; \
             CREATE FUNCTION trade_owner_reject_commit() RETURNS trigger LANGUAGE plpgsql \
             SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN \
             PERFORM nextval('public.trade_owner_commit_seen'); \
             RAISE EXCEPTION 'trade owner commit fault' USING ERRCODE='23514'; END; $$; \
             CREATE CONSTRAINT TRIGGER zz_trade_owner_commit_fault AFTER INSERT ON game_player_trades \
             DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION trade_owner_reject_commit();",
        ).execute(&h.pool).await?;
        let before = native_trade_snapshot(h).await?;
        assert_eq!(
            owner
                .drive_transfer(&context, actors[1].actor, &mut ingress[1], actors[0].fence)
                .await?,
            NativeTradeCompletion::Refused(CloseReason::Rejected)
        );
        let seen: (i64, bool) =
            sqlx::query_as("SELECT last_value,is_called FROM trade_owner_commit_seen")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(seen, (1, true));
        assert!(owner.trades.is_empty() && owner.by_actor.is_empty());
        assert_eq!(native_trade_snapshot(h).await?, before);
        assert_eq!(h.count("game_player_trades").await?, 0);
        assert!(matches!(
            ingress[1].classify_duplicate(trigger.command_id(), &semantic),
            DuplicateDisposition::ReplayRetainedOutcome(outcome) if outcome.disposition() == "REJECTED"
        ));
        Ok(())
    })
}

#[test]
fn native_trade_owner_retains_the_same_request_when_cancelled_at_actual_commit() -> TestResult {
    pg_trade("tradeownercancelcommit", async |h| {
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
        let mut ingress = [CommandIngress::new(), CommandIngress::new()];
        let items =
            native_inventory_with_ingress(h, &recovery, generation, Some(&mut ingress)).await?;
        let context = TradeReadContext {
            runtime: &runtime,
            root: &h.root,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        let mut owner =
            native_transferring_with_ingress(&context, actors, items, &mut ingress, Instant::now())
                .await?;
        let identity = native_trade_event_identity();
        owner.freeze_transfer(&runtime, generation, actors[1].actor, identity)?;
        let key = owner.transferring_key(actors[1].actor)?;
        let (trigger, semantic, occurrence) = {
            let (issued, _) = owner
                .issued_transfer(&runtime, actors[1].actor)
                .map_err(native::debug)?
                .unwrap();
            (
                issued.trigger(),
                owner.accept_semantic(issued.trigger(), *issued.trade_id())?,
                *issued.occurrence(),
            )
        };
        let other_root =
            DurabilityRoot::connect_test_runtime(&h.database.url).map_err(native::debug)?;
        assert!(
            other_root
                .maintain_ready_once()
                .await
                .map_err(native::debug)?
        );
        sqlx::raw_sql(
            "CREATE FUNCTION trade_owner_wait_commit() RETURNS trigger LANGUAGE plpgsql \
             SECURITY DEFINER SET search_path=pg_catalog,public AS $$ BEGIN \
             PERFORM pg_advisory_xact_lock(1129321017::bigint); RETURN NEW; END; $$; \
             CREATE CONSTRAINT TRIGGER zz_trade_owner_commit_wait AFTER INSERT ON game_player_trades \
             DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION trade_owner_wait_commit();",
        ).execute(&h.pool).await?;
        let mut gate = h.pool.begin().await?;
        let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *gate)
            .await?;
        sqlx::query("SELECT pg_advisory_xact_lock(1129321017::bigint)")
            .execute(&mut *gate)
            .await?;
        let mut attempt = Box::pin(owner.drive_transfer(
            &context,
            actors[1].actor,
            &mut ingress[1],
            actors[0].fence,
        ));
        let observed = tokio::time::timeout(Duration::from_millis(1500), async {
            loop {
                let mut observation = std::pin::pin!(
                    sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.datname=current_database() \
                         AND a.state='active' AND upper(btrim(a.query))='COMMIT' \
                         AND a.wait_event_type='Lock' AND $1=ANY(pg_blocking_pids(a.pid)))",
                    ).bind(gate_pid).fetch_one(&h.pool)
                );
                let blocked = std::future::poll_fn(|task| {
                    if let std::task::Poll::Ready(result) =
                        std::future::Future::poll(attempt.as_mut(), task)
                    {
                        return std::task::Poll::Ready(Err::<bool, Box<dyn std::error::Error>>(
                            format!("trade continuation returned before the COMMIT gate: {result:?}").into(),
                        ));
                    }
                    std::future::Future::poll(observation.as_mut(), task)
                        .map(|result| result.map_err(Into::into))
                }).await?;
                if blocked {
                    return Ok::<(), Box<dyn std::error::Error>>(());
                }
                tokio::task::yield_now().await;
            }
        }).await;
        // Dropping the borrowed native future cancels its continuation while
        // PostgreSQL is demonstrably waiting inside COMMIT. It supplies no outcome.
        drop(attempt);
        // Release the server gate only after dropping that continuation. The
        // database may now commit or abort; the owner still knows neither result.
        gate.commit().await?;
        observed??;
        let retained = owner
            .trades
            .get(&key)
            .unwrap()
            .retained_swap
            .as_ref()
            .unwrap();
        assert_eq!(retained.identity, identity);
        assert!(retained.resolution.is_none());
        assert_eq!(owner.phase(actors[1].actor), Some(Phase::Transferring));
        assert_eq!(owner.by_actor.len(), 2);
        assert!(matches!(
            ingress[1].classify_duplicate(trigger.command_id(), &semantic),
            DuplicateDisposition::PendingOriginal
        ));
        // A new native Root is real but cannot use the original Root's recovery
        // authority. This pre-TX rejection cannot settle the earlier uncertainty.
        let foreign_context = TradeReadContext {
            root: &other_root,
            runtime: &runtime,
            recovery: &recovery,
            node: &h.node,
            room: pin.qualified_room(),
            objects: &objects,
            generation,
        };
        assert!(matches!(
            owner
                .drive_transfer(
                    &foreign_context,
                    actors[1].actor,
                    &mut ingress[1],
                    actors[0].fence
                )
                .await,
            Err(OwnerError::Swap(PlayerTradeError::AuthorityRejected))
        ));
        assert_eq!(owner.phase(actors[1].actor), Some(Phase::Transferring));
        assert!(
            owner
                .trades
                .get(&key)
                .unwrap()
                .retained_swap
                .as_ref()
                .unwrap()
                .resolution
                .is_none()
        );
        // PostgreSQL may complete the old COMMIT or observe cancellation and
        // roll it back. Neither is guessed here; only the real occurrence-locked
        // recovery reader may decide whether this exact request needs a retry.
        drop(other_root);
        let completion = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                h.root.maintain_ready_once().await.map_err(native::debug)?;
                match owner
                    .drive_transfer(&context, actors[1].actor, &mut ingress[1], actors[0].fence)
                    .await
                {
                    Ok(value) => return Ok::<_, Box<dyn std::error::Error>>(value),
                    Err(OwnerError::Swap(PlayerTradeError::Storage(_))) => {
                        assert_eq!(owner.phase(actors[1].actor), Some(Phase::Transferring));
                        let retained = owner
                            .trades
                            .get(&key)
                            .unwrap()
                            .retained_swap
                            .as_ref()
                            .unwrap();
                        assert_eq!(retained.identity, identity);
                        assert!(retained.resolution.is_none());
                        tokio::task::yield_now().await;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        })
        .await??;
        let NativeTradeCompletion::Completed(committed) = completion else {
            return Err("cancelled native continuation failed to reconcile/complete".into());
        };
        assert_eq!(committed.occurrence, occurrence);
        assert_eq!(committed.transaction_id, identity.transaction_id);
        assert_eq!(committed.event_id, identity.event_id);
        assert!(owner.trades.is_empty() && owner.by_actor.is_empty());
        assert_eq!(h.count("game_player_trades").await?, 1);
        assert_eq!(h.count("game_player_trade_participants").await?, 2);
        assert_eq!(h.count("game_player_trade_receipts").await?, 2);
        let events: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_audit_outbox WHERE event_id=encode($1,'hex')::uuid",
        )
        .bind(identity.event_id.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(events, 1);
        for side in [0, 1] {
            let bag = h
                .root
                .read_character_backpack(&recovery, actors[side].fence.character_id)
                .await
                .map_err(native::debug)?
                .unwrap();
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.item_instance_id, items[1 - side]);
        }
        assert!(matches!(
            ingress[1].classify_duplicate(trigger.command_id(), &semantic),
            DuplicateDisposition::ReplayRetainedOutcome(outcome) if outcome.disposition() == "COMPLETED"
        ));
        Ok(())
    })
}
