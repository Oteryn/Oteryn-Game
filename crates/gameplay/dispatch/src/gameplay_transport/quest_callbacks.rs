//! QUESTCB2: content callbacks on literal USE and committed step ingress.
use super::{
    ComposedFreshAdmission,
    actor_spell::ChannelSpellStates,
    connection::{UseCommand, UseOutcome},
};
mod execute;
mod rewards;

use super::paced_movement::underwater_drown::underwater;
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
    quest_world_callback::QuestWorldContext,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot,
};
use crate::map::{
    boot::BundleMap,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position, Trigger};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::OnceLock;

#[derive(Clone)]
struct Pending {
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    callback: String,
    key: u64,
    revision: u64,
    cause: QuestCause,
    origin: Option<MovementPositionSnapshot>,
    context: Option<QuestWorldContext>,
    ingress_settled: bool,
}

#[derive(Default)]
pub(super) struct PendingCallbacks {
    records: std::sync::Mutex<std::collections::HashMap<GameSessionId, Pending>>,
    #[cfg(test)]
    pub(super) fixture: OnceLock<Vec<Callback>>,
    #[cfg(test)]
    pub(super) interrupt_after_commit: std::sync::atomic::AtomicBool,
}

fn callbacks() -> &'static [Callback] {
    static DATA: OnceLock<Vec<Callback>> = OnceLock::new();
    DATA.get_or_init(|| world_callbacks::load().unwrap_or_default())
}
fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}
fn native(p: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: p.x.try_into().ok()?,
        y: p.y.try_into().ok()?,
        floor: i8::try_from(p.floor).ok()?.checked_neg()?,
    })
}
fn matches(map: &BundleMap, callback: &Callback, key: u64) -> bool {
    let Some((pos, ordinal)) = crate::map::view::placement_of(key) else {
        return false;
    };
    let selector = &callback.selector;
    selector.position.is_none_or(|p| tile(p) == pos)
        && (selector.position.is_none()
            || selector.unique.is_some()
            || selector.action.is_some()
            || selector.appearance.is_some()
            || ordinal == 0)
        && map
            .facts
            .quest_callback_matches(key, selector.unique, selector.action, None)
        && (map
            .facts
            .quest_callback_matches(key, None, None, selector.appearance)
            || (selector.position.is_some()
                && selector.appearance.is_some_and(|appearance| {
                    // Plain placements are omitted from the sparse attribute index. Resolve
                    // their palette through the selected base stack, never a client item id.
                    map.overlay
                        .base()
                        .tile(pos.x, pos.y, pos.floor)
                        .and_then(|tile| {
                            tile.ids()
                                .iter()
                                .zip(tile.depths())
                                .filter(|(_, depth)| **depth == 0)
                                .nth(usize::from(ordinal))
                        })
                        .and_then(|(id, _)| map.facts.base_entry(pos, ordinal, *id))
                        .is_some_and(|facts| facts.appearance_id == appearance)
                })))
}

fn step_command(
    actor: ExactActorRef,
    session: GameSessionId,
    revision: u64,
    epoch: u64,
) -> Option<CommandRef> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn:quest-world-step:v1")
        .chain_update(actor.placement_identity())
        .chain_update(epoch.to_be_bytes())
        .chain_update(revision.to_be_bytes())
        .finalize();
    let mut id = [0; 8];
    id.copy_from_slice(&digest[..8]);
    // The full actor, movement revision and epoch also enter the durable binding:
    // a compact-id collision refuses, rather than replaying another step's reward.
    CommandId::new(u64::from_be_bytes(id).max(1))
        .ok()
        .map(|id| CommandRef::new(session, id))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Resolve a closed transport's reserved USE before the session stream can reopen.
    /// Historical receipt recovery does not grant player control or install a projection.
    pub(super) async fn reconcile_quest_callback_receipts(&self, session: GameSessionId) -> bool {
        let pending = match self.states.quest_callbacks.records.lock() {
            Ok(records) => records
                .get(&session)
                .into_iter()
                .filter(|p| matches!(p.cause, QuestCause::Use(_)) && !p.ingress_settled)
                .cloned()
                .collect::<Vec<_>>(),
            Err(_) => return false,
        };
        if pending.is_empty() {
            return true;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        for pending in pending {
            let QuestCause::Use(command) = pending.cause else {
                return false;
            };
            let (Some(context), Some(catalogue)) = (pending.context, self.quest_catalogue.as_ref())
            else {
                return false;
            };
            let Ok(Some(fence)) = self.current_quest_fence(pending.session).await else {
                return false;
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let disposition = match slot.commit_world_callback(self.root, self.character, self.holder, fence,
                QuestTransitionRequest { transition_key: pending.callback, cause: pending.cause },
                std::sync::Arc::clone(catalogue), &permit, context).await {
                Ok(QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)) => UseDisposition::Committed,
                Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(_)) => return false,
                Ok(_) | Err(_) => UseDisposition::Rejected,
            };
            if self
                .states
                .command_ingress
                .complete_quest_callback(command, disposition)
                .is_none()
            {
                return false;
            }
            let Ok(mut records) = self.states.quest_callbacks.records.lock() else {
                return false;
            };
            if disposition == UseDisposition::Committed {
                if let Some(record) = records.get_mut(&pending.session) {
                    record.ingress_settled = true;
                }
            } else {
                records.remove(&pending.session);
            }
        }
        true
    }

    pub(super) fn has_pending_quest_callback(&self, session: GameSessionId) -> bool {
        self.states
            .quest_callbacks
            .records
            .lock()
            .map_or(true, |p| p.contains_key(&session))
    }

    fn callback_content(&self) -> &[Callback] {
        #[cfg(test)]
        if let Some(content) = self.states.quest_callbacks.fixture.get() {
            return content;
        }
        callbacks()
    }

    /// Before further player input, reconcile an interrupted commit with its original cause.
    /// The Channel's existing input gate closes a connection if a result is still unknown.
    pub(super) async fn reconcile_quest_callbacks(&self, session: GameSessionId) -> bool {
        if !self.reconcile_quest_callback_receipts(session).await {
            return false;
        }
        let pending = match self.states.quest_callbacks.records.lock() {
            Ok(p) => p.get(&session).cloned().into_iter().collect::<Vec<_>>(),
            Err(_) => return false,
        };
        if pending.is_empty() {
            return true;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        for pending in pending {
            let Some(map) = self.bundle_map() else {
                return false;
            };
            let Some(callback) = self
                .callback_content()
                .iter()
                .find(|c| c.key == pending.callback)
            else {
                return false;
            };
            let Ok(Some(fence)) = self.current_quest_fence(pending.session).await else {
                return false;
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let optional_pvp = self
                .special_movement_policy(&permit, pending.actor, pending.session)
                .await;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if runtime.read_actor_position(pending.actor) != Ok(pending.expected) {
                return false;
            }
            let result = self
                .execute_quest_callback(
                    &permit,
                    &mut runtime,
                    &mut states,
                    pending.actor,
                    pending.session,
                    map,
                    callback,
                    pending.key,
                    pending.revision,
                    pending.cause,
                    Some(fence),
                    Some(&mut slot),
                    optional_pvp,
                )
                .await;
            if result != UseDisposition::Committed {
                if self.has_pending_quest_callback(pending.session) {
                    return false;
                }
                if let Some(origin) = pending.origin {
                    self.refuse_quest_step(
                        optional_pvp,
                        &mut runtime,
                        &mut states,
                        pending.actor,
                        pending.session,
                        map,
                        callback,
                        pending.expected,
                        origin,
                    );
                }
            }
        }
        true
    }

    pub(super) async fn use_quest_callback(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let source = super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: self.active_generation?.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let selected = self
            .callback_content()
            .iter()
            .filter(|c| c.trigger == Trigger::Use && matches(map, c, resolved.placement_key))
            .collect::<Vec<_>>();
        let [callback] = selected.as_slice() else {
            return None;
        };
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let Some(permit) = self.spell_lane_permit().await else {
                return UseDisposition::Rejected;
            };
            let fence = if callback.storage.is_empty() {
                None
            } else {
                self.current_quest_fence(command.game_session_id)
                    .await
                    .ok()
                    .flatten()
            };
            if fence.is_some_and(|f| command.item_fence != Some(f.item_fence())) {
                return UseDisposition::Rejected;
            }
            let mut slot = match fence {
                Some(f) => Some(self.revision_sequencer.acquire(f.character_id).await),
                None => None,
            };
            let optional_pvp = self
                .special_movement_policy(&permit, actor, command.game_session_id)
                .await;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return UseDisposition::Rejected;
            };
            let Some(at) = native(expected.position()) else {
                return UseDisposition::Rejected;
            };
            if !super::map_door::reachable(at, resolved.pos) {
                return UseDisposition::TooFar;
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut g| g.admit(actor, self.clock_origin.elapsed()))
            {
                return UseDisposition::Exhausted;
            }
            let Ok(id) = CommandId::new(command.command_id) else {
                return UseDisposition::Rejected;
            };
            let result = self.execute_quest_callback(
                &permit,
                &mut runtime,
                &mut states,
                actor,
                command.game_session_id,
                map,
                callback,
                resolved.placement_key,
                target.expected_revision,
                QuestCause::Use(CommandRef::new(command.game_session_id, id)),
                fence,
                slot.as_mut(),
                optional_pvp,
            )
            .await;
            drop(states);
            drop(runtime);
            drop(slot);
            if matches!(result, UseDisposition::Committed | UseDisposition::Rejected)
                && let Some(fence) = fence
                && let Some(delivery) = self.deliver_monk_reward(actor, command, callback, fence).await
            {
                if delivery == UseDisposition::Committed { return delivery; }
                if result == UseDisposition::Committed {
                    self.states.chat.push_quest_feedback(command.game_session_id,
                        "Your shrine reward is earned. Make room and capacity in your backpack and use this shrine again to collect it.");
                    return result;
                }
                return delivery;
            }
            result
        })
        .await
        .unwrap_or(UseDisposition::Rejected);
        Some(UseOutcome {
            disposition: result,
            ..UseOutcome::rejected()
        })
    }

    /// The ordinary step already holds the spell lane. Release its runtime guard before
    /// acquiring the quest revision slot, then compare the movement snapshot on re-entry.
    pub(super) async fn step_quest_callback(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let Some(at) = native(entered.position()) else {
            return entered;
        };
        let Some(base) = map.overlay.base().tile(at.x, at.y, at.floor) else {
            return entered;
        };
        let selected = (0..base.depths().iter().filter(|d| **d == 0).count())
            .filter_map(|o| {
                let key = placement_key(at, u8::try_from(o).ok()?)?;
                if map.facts.base_hidden(key) {
                    return None;
                }
                Some(
                    self.callback_content()
                        .iter()
                        .filter(move |c| c.trigger == Trigger::StepIn && matches(map, c, key))
                        .map(move |c| (c, key)),
                )
            })
            .flatten()
            .collect::<Vec<_>>();
        // Ambiguous content never chooses one action by incidental registration order.
        let [(callback, key)] = selected.as_slice() else {
            return entered;
        };
        let Some(command) = step_command(actor, session, entered.revision(), self.reset_epoch)
        else {
            return entered;
        };
        let cause = QuestCause::Step(command);
        let operation = async {
            let fence = if callback.storage.is_empty() {
                None
            } else {
                self.current_quest_fence(session).await.ok().flatten()
            };
            let mut slot = match fence {
                Some(f) => Some(self.revision_sequencer.acquire(f.character_id).await),
                None => None,
            };
            let optional_pvp = self.special_movement_policy(permit, actor, session).await;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered) {
                return entered;
            }
            let revision = map.facts.object_revision(*key);
            let result = self
                .execute_quest_callback(
                    permit,
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    map,
                    callback,
                    *key,
                    revision,
                    cause,
                    fence,
                    slot.as_mut(),
                    optional_pvp,
                )
                .await;
            let unresolved =
                self.states
                    .quest_callbacks
                    .records
                    .lock()
                    .map_or(true, |mut records| {
                        if let Some(pending) = records.get_mut(&session) {
                            pending.origin = Some(origin);
                            true
                        } else {
                            false
                        }
                    });
            if result != UseDisposition::Committed && !unresolved {
                self.refuse_quest_step(
                    optional_pvp,
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    map,
                    callback,
                    entered,
                    origin,
                );
            }
            runtime.read_actor_position(actor).unwrap_or(entered)
        };
        match tokio::time::timeout(std::time::Duration::from_secs(10), operation).await {
            Ok(snapshot) => snapshot,
            Err(_) => {
                if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                    let pending = records.entry(session).or_insert(Pending {
                        actor,
                        session,
                        expected: entered,
                        callback: callback.key.clone(),
                        key: *key,
                        revision: map.facts.object_revision(*key),
                        cause,
                        origin: Some(origin),
                        context: None,
                        ingress_settled: false,
                    });
                    pending.origin = Some(origin);
                }
                entered
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn refuse_quest_step(
        &self,
        optional_pvp: Option<bool>,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        entered: MovementPositionSnapshot,
        origin: MovementPositionSnapshot,
    ) {
        if !callback.return_on_failure {
            return;
        }
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let blocked = super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            runtime,
            self.movement_cells,
            actor,
            session,
            entered,
            origin.position(),
            blocked,
            &closed,
        ) && let Ok(snapshot) = runtime.commit_source_step(proof)
        {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                self.owner_now().get(),
                Some(entered.position()),
                snapshot.position(),
                optional_pvp,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                self.owner_now().get(),
                underwater(map, entered.position()),
                underwater(map, snapshot.position()),
            );
        }
        if let Some(message) = &callback.failure_message {
            self.states.chat.push_quest_feedback(session, message);
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
