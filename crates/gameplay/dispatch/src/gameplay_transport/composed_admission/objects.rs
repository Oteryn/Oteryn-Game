use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// USE-WIRE-V1 reach (#162 5868482467): same floor, Chebyshev distance <=1 from any of the
    /// target's own collision cells, no line-of-sight check. A pure function so it is directly
    /// unit-testable independent of any Channel runtime: the qualified native entry room is too
    /// small (#935/#937: every accepted walkable cell is already within one step of the door) to
    /// exercise a genuine TOO_FAR case through real movement.
    pub(in crate::gameplay_transport) fn use_object_reachable(
        actor_x: i32,
        actor_y: i32,
        actor_floor: i32,
        target_cells: &std::collections::BTreeSet<crate::content::LogicalCell>,
    ) -> bool {
        target_cells.iter().any(|cell| {
            oteryn_gameplay_items::reach::within_reach(
                (actor_x, actor_y, actor_floor),
                (cell.x, cell.y, cell.z),
            )
        })
    }

    /// C2 routing: the chest placement of `chest` that `placement` names, if any. The client
    /// names only the placement; the chest, its claim and its reward come from Content.
    pub(in crate::gameplay_transport) fn chest_target<'c>(
        chest: &'c crate::content::CanonicalReferencePlayableContent,
        placement: &[u8],
    ) -> Option<&'c crate::content::PlacementRef> {
        chest
            .placements
            .iter()
            .find(|candidate| candidate.key.as_str().as_bytes() == placement)
    }

    /// MAP-DOOR-1: one `USE_INTENT` on a door of a bundle World. Under the Channel-owner lock
    /// that `step` takes, so the actor and every other committed position cannot move between
    /// the occupancy read and the door's compare-and-swap.
    pub(in crate::gameplay_transport) async fn use_bundle_door(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &crate::map::boot::BundleMap,
        target: &oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        let mut runtime = self.runtime.lock().await;
        if self.states.spell_states.lock().await.is_dead(actor) {
            return UseOutcome::rejected();
        }
        // The USE's own GameSession must still be the actor's bound session with no control
        // loss, read under the same lock as the commit (a stale or superseded session acts on
        // nothing).
        if !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return UseOutcome::rejected();
        }
        // Admission is taken under the owner lock that also covers the commit below, and the
        // owner pass resets the turn cap under the same lock, so an act is counted in the turn
        // that commits it and its order is the owner lock's.
        let admitted = self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()));
        if !admitted {
            return UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Exhausted,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            };
        }
        let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
            return UseOutcome::rejected();
        };
        let native = |position: crate::foundation::MovementLocalPosition| {
            Some(crate::map::overlay::TilePos {
                x: u16::try_from(position.x).ok()?,
                y: u16::try_from(position.y).ok()?,
                floor: i8::try_from(position.floor).ok()?.checked_neg()?,
            })
        };
        let Some(actor_tile) = native(expected.position()) else {
            return UseOutcome::rejected();
        };
        map_door::use_door(
            &map.overlay,
            &map.facts,
            runtime.content_pin().client_artifact_digest(),
            actor_tile,
            |door| map_door::doorway_occupied(&runtime, actor, actor_tile, door),
            target,
        )
    }

    /// C2: one `USE_INTENT` on the entry chest (USE-WIRE-V1 field 1, #162 5914960502). Reach is
    /// the door's rule (same floor, Chebyshev distance <=1 from the chest's cell), read under
    /// the Channel-owner lock, which is released before the durable MINT so the Channel is
    /// never blocked on the database. The MINT itself is DUR-03's
    /// (`interaction_chest_use::use_chest`), fenced by the admitted session's item fence and
    /// keyed by this `USE`'s `CommandRef`: a mint is `COMMITTED` with no overlay delta, a claim
    /// already taken is `NOTHING_TO_USE`, anything else `REJECTED` with nothing written.
    pub(in crate::gameplay_transport) async fn use_chest(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        chest: &crate::content::PlacementRef,
    ) -> UseOutcome {
        use crate::interaction_chest_use::{ChestUseRequest, entry_chest, use_chest};
        // The chest must stand in this Channel's World (D39 review 5906461018 item 7).
        if chest.address.world_id != self.world_id {
            return UseOutcome::rejected();
        }
        {
            let mut runtime = self.runtime.lock().await;
            if self.states.spell_states.lock().await.is_dead(actor) {
                return UseOutcome::rejected();
            }
            let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
                return UseOutcome::rejected();
            };
            let position = expected.position();
            let cells = std::collections::BTreeSet::from([chest.address.cell]);
            if !Self::use_object_reachable(
                position.x,
                position.y,
                i32::from(position.floor),
                &cells,
            ) {
                return UseOutcome {
                    disposition: oteryn_protocol_oteryn::world_object::UseDisposition::TooFar,
                    committed: None,
                    earned: connection::EarnedNotice::NoneEarned,
                    gate_message_id: None,
                };
            }
        }
        let Ok(command_id) = crate::foundation::CommandId::new(command.command_id) else {
            return UseOutcome::rejected();
        };
        let request = ChestUseRequest {
            command: crate::foundation::CommandRef::new(command.game_session_id, command_id),
            chest: chest.key.clone(),
            content_revision: entry_chest::CONTENT_REVISION.to_owned(),
            ruleset_revision: entry_chest::RULESET_REVISION.to_owned(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
        };
        let session = crate::combat::DurabilitySession {
            root: self.root,
            authority: self.character,
            node: self.holder,
        };
        let (disposition, _, notice) = use_chest(
            &session,
            self.chest,
            self.achievements,
            command.item_fence,
            request,
        )
        .await;
        // QUEST-STATE-0 §5.4: a chest that advances a quest left a PENDING obligation with its
        // claim; request it in this session. An outcome that is not proven is picked up by the
        // owner cadence after the backoff.
        if crate::interaction_chest_use::resolve_chest(self.chest, &chest.key)
            .is_ok_and(|resolved| resolved.quest_transition.is_some())
        {
            match disposition {
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed => {
                    self.refresh_quest_session(command.game_session_id, false)
                        .await;
                }
                oteryn_protocol_oteryn::world_object::UseDisposition::Rejected => {
                    self.schedule_quest_refresh(command.game_session_id);
                }
                _ => {}
            }
        }
        UseOutcome {
            disposition,
            committed: None,
            earned: earned_notice(self.achievements, notice, &mut operator_event),
            gate_message_id: None,
        }
    }

    pub(in crate::gameplay_transport) async fn observe_object_overlay(
        &self,
    ) -> Option<oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry> {
        if self
            .rooms
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some())
            || self.bundle_map().is_some()
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let door = self.door.lock().await;
        Some(
            oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry {
                content_generation: runtime.content_pin().client_artifact_digest(),
                placement: door.placement_key().as_str().as_bytes().to_vec(),
                state: door.state_key().as_str().as_bytes().to_vec(),
                revision: door.revision(),
            },
        )
    }

    pub(in crate::gameplay_transport) async fn use_world_object(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        if let Some(outcome) = self.open_daily_shrine(actor, command, &target).await {
            return outcome;
        }
        if self
            .rooms
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some())
        {
            return UseOutcome::rejected();
        }
        if let Some(map) = self.bundle_map() {
            if let Some(outcome) = self.use_bundle_chest(actor, command, map, &target).await {
                return outcome;
            }
            return self
                .use_bundle_door(actor, command.game_session_id, map, &target)
                .await;
        }
        if let Some(chest) = Self::chest_target(self.chest, &target.placement) {
            return self.use_chest(actor, command, chest).await;
        }
        let mut runtime = self.runtime.lock().await;
        // DEATH-2 §4.2: a dead player takes no input until its respawn.
        if self.states.spell_states.lock().await.is_dead(actor) {
            return UseOutcome::rejected();
        }
        let Ok(expected) = runtime.borrow_movement_position().read(actor) else {
            return UseOutcome::rejected();
        };
        let position = expected.position();
        let mut door = self.door.lock().await;
        if target.placement.as_slice() != door.placement_key().as_str().as_bytes() {
            return UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            };
        }
        let actor_floor = i32::from(position.floor);
        if !Self::use_object_reachable(position.x, position.y, actor_floor, door.collision_cells())
        {
            return UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::TooFar,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            };
        }
        // #162 5868482467 (shared-lease P1 repair r4121956127): occupancy is every currently
        // positioned committed actor in the Channel, not only the issuer — including one
        // retained during disconnect grace, which stays positioned (only its `control_loss`
        // mark changes, never its slot's `committed`/`position`). Read under the same `runtime`
        // lock and the same work item as `attempt_use` below, so this is TOCTOU-free: nothing
        // else can move an actor or change the door's state between this read and that call.
        let occupied: std::collections::BTreeSet<crate::content::LogicalCell> = runtime
            .committed_player_positions()
            .into_iter()
            .map(|other| crate::content::LogicalCell {
                x: other.x,
                y: other.y,
                z: i32::from(other.floor),
            })
            .collect();
        let content_generation = runtime.content_pin().client_artifact_digest();
        match door.attempt_use(target.expected_revision, &occupied) {
            Ok(crate::world_runtime::LocalObjectUseOutcome::Committed { state, revision }) => {
                UseOutcome {
                    disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
                    committed: Some(
                        oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry {
                            content_generation,
                            placement: door.placement_key().as_str().as_bytes().to_vec(),
                            state: state.as_str().as_bytes().to_vec(),
                            revision,
                        },
                    ),
                    earned: connection::EarnedNotice::NoneEarned,
                    gate_message_id: None,
                }
            }
            Ok(crate::world_runtime::LocalObjectUseOutcome::NothingToUse) => UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::NothingToUse,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::Occupied) => UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::Occupied,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::StaleState) => UseOutcome {
                disposition: oteryn_protocol_oteryn::world_object::UseDisposition::StaleState,
                committed: None,
                earned: connection::EarnedNotice::NoneEarned,
                gate_message_id: None,
            },
            Ok(crate::world_runtime::LocalObjectUseOutcome::Rejected) | Err(_) => {
                UseOutcome::rejected()
            }
        }
    }
}
