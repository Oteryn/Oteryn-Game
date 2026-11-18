//! Crystal 201189f movements_boss_entrance.lua:1–54.
use super::*;
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterRequest, ActivityHandoffOwner, ActivityParticipant,
    ActivityRefusal,
};
use crate::durability::character_last_position::LastPositionCell;
use crate::foundation::{CharacterId, MovementLocalPosition, MovementPositionSnapshot};
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use std::num::NonZeroU32;

#[derive(Clone, Copy)]
pub(super) struct WarzonePortal {
    key: &'static str,
    destination: LastPositionCell,
}

pub(super) fn at(map: &BundleMap, position: MovementLocalPosition) -> Option<WarzonePortal> {
    if position.floor != 15 {
        return None;
    }
    let (boss, x, y) = match (position.x, position.y) {
        (33460, 32267) => ("oteryn:boss.the_baron_from_below", 33650, 32310),
        (33324, 32109) => ("oteryn:boss.the_count_of_the_core", 33681, 32338),
        (33275, 32316) => ("oteryn:boss.the_duke_of_the_depths", 33717, 32302),
        _ => return None,
    };
    let tile = TilePos {
        x: position.x.try_into().ok()?,
        y: position.y.try_into().ok()?,
        floor: -15,
    };
    let base = map.overlay.base().tile(tile.x, tile.y, tile.floor)?;
    let is_portal = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .any(|(ordinal, (id, _))| {
            let Some(ordinal) = u8::try_from(ordinal).ok() else {
                return false;
            };
            let Some(key) = placement_key(tile, ordinal) else {
                return false;
            };
            !map.facts.base_hidden(key)
                && map
                    .facts
                    .quest_callback_matches(key, None, Some(57243), None)
                && map
                    .facts
                    .base_entry(tile, ordinal, *id)
                    .is_some_and(|entry| entry.appearance_id == 1949)
        });
    is_portal.then_some(WarzonePortal {
        key: boss,
        destination: LastPositionCell { x, y, floor: 15 },
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn finish_warzone_portal_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: Option<MovementPositionSnapshot>,
        entered: MovementPositionSnapshot,
        result: Option<Result<(), ActivityRefusal>>,
    ) -> StepOutcome {
        if result == Some(Ok(())) {
            return StepOutcome {
                disposition: StepDisposition::Moved,
                moved_to: None,
            };
        }
        let message = if result == Some(Err(ActivityRefusal::Cooldown)) {
            "You have to wait to face this boss again!"
        } else {
            "Sorry, not possible."
        };
        let mut arrived = entered;
        self.states.chat.push_quest_feedback(session, message);
        if let (Some(origin), Some(map), Some(_permit)) =
            (origin, self.bundle_map(), self.spell_lane_permit().await)
            && let Some((mut runtime, states)) = self.lock_quest_teleport_owners().await
            && runtime.read_actor_position(actor) == Ok(entered)
        {
            let blocked =
                super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
            let doors = crate::map::floor::BundleDoors(map.facts.doors());
            if let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                entered,
                origin.position(),
                blocked,
                &doors,
            ) && let Ok(returned) = runtime.commit_source_step(proof)
            {
                arrived = returned;
            }
        }
        let runtime = self.runtime.lock().await;
        StepOutcome {
            disposition: StepDisposition::Moved,
            moved_to: self.wire_observation(&runtime, arrived.position()),
        }
    }

    pub(super) async fn enter_warzone_portal(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        portal: WarzonePortal,
    ) -> Option<Result<(), ActivityRefusal>> {
        use super::super::native_activity::{NativeActivityOutcome, boss_room_native_ready};
        let admitted = self.activity_runtimes.connection(session)?;
        let fence = admitted.item_fence?;
        if admitted.runtime_actor != Some(actor)
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return None;
        }
        // The paced step has released the spell lane. The first entrant can
        // now publish its room before a second entrant selects an allocation.
        let _admission = self.states.warzone_admission.lock().await;
        let origin_pin = {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered) {
                return None;
            }
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let owner = ActivityHandoffOwner::new(self.root.clone());
        if owner
            .portal_boss_on_cooldown(self.character, self.holder, fence, portal.key)
            .await
            .ok()?
        {
            return Some(Err(ActivityRefusal::Cooldown));
        }
        let definition = owner
            .read_portal_boss_room(self.character, self.holder, fence, portal.key, origin_pin)
            .await
            .ok()??;
        if definition.policy.cooldown_ms != 36_000_000
            || definition.lever_positions.as_slice()
                != [LastPositionCell {
                    x: entered.position().x,
                    y: entered.position().y,
                    floor: 15,
                }]
            || definition.entry_positions.as_slice() != [portal.destination]
        {
            return Some(Err(ActivityRefusal::ContentChanged));
        }
        let existing = owner
            .open_portal_room(self.character, self.holder, fence, portal.key)
            .await
            .ok()?;
        let joining = existing.is_some();
        if !joining && let Err(reason) = self.ensure_activity_template_runtime(&definition).await {
            return Some(Err(reason));
        }
        let mut after = None;
        let (handoff, allocation, destination) = loop {
            let candidates = if let Some((handoff, allocation)) = &existing {
                vec![(*handoff, allocation.clone())]
            } else {
                owner
                    .ready_allocations(self.character, self.holder, admitted.world_id, after)
                    .await
                    .ok()?
                    .into_iter()
                    .map(|allocation| {
                        Some((*SecureIdentifiers.game_session_id()?.as_bytes(), allocation))
                    })
                    .collect::<Option<Vec<_>>>()?
            };
            let mut selected = None;
            for (handoff, allocation) in &candidates {
                let Some(registered) = self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                else {
                    continue;
                };
                if registered.assignment_source_revision != allocation.source_revision
                    || registered.template.as_ref().is_none_or(|template| {
                        template.key != definition.room_template_key
                            || template.revision != definition.template_revision
                    })
                {
                    continue;
                }
                let runtime = registered.runtime.lock().await;
                if runtime.owner_fence().is_ok()
                    && (joining || boss_room_native_ready(&runtime, &definition))
                {
                    drop(runtime);
                    selected = Some((*handoff, allocation.clone(), registered));
                    break;
                }
            }
            if let Some(selected) = selected {
                break selected;
            }
            if joining || candidates.len() < 15 {
                return Some(Err(ActivityRefusal::NoCapacity));
            }
            let (_, tail) = candidates.last()?;
            let RuntimeScopeRefV1::Instance { instance_id, .. } = tail.scope else {
                return None;
            };
            if after.is_some_and(|previous| previous >= instance_id) {
                return None;
            }
            after = Some(instance_id);
        };
        let (participant, command_id) = {
            let runtime = self.runtime.lock().await;
            let movement = runtime.committed_movement_occurrence(actor).ok()??;
            if runtime.read_actor_position(actor) != Ok(entered) || movement.after() != entered {
                return None;
            }
            let saved = self.placed_record(&runtime, entered.position());
            let exit = self.placed_record(
                &runtime,
                MovementLocalPosition {
                    x: definition.exit_position.x,
                    y: definition.exit_position.y,
                    floor: definition.exit_position.floor,
                },
            );
            let command = u64::from_be_bytes(movement.binding()[..8].try_into().ok()?).max(1);
            (
                ActivityParticipant {
                    source: fence,
                    destination_game_session_id: SecureIdentifiers.game_session_id()?,
                    destination_transport: Some(SecureIdentifiers.transport_ref()?),
                    selected_capabilities: admitted
                        .continuity
                        .selected_capabilities
                        .as_slice()
                        .to_vec(),
                    source_position: saved.record,
                    source_position_write_sequence: saved.write_sequence,
                    destination_cell: portal.destination,
                    exit_position: exit.record,
                },
                command,
            )
        };
        let destination_content = {
            let runtime = destination.runtime.lock().await;
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let request = ActivityEnterRequest {
            handoff_id: handoff,
            command_id,
            initiating_character_id: CharacterId::decode(fence.character_id.as_bytes()).ok()?,
            activity_key: definition.boss_key,
            template_revision: definition.template_revision,
            content_revision: definition.content_revision,
            destination_scope: allocation.scope,
            destination_generation: allocation.generation,
            destination_assignment_source_revision: allocation.source_revision,
            destination_content,
            time_limit_ms: definition.policy.time_limit_ms,
            participants: vec![participant],
        };
        match self
            .enter_native_boss_window(request, NonZeroU32::new(8000)?)
            .await
            .ok()?
        {
            NativeActivityOutcome::Committed(commit) => Some(
                self.publish_activity_commit(&commit)
                    .map_err(|_| ActivityRefusal::AuthorityRejected),
            ),
            NativeActivityOutcome::Refused(reason) => Some(Err(reason)),
        }
    }
}
