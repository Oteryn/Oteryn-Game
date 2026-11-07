//! WVAR step admission. The map selects physical tiles; only the current durable
//! World decision and Character/session/generation fences authorize a passage.
use super::{ComposedFreshAdmission, paced_movement::underwater_drown::underwater};
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_world_change::variants;

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn world_change_entry_clock(
        &self,
        slot: &mut crate::durability::character_revision_sequencer::RevisionSlot,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> (bool, bool) {
        use crate::durability::quest_state::{
            NativeWorldChangeEntry, QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use oteryn_gameplay_quests::quest::{QuestRefusal, world_change_entry as rule};
        let Some(command) = super::quest_callbacks::step_command(
            actor,
            session,
            entered.revision(),
            self.reset_epoch,
        ) else {
            return (false, false);
        };
        let source = {
            let runtime = self.runtime.lock().await;
            match NativeWorldChangeEntry::from_current_runtime(
                &runtime,
                actor,
                command,
                entered,
                self.reset_epoch,
                fence.item_fence(),
            ) {
                Ok(None) => return (true, false),
                Ok(Some(source)) => source,
                Err(_) => return (false, false),
            }
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return (false, false);
        };
        for transition in [rule::ENTER, rule::REFRESH] {
            let request = QuestTransitionRequest {
                transition_key: transition.into(),
                cause: QuestCause::Step(command),
            };
            match slot
                .commit_world_change_entry(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    request,
                    std::sync::Arc::clone(catalogue),
                    source.clone(),
                )
                .await
            {
                Ok(
                    QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_),
                ) => return (transition == rule::ENTER, true),
                Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))
                    if transition == rule::ENTER => {}
                _ => return (false, false),
            }
        }
        (false, false)
    }

    pub(super) async fn world_change_step(
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
        let p = entered.position();
        let (Ok(x), Ok(y), Ok(z)) = (
            u16::try_from(p.x),
            u16::try_from(p.y),
            i8::try_from(p.floor),
        ) else {
            return entered;
        };
        let position = (x, y, z);
        let at = TilePos { x, y, floor: -z };
        let Some(tile) = map.overlay.base().tile(x, y, -z) else {
            return entered;
        };
        let matches = variants::definitions()
            .into_iter()
            .flatten()
            .flat_map(|d| &d.variants)
            .flat_map(|v| &v.portals)
            .any(|portal| {
                portal.position == position
                    && tile
                        .ids()
                        .iter()
                        .zip(tile.depths())
                        .filter(|(_, d)| **d == 0)
                        .enumerate()
                        .any(|(ordinal, (id, _))| {
                            u8::try_from(ordinal)
                                .ok()
                                .and_then(|o| placement_key(at, o).map(|key| (o, key)))
                                .is_some_and(|(o, key)| {
                                    !map.facts.base_hidden(key)
                                        && map.facts.quest_callback_matches(
                                            key,
                                            portal.unique,
                                            portal.action,
                                            None,
                                        )
                                        && map
                                            .facts
                                            .base_entry(at, o, *id)
                                            .is_some_and(|f| f.appearance_id == portal.appearance)
                                })
                        })
            });
        if !matches {
            return entered;
        }
        let Some(mut fence) = self.current_quest_fence(session).await.ok().flatten() else {
            return entered;
        };
        if fence.runtime_scope.world_id() != self.world_id {
            return entered;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let Ok(revision) = slot.cursor(self.root, self.character).await else {
            return entered;
        };
        fence.expected_character_revision = revision;
        {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered)
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return entered;
            }
        }
        let mut destination = self
            .root
            .world_change_passage(
                self.character,
                self.holder,
                fence,
                self.reset_epoch,
                position,
                &self.premium_coordinator,
            )
            .await;
        let mut refresh = false;
        if destination.as_ref().is_ok_and(|value| value.is_some()) {
            let (allowed, changed) = self
                .world_change_entry_clock(&mut slot, fence, actor, session, entered)
                .await;
            refresh = changed;
            if !allowed {
                destination = Ok(None);
            }
        }
        drop(slot);
        if refresh {
            self.refresh_quest_session(session, false).await;
        }
        // Denial returns to the exact previous tile, including a failed current read.
        let target = match destination {
            Ok(Some((x, y, z))) => MovementLocalPosition {
                x: i32::from(x),
                y: i32::from(y),
                floor: i16::from(z),
            },
            _ => origin.position(),
        };
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return entered;
        };
        if runtime.owner_fence().is_err()
            || runtime.read_actor_position(actor) != Ok(entered)
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
        {
            return entered;
        }
        let blocked = super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells,
            actor,
            session,
            entered,
            target,
            blocked,
            &closed,
        ) else {
            return entered;
        };
        let Ok(snapshot) = runtime.commit_source_step(proof) else {
            return entered;
        };
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(entered.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            self.owner_now().get(),
            underwater(map, entered.position()),
            underwater(map, snapshot.position()),
        );
        snapshot
    }
}
