//! ITEM-TIMED-1: real command, admitted content and server-owned duration cadence.
use super::item_ref_admission::{generation_item_facts, pinned_index};
pub(super) use super::timed_item_content::facts;
use super::{ComposedFreshAdmission, connection::UseCommand, item_view::ItemKey};
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_timed_runtime::{TimedOperation, TimedRequest};
use crate::durability::item_timed_state::{
    TimedDefinitionFacts, TimedExpiryTarget, TimedWriteError,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::timed_item::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct TimedRuntime {
    pub(super) imbuements: tokio::sync::Mutex<HashMap<GameSessionId, u64>>,
    pub(super) sessions: tokio::sync::Mutex<HashMap<GameSessionId, SessionClock>>,
    pub(super) next_tick_ms: std::sync::atomic::AtomicU64,
}
#[derive(Default)]
pub(super) struct SessionClock {
    recent: [Option<u64>; 5],
    pub(super) items: HashMap<[u8; 16], ItemClock>,
    pub(super) serial: u64,
}
pub(super) struct ItemClock {
    pub(super) at_ms: u64,
    pub(super) pending: Option<(TimedRequest, u64)>,
}

fn refusal(disposition: TimedDisposition) -> Option<Vec<u8>> {
    encode_timed_item_result(TimedItemResult::refused(disposition)).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn timed_item_command(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        payload: &[u8],
        target: Option<ItemKey>,
    ) -> Option<Vec<u8>> {
        self.timed_command(actor, command, payload, target, false)
            .await
    }

    pub(super) async fn timed_command(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        payload: &[u8],
        target: Option<ItemKey>,
        server_use: bool,
    ) -> Option<Vec<u8>> {
        let binding: [u8; 32] = if server_use {
            let Some(ItemKey::Instance(item)) = target else {
                return refusal(TimedDisposition::Rejected);
            };
            Sha256::new()
                .chain_update(b"oteryn:timed-standard-use:v1")
                .chain_update(item)
                .finalize()
                .into()
        } else {
            Sha256::digest(payload).into()
        };
        let id = |tag: &[u8]| {
            super::npc_trade_command::derived(tag, command.game_session_id, command.command_id)
        };
        let transaction_id = id(b"oteryn:timed-command:v1");
        match self
            .root
            .read_timed_command(
                self.character,
                command.game_session_id,
                transaction_id,
                binding,
            )
            .await
        {
            Ok(Some(result)) => {
                let mut clocks = self.states.timed_items.sessions.lock().await;
                if let Some(clock) = clocks.get_mut(&command.game_session_id) {
                    for lane in clock.items.values_mut() {
                        if let Some((request, at)) = &lane.pending
                            && request.transaction_id == transaction_id
                        {
                            if result.disposition == TimedDisposition::Ok {
                                lane.at_ms = *at;
                            }
                            lane.pending = None;
                        }
                    }
                }
                return encode_timed_item_result(result).ok();
            }
            Ok(None) => (),
            Err(TimedWriteError::Unavailable(_)) => return None,
            Err(_) => return refusal(TimedDisposition::Rejected),
        }
        let Ok(intent) = decode_timed_item_intent(payload) else {
            return refusal(TimedDisposition::Rejected);
        };
        let Some(ItemKey::Instance(item)) = target else {
            return refusal(TimedDisposition::Stale);
        };
        let _lane = self.spell_lane_permit().await?;
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(item))
        {
            return refusal(TimedDisposition::Rejected);
        }
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
            return None;
        };
        if command.item_fence != Some(fence.item_fence()) {
            return refusal(TimedDisposition::Rejected);
        }
        let native = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_err()
                || self.states.spell_states.lock().await.is_dead(actor)
                || pinned_index(&runtime, self.active_generation).is_none()
            {
                return refusal(TimedDisposition::Rejected);
            }
            if let Some(npc) = intent.npc {
                let visible = runtime.visible_entities();
                let Some(speaker) = visible.players.iter().find(|p| p.actor == actor) else {
                    return refusal(TimedDisposition::Rejected);
                };
                let Some(placed) = visible.npcs.iter().find(|p| {
                    p.actor.placement_identity() == npc.identity && p.generation == npc.generation
                }) else {
                    return refusal(TimedDisposition::Stale);
                };
                if runtime.npc_key(placed.actor) != Some("oteryn:npc.aldo")
                    || speaker.position.floor != placed.position.floor
                    || speaker.position.x.abs_diff(placed.position.x) > 3
                    || speaker.position.y.abs_diff(placed.position.y) > 3
                {
                    return refusal(TimedDisposition::Rejected);
                }
            }
            self.active_generation?.native_gameplay()?
        };
        {
            let now = u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX);
            let mut clocks = self.states.timed_items.sessions.lock().await;
            let clock = clocks.entry(command.game_session_id).or_default();
            let Some(slot) = clock
                .recent
                .iter_mut()
                .find(|at| at.is_none_or(|at| now.saturating_sub(at) >= 1_000))
            else {
                return refusal(TimedDisposition::Rejected);
            };
            *slot = Some(now);
        }
        // Read only this Character's bounded carried tree. Item UUIDs and definitions never
        // come from the payload; the transaction revalidates ownership under its root lock.
        let inventory = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .ok()?;
        let equipment =
            super::equipment_move::read_current_equipment(self, command.game_session_id).await;
        let definition = inventory
            .as_ref()
            .and_then(|bag| {
                bag.entries
                    .iter()
                    .find(|entry| entry.item.item_instance_id == item)
                    .map(|entry| entry.item.definition.clone())
            })
            .or_else(|| {
                equipment.as_ref().and_then(|eq| {
                    eq.items
                        .iter()
                        .find(|entry| entry.item_instance_id == item)
                        .map(|entry| entry.definition.clone())
                })
            });
        let Some(definition) = definition else {
            return refusal(TimedDisposition::Rejected);
        };
        let (definition, target, operation) = if intent.action == TimedAction::Repair {
            if definition.production_key != "oteryn:item.tibia.i6530" {
                return refusal(TimedDisposition::Rejected);
            }
            let index = native.item_index()?;
            let def = |key: &str| {
                Some(TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: index.revision(key)?.into(),
                })
            };
            let repaired = def("oteryn:item.tibia.i6529")?;
            let (full, _) = facts(native, &repaired)?;
            let bag = inventory.as_ref()?;
            let coins = FeeChangeFacts {
                item_instance_ids: [id(b"oteryn:repair-change-0"), id(b"oteryn:repair-change-1")],
                gold: def("oteryn:item.tibia.i3031")?,
                platinum: def("oteryn:item.tibia.i3035")?,
                crystal: def("oteryn:item.tibia.i3043")?,
                backpack: generation_item_facts(native, &bag.backpack.definition).ok()?,
            };
            (
                TimedDefinitionFacts {
                    definition,
                    ..full.clone()
                },
                Some(TimedExpiryTarget {
                    definition: repaired,
                    full: Some(full.full),
                    lit_continuous: false,
                }),
                TimedOperation::Repair(Box::new(coins)),
            )
        } else {
            let Some((full, target)) = facts(native, &definition) else {
                return refusal(TimedDisposition::Rejected);
            };
            (
                full,
                target,
                if intent.action == TimedAction::Use {
                    TimedOperation::Use { elapsed_ms: 0 }
                } else {
                    TimedOperation::Inspect
                },
            )
        };
        // The same session clock lock serializes equipment stops, charge use and the owner tick.
        let mut clocks = self.states.timed_items.sessions.lock().await;
        let now = u64::try_from(self.clock_origin.elapsed().as_millis()).unwrap_or(u64::MAX);
        let operation = if matches!(operation, TimedOperation::Use { .. }) {
            let elapsed = match clocks
                .get(&command.game_session_id)
                .and_then(|clock| clock.items.get(&item))
            {
                Some(clock) if clock.pending.is_some() => return None,
                Some(clock) => now.saturating_sub(clock.at_ms),
                None => 0,
            };
            TimedOperation::Use {
                elapsed_ms: elapsed,
            }
        } else {
            operation
        };
        let request = TimedRequest {
            transaction_id,
            content_digest: native.source_digest(),
            event_id: id(b"oteryn:timed-event:v1"),
            binding,
            item,
            expected_revision: if server_use {
                self.root
                    .read_timed_row(self.character, self.holder, fence, item)
                    .await
                    .ok()?
                    .map_or(0, |row| row.revision)
            } else {
                intent.expected_revision
            },
            definition,
            target,
            operation,
            occurred_at_unix_ms: super::npc_trade_command::unix_ms_now(),
        };
        let is_use = matches!(request.operation, TimedOperation::Use { .. });
        if is_use
            && let Some(lane) = clocks
                .get_mut(&command.game_session_id)
                .and_then(|clock| clock.items.get_mut(&item))
        {
            // Retain the elapsed debit before issuing: an ambiguous commit must replay
            // this exact request before any later owner tick can consume its interval.
            lane.pending = Some((request.clone(), now));
        }
        match self
            .root
            .commit_timed_action(self.character, self.holder, fence, request)
            .await
        {
            Ok(result) => {
                if is_use
                    && let Some(clock) = clocks
                        .get_mut(&command.game_session_id)
                        .and_then(|clock| clock.items.get_mut(&item))
                {
                    if result.disposition == TimedDisposition::Ok {
                        clock.at_ms = now;
                    }
                    clock.pending = None;
                }
                encode_timed_item_result(result).ok()
            }
            Err(TimedWriteError::Unavailable(_)) => None,
            Err(error) => {
                if is_use
                    && let Some(clock) = clocks
                        .get_mut(&command.game_session_id)
                        .and_then(|clock| clock.items.get_mut(&item))
                {
                    clock.pending = None;
                }
                refusal(
                    if matches!(error, TimedWriteError::RevisionMismatch { .. }) {
                        TimedDisposition::Stale
                    } else {
                        TimedDisposition::Rejected
                    },
                )
            }
        }
    }
}
