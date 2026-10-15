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
