//! Same-node House transfers on the composed owner and the existing native carrier.
use super::connection::{
    AdmittedSession, ConnectionIdentifiers, FirstEntryOutcome, SessionContinuity,
};
use super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::character_last_position::{
    LastPositionCell, LastPositionRecord, LastVitals,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::house_transfer::HouseTransferRequest;
use crate::foundation::{
    CharacterLease, LocalPosition, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use oteryn_protocol_oteryn::house_transfer::{self as wire, HouseTransferAck};
#[path = "house_transfer/admission.rs"]
mod admission;
#[path = "house_transfer/movement.rs"]
mod movement;
#[path = "house_transfer/session_views.rs"]
mod session_views;

/// None is an uncertain outcome: close, preserving the durable destination for recovery.
pub(super) type TransferResult = Option<(Option<AdmittedSession>, Vec<u8>)>;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn transfer_house(
        &self,
        admitted: AdmittedSession,
        command: u64,
        body: &[u8],
    ) -> TransferResult {
        let refuse = || Some((None, Vec::new()));
        let Ok(intent) = wire::decode_house_transfer_intent(body) else {
            return refuse();
        };
        let (Some(actor), Some(controller), Some(fence)) = (
            admitted.runtime_actor,
            admitted.controller,
            admitted.item_fence,
        ) else {
            return refuse();
        };
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_HOUSE_TRANSFER_V1)
            || admitted.world_id != self.world_id
            || admitted.channel_id != self.channel_id
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || self
                .states
                .player_trades
                .transitioning(admitted.game_session_id)
            || self.states.newhaven_travel.held(admitted.game_session_id)
        {
            return refuse();
        }
        let house = HouseId::new(self.world_id, &intent.house)?;
        self.evidence
            .refresh_account(
                self.root,
                self.holder,
                &super::canonical_uuid(&controller.account_id),
            )
            .await
            .ok()?;
        // The Channel selection is retained in the House, so exit cannot widen it.
        let capabilities = self
            .required_capabilities
            .authenticate_resume_support(
                admitted.continuity.selected_capabilities.as_slice(),
                admitted.continuity.selected_capabilities,
            )
            .ok()?;
        if capabilities.selected() != admitted.continuity.selected_capabilities {
            return refuse();
        }
        if !self
            .prepare_house_transfer_checkpoints(actor, admitted.game_session_id)
            .await
        {
            return None;
        }
        let _lane = self.spell_lane_permit().await?;
        let mut runtime = self.runtime.lock().await;
        let mut spells = self.states.spell_states.lock().await;
        let mut attack = self.states.attack.lock().await;
        let locks = super::combat_lock_fact::CombatLockFact::read(
            &spells,
            &attack,
            actor,
            admitted.game_session_id,
            self.owner_now(),
        );
        if spells.is_dead(actor)
            || spells.has_pending_spell_commit(actor, admitted.game_session_id)
            || locks.pve_in_fight_until.is_some()
            || locks.pvp == super::combat_lock_fact::PvpLockSource::LiveBlocked
        {
            return refuse();
        }
        let (lease, position, origin) = if intent.exit {
            let Ok(state) = runtime.house_actor_state(actor, admitted.game_session_id) else {
                return refuse();
            };
            if state.house != intent.house
                || state.scope != fence.runtime_scope
                || state.generation != fence.scope_ownership_generation
            {
                return refuse();
            }
            let position = if self.bundle_map().is_some() {
                super::house_query::entrance(&intent.house)?
            } else {
                state.origin_position
            };
            let census = runtime.positioned_actor_census().ok()?;
            let doors = self.door.lock().await;
            let clear = |p: LocalPosition| {
                let cell = crate::foundation::MovementLocalPosition {
                    x: p.x,
                    y: p.y,
                    floor: p.floor,
                };
                super::respawn_cell_admissible(
                    self.movement_cells.index(),
                    self.movement_cells.scope(),
                    self.world_id,
                    runtime.content_pin().server_artifact_digest(),
                    doors.blocking_cells(),
                    cell,
                ) && !census
                    .iter()
                    .any(|(_, snapshot, _)| snapshot.position() == cell)
            };
            let position = [
                position,
                state.origin_position,
                runtime.content_pin().entry_start(),
            ]
            .into_iter()
            .find(|p| clear(*p))?;
            (state.lease, position, position)
        } else {
            if fence.runtime_scope != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
                || fence.scope_ownership_generation != runtime.binding().scope_generation()
            {
                return refuse();
            }
            let Ok(lease) = runtime.bound_attacker_lease(actor, admitted.game_session_id) else {
                return refuse();
            };
            let Ok(current) = runtime.read_actor_position(actor) else {
                return refuse();
            };
            let origin = current.position();
            let Some(position) = self.house_door_destination(&intent.house, origin) else {
                return refuse();
            };
            (
                lease,
                position,
                LocalPosition {
                    x: origin.x,
                    y: origin.y,
                    floor: origin.floor,
                },
            )
        };
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return refuse();
        }
        let destination = SecureIdentifiers.game_session_id()?;
        if !self.house_session_views_ready(admitted.game_session_id, destination) {
            return refuse();
        }
        let Some(views) = spells.prepare_house_transfer_views(
            &runtime,
            &attack,
            actor,
            admitted.game_session_id,
            destination,
        ) else {
            return refuse();
        };
        let transfer = *SecureIdentifiers.game_session_id()?.as_bytes();
        let next_lease =
            CharacterLease::new(lease.character_id(), lease.generation().checked_add(1)?).ok()?;
        let saved = if let Some((_, _, state)) = spells
            .actors
            .iter()
            .find(|(held, session, _)| *held == actor && *session == admitted.game_session_id)
        {
            (
                state
                    .saved_vitals()
                    .and_then(|(health, mana)| LastVitals::new(health, mana)),
                Some(state.bed_rest_snapshot(self.owner_now().get())),
            )
        } else if intent.exit {
            let previous = self
                .root
                .house_transfer_recovery(admitted.game_session_id)
                .await
                .ok()??;
            (
                previous.placement.record.vitals,
                previous.placement.record.rest,
            )
        } else {
            (None, None)
        };
        let pin = runtime.content_pin();
        let request = HouseTransferRequest {
            fence,
            transport: controller.transport,
            command,
            house: house.clone(),
            origin: self.channel_id,
            ack: HouseTransferAck {
                transfer,
                session: *destination.as_bytes(),
                world: *self.world_id.as_bytes(),
                channel: *self.channel_id.as_bytes(),
                house: (!intent.exit).then_some(intent.house.clone()),
                scope_generation: runtime.binding().scope_generation().get(),
                connection_generation: 1,
                lease_generation: next_lease.generation(),
                selection: capabilities.selected().as_slice().to_vec(),
                position: [position.x, position.y, i32::from(position.floor)],
            },
            last_position: LastPositionRecord {
                world_id: self.world_id,
                cell: LastPositionCell {
                    x: origin.x,
                    y: origin.y,
                    floor: origin.floor,
                },
                coordinate_frame_digest: pin.frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: pin.server_artifact_digest(),
                vitals: saved.0,
                rest: saved.1,
            },
        };
        let saved_position = request.last_position;
        let transition = if intent.exit {
            None
        } else {
            let id = runtime.mint_transition_fence().ok()?;
            let token = crate::foundation::ChannelRuntimeV1::transition_fence(id);
            runtime
                .fence_player_writes(actor, admitted.game_session_id, token)
                .ok()?;
            Some(token)
        };
        let committed = self
            .root
            .commit_house_transfer(
                self.character,
                self.holder,
                request,
                capabilities.current_check(),
            )
            .await;
        let (ack, payload) = match committed {
            Ok(committed) => committed,
            Err(_) => {
                let (source, _) = FreshAdmissionStore::from_root(self.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await
                    .ok()?;
                if source.session_state() == crate::foundation::GameSessionState::Active
                    && super::item_fence_of(source) == Some(fence)
                    && source.current_transport() == Some(controller.transport)
                {
                    if let Some(token) = transition {
                        runtime
                            .lift_player_fence(actor, admitted.game_session_id, token)
                            .ok()?;
                    }
                    return refuse();
                }
                return None;
            }
        };
        // The native handoff retains the authenticated source selection. A new
        // global offer calculation must not silently remove a committed grant.
        if ack.selection.as_slice() != capabilities.selected().as_slice() {
            return None;
        }
        if let Some(token) = transition {
            runtime
                .lift_player_fence(actor, admitted.game_session_id, token)
                .ok()?;
        }
        runtime
            .commit_house_custody(
                actor,
                admitted.game_session_id,
                destination,
                next_lease,
                &intent.house,
                house.runtime_scope().ok()?,
                if intent.exit {
                    fence.scope_ownership_generation
                } else {
                    ScopeOwnershipGeneration::new(ack.scope_generation).ok()?
                },
                position,
                intent.exit,
            )
            .ok()?;
        // Publish the existing actor data only after its native custody commit.
        // The prepared token verifies the real successor lease and new scope;
        // remaining clocks and pending authority are never reset or retargeted.
        spells.commit_house_transfer_views(&runtime, &mut attack, views)?;
        drop(attack);
        drop(spells);
        drop(runtime);
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(destination)
            .await
            .ok()?;
        let mut checkpoint = saved_position;
        if !intent.exit {
            checkpoint.cell = LastPositionCell {
                x: ack.position[0],
                y: ack.position[1],
                floor: ack.position[2].try_into().ok()?,
            };
        }
        self.track_transferred_position(
            admitted.game_session_id,
            destination,
            actor,
            &checkpoint,
            fence.character_id,
        )
        .await?;
        let mut next = admitted;
        next.game_session_id = destination;
        next.first_entry = FirstEntryOutcome::Positioned;
        next.item_fence = super::item_fence_of(current);
        next.continuity = SessionContinuity {
            selected_capabilities: capabilities.selected(),
            ..SessionContinuity::FRESH
        };
        // These are actor rate observations. The actual destination still starts
        // its own command namespace and publication revisions.
        next.continuity.combat.target_intents = admitted.continuity.combat.target_intents;
        next.continuity.combat.mode_intents = admitted.continuity.combat.mode_intents;
        self.carry_house_session_views(&admitted, &next)
            .await
            .ok()?;
        if intent.exit {
            self.initialize_transferred_channel(&next, &saved_position)
                .await?;
        }
        if !intent.exit {
            super::hireling::recall_lost_houses(self).await?;
            super::hireling::refresh(
                self,
                actor,
                super::connection::UseCommand {
                    game_session_id: next.game_session_id,
                    command_id: command,
                    item_fence: next.item_fence,
                },
            )
            .await?;
        }
        Some((Some(next), payload))
    }

    fn house_door_destination(
        &self,
        house: &str,
        origin: crate::foundation::MovementLocalPosition,
    ) -> Option<LocalPosition> {
        if self.bundle_map().is_some() {
            let entry = super::house_query::entrance(house)?;
            if (origin.x, origin.y, origin.floor) != (entry.x, entry.y, entry.floor) {
                return None;
            }
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let position = LocalPosition {
                x: origin.x.checked_add(dx)?,
                y: origin.y.checked_add(dy)?,
                floor: origin.floor,
            };
            if let Some(map) = self.bundle_map() {
                let (Ok(x), Ok(y), Ok(floor)) = (
                    u16::try_from(position.x),
                    u16::try_from(position.y),
                    i8::try_from(position.floor),
                ) else {
                    continue;
                };
                let tile = crate::map::overlay::TilePos {
                    x,
                    y,
                    floor: floor.checked_neg()?,
                };
                if map
                    .facts
                    .house_door(tile, super::house_query::source_house_id(house)?)
                    .is_some()
                {
                    return Some(position);
                }
            } else {
                let cell = crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                };
                let Some(placement) = self
                    .movement_cells
                    .house_tiles()
                    .house_at(self.movement_cells.scope(), cell)
                    .ok()?
                else {
                    continue;
                };
                let entry = placement.entry();
                if placement.house().key == house
                    && placement.door_at(cell).is_some()
                    && (entry.x, entry.y, entry.z) == (origin.x, origin.y, i32::from(origin.floor))
                {
                    return Some(position);
                }
            }
        }
        None
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn project_house_transfer(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<(u64, Vec<u8>)>, ()> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_HOUSE_TRANSFER_V1)
        {
            return Ok(None);
        }
        let Some(mut receipt) = self
            .root
            .house_transfer_recovery(admitted.game_session_id)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        let fence = admitted.item_fence.ok_or(())?;
        let controller = admitted.controller.ok_or(())?;
        let (session, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| ())?;
        if session.session_state() != crate::foundation::GameSessionState::Active
            || session.current_transport() != Some(controller.transport)
            || super::item_fence_of(session) != Some(fence)
            || receipt.ack.session != *admitted.game_session_id.as_bytes()
            || receipt.ack.world != *admitted.world_id.as_bytes()
            || receipt.ack.channel != *admitted.channel_id.as_bytes()
            || receipt.ack.scope_generation != fence.scope_ownership_generation.get()
            || receipt.ack.lease_generation != fence.character_lease_generation
            || receipt.ack.selection != admitted.continuity.selected_capabilities.as_slice()
            || (receipt.command == 0) != receipt.admission_restore
            || (receipt.admission_restore && receipt.ack.house.is_none())
        {
            return Err(());
        }
        let runtime = self.runtime.lock().await;
        let actor = admitted.runtime_actor.ok_or(())?;
        if let Some(house) = &receipt.ack.house {
            let state = runtime
                .house_actor_state(actor, admitted.game_session_id)
                .map_err(|_| ())?;
            if &state.house != house
                || state.scope != fence.runtime_scope
                || state.generation != fence.scope_ownership_generation
                || state.lease.generation() != fence.character_lease_generation
            {
                return Err(());
            }
        } else {
            let lease = runtime
                .bound_attacker_lease(actor, admitted.game_session_id)
                .map_err(|_| ())?;
            if fence.runtime_scope != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
                || lease.generation() != fence.character_lease_generation
            {
                return Err(());
            }
        }
        receipt.ack.connection_generation = admitted.continuity.connection_generation;
        Ok(Some((
            receipt.command,
            wire::encode_house_transfer_ack(&receipt.ack).map_err(|_| ())?,
        )))
    }

    pub(super) async fn recover_transferred_house(
        &self,
        budget: &super::AdmissionRetryBudget<'_>,
        attempt: super::connection::ResumeAttempt<'_>,
    ) -> Result<Option<AdmittedSession>, super::connection::AdmissionRefusal> {
        use super::connection::{AdmissionRefusal::*, ControllerBinding};
        use crate::durability::recovery_evidence_composition::RecoveryEvidenceSubject;
        use crate::foundation::fnd04_verifier::fresh_grant_signing_key_id;
        // Every resume reads its receipt first, before anything is written: a busy holder
        // retries under the resume's budget instead of closing the resume (NODEBOOT-1).
        let Some(candidate) = budget
            .retry_unavailable(async || {
                self.root
                    .house_transfer_recovery(attempt.game_session_id)
                    .await
                    .map_err(|_| Unavailable)
            })
            .await?
        else {
            return Ok(None);
        };
        if candidate.started {
            // Ordinary reconnect owns sessions that have already used their
            // destination command stream; lost-ack recovery cannot reset it.
            return Ok(None);
        }
        if candidate.ack.world != *self.world_id.as_bytes()
            || candidate.ack.channel != *self.channel_id.as_bytes()
        {
            return Err(Rejected);
        }
        // A genuine retained control loss belongs to CompleteReconnect even
        // before the destination has received its first command. Recovering its
        // handoff ACK cannot clear that loss or create reconnect protection.
        let retained_destination = crate::foundation::GameSessionId::decode(&candidate.ack.session)
            .map_err(|_| Unavailable)?;
        let token = std::str::from_utf8(attempt.recovery_material).map_err(|_| Rejected)?;
        let key = fresh_grant_signing_key_id(token).ok_or(Rejected)?;
        self.evidence
            .refresh_recovery(self.root, self.holder, &candidate.current.account_id, &key)
            .await
            .map_err(|_| Unavailable)?;
        let subject = RecoveryEvidenceSubject::new(&candidate.current.account_id, &key)
            .map_err(|_| Rejected)?;
        let verified = self
            .root
            .verify_registered_recovery(self.holder, &subject, token, &candidate.current)
            .await
            .map_err(|_| Unavailable)?
            .map_err(|_| Rejected)?;
        // The authenticated receipt retains the source's actual scope-bound
        // selection, including privately composed producers. Revalidate it
        // against that boot policy, not the global fresh advertisement, and
        // reject any filtering instead of silently changing the grant.
        let selected = self
            .required_capabilities
            .authenticate_support(&candidate.ack.selection)
            .map_err(|_| RecoveryCapabilityRequired)?
            .selected();
        if selected.as_slice() != candidate.ack.selection.as_slice() {
            return Err(RecoveryCapabilityRequired);
        }
        if !selected.resumable_with(attempt.supported_capabilities) {
            return Err(RecoveryCapabilityRequired);
        }
        let capabilities = self
            .required_capabilities
            .authenticate_resume_support(attempt.supported_capabilities, selected)
            .map_err(|_| RecoveryCapabilityRequired)?;
        if capabilities.selected() != selected {
            return Err(RecoveryCapabilityRequired);
        }
        self.reserve_precondition().await?;
        let _lane = self.spell_lane_permit().await.ok_or(Unavailable)?;
        let mut runtime = self.runtime.lock().await;
        let loss = if let Some((actor, _)) = runtime
            .house_player_for_session(retained_destination)
            .map_err(|_| Unavailable)?
        {
            runtime
                .house_player_control_facts(actor, retained_destination)
                .map_err(|_| Unavailable)?
                .control_loss
        } else if let Some((actor, _)) = runtime
            .positioned_player_for_session(retained_destination)
            .map_err(|_| Unavailable)?
        {
            runtime
                .player_control_facts(actor, retained_destination)
                .map_err(|_| Unavailable)?
                .control_loss
        } else {
            None
        };
        if loss.is_some() {
            return Ok(None);
        }
        let recovered = self
            .root
            .recover_house_transfer(
                self.character,
                self.holder,
                candidate,
                verified,
                attempt.transport,
                capabilities.current_check(),
            )
            .await
            .map_err(|_| Unavailable)?;
        let destination = crate::foundation::GameSessionId::decode(&recovered.ack.session)
            .map_err(|_| Unavailable)?;
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(destination)
            .await
            .map_err(|_| Unavailable)?;
        let fence = super::item_fence_of(current).ok_or(Unavailable)?;
        let lease = CharacterLease::new(
            recovered.current.character_id,
            recovered.ack.lease_generation,
        )
        .map_err(|_| Unavailable)?;
        let mut position = LocalPosition {
            x: recovered.ack.position[0],
            y: recovered.ack.position[1],
            floor: recovered.ack.position[2]
                .try_into()
                .map_err(|_| Unavailable)?,
        };
        let stored = recovered.placement;
        if stored.record.coordinate_frame_digest != runtime.content_pin().frame_binding_digest()
            || stored.record.server_artifact_digest
                != runtime.content_pin().server_artifact_digest()
            || stored.record.map_revision_digest != runtime.map_revision_digest()
        {
            return Err(Rejected);
        }
        let mut origin = LocalPosition {
            x: stored.record.cell.x,
            y: stored.record.cell.y,
            floor: stored.record.cell.floor,
        };
        if let Some(metadata) = &recovered.house_position {
            if recovered.ack.house.as_deref() != Some(metadata.house.house_key())
                || metadata.origin_channel != self.channel_id
                || metadata.house.world_id() != self.world_id
            {
                return Err(Rejected);
            }
            position = LocalPosition {
                x: stored.record.cell.x,
                y: stored.record.cell.y,
                floor: stored.record.cell.floor,
            };
            origin = LocalPosition {
                x: metadata.origin.x,
                y: metadata.origin.y,
                floor: metadata.origin.floor,
            };
        } else if recovered.ack.house.is_some() && stored.key.write_sequence != 1 {
            // Legacy entry-only checkpoint encoded the Channel origin; no other
            // unscoped row can stand in for a current named-House position.
            return Err(Rejected);
        }
        let house = recovered
            .ack
            .house
            .as_ref()
            .map(|key| {
                let house = HouseId::new(self.world_id, key).ok_or(Rejected)?;
                Ok::<_, super::connection::AdmissionRefusal>(
                    oteryn_gameplay_entities::carrier::house_custody::HouseActorState {
                        house: key.clone(),
                        scope: house.runtime_scope().map_err(|_| Unavailable)?,
                        generation: ScopeOwnershipGeneration::new(recovered.ack.scope_generation)
                            .map_err(|_| Unavailable)?,
                        position,
                        lease,
                        origin_position: origin,
                    },
                )
            })
            .transpose()?;
        if let Some(state) = &house {
            let doors = self.door.lock().await.blocking_cells().clone();
            let existing = runtime
                .house_player_for_session(destination)
                .map_err(|_| Unavailable)?;
            let except = existing.as_ref().map(|(actor, _)| *actor);
            let mut placed = stored.record;
            placed.cell = LastPositionCell {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
            let mut check = movement::PlacementCheck {
                owner: self,
                runtime: &runtime,
                candidate: state,
                record: &placed,
                doors: &doors,
                except,
                retain_hireling_overlap: existing
                    .as_ref()
                    .is_some_and(|(_, held)| held.position == position),
                valid: false,
            };
            let request = crate::durability::house_interior::HouseStepReadRequest {
                fence,
                origin_channel: *self.channel_id.as_bytes(),
                house: HouseId::new(self.world_id, &state.house).ok_or(Rejected)?,
                content_digest: runtime.content_pin().server_artifact_digest(),
            };
            self.root
                .with_house_step_read(
                    self.character,
                    self.holder,
                    request,
                    &mut check,
                    |read, check| {
                        check.valid = check.validates(
                            crate::durability::house_transfer::HouseAdmissionRead {
                                interior: read.interior,
                                hirelings: read.hirelings,
                            },
                        );
                        false
                    },
                )
                .await
                .map_err(|_| Unavailable)?;
            if !check.valid {
                return Err(Rejected);
            }
        }
        let mut spells = self.states.spell_states.lock().await;
        let mut attack = self.states.attack.lock().await;
        let views = spells
            .prepare_recovered_house_transfer_views(&runtime, &attack, &recovered)
            .ok_or(Unavailable)?;
        let retained_fresh = if recovered.admission_restore {
            let state = house.clone().ok_or(Unavailable)?;
            let source_lease = CharacterLease::new(
                recovered.current.character_id,
                recovered
                    .ack
                    .lease_generation
                    .checked_sub(1)
                    .ok_or(Unavailable)?,
            )
            .map_err(|_| Unavailable)?;
            runtime
                .restore_retained_house_admission(
                    recovered.source,
                    destination,
                    source_lease,
                    state,
                )
                .map_err(|_| Unavailable)?
        } else {
            None
        };
        let actor = match retained_fresh {
            Some(actor) => actor,
            None => runtime
                .restore_transfer_actor(recovered.source, destination, lease, position, house)
                .map_err(|_| Unavailable)?,
        };
        if let Some(views) = views {
            spells
                .commit_house_transfer_views(&runtime, &mut attack, views)
                .ok_or(Unavailable)?;
        }
        drop(attack);
        drop(spells);
        drop(runtime);
        let account = recovered.account;
        let mut admitted = AdmittedSession {
            game_session_id: destination,
            world_id: self.world_id,
            channel_id: self.channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Reconciled,
            controller: Some(ControllerBinding {
                transport: attempt.transport,
                account_id: account,
            }),
            continuity: SessionContinuity {
                connection_generation: recovered.ack.connection_generation,
                selected_capabilities: selected,
                ..SessionContinuity::FRESH
            },
            item_fence: Some(fence),
        };
        if recovered.ack.house.is_none() {
            self.carry_recovered_house_session_views(recovered.source, &admitted)
                .await?;
            self.initialize_transferred_channel(&admitted, &recovered.placement.record)
                .await
                .ok_or(Unavailable)?;
            let has_progression = self
                .states
                .progression_sessions
                .lock()
                .map_err(|_| Unavailable)?
                .contains_key(&destination);
            let has_quests = self
                .states
                .quest_sessions
                .lock()
                .map_err(|_| Unavailable)?
                .contains_key(&destination);
            if !has_progression {
                self.admit_progression_session(&mut admitted).await;
            }
            if !has_quests {
                self.admit_quest_session(&admitted).await;
            }
        }
        if recovered.ack.house.is_some() {
            self.admit_premium(account, admitted.game_session_id);
            super::item_use::gourmet::restore_house(self, &admitted, &stored.record)
                .await
                .ok_or(Unavailable)?;
            self.restore_house_admission_views(&admitted, Some(recovered.source))
                .await?;
            super::hireling::recall_lost_houses(self)
                .await
                .ok_or(Unavailable)?;
            super::hireling::refresh(
                self,
                actor,
                super::connection::UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: admitted.continuity.next_command_id,
                    item_fence: admitted.item_fence,
                },
            )
            .await
            .ok_or(Unavailable)?;
        }
        let mut checkpoint = stored.record;
        checkpoint.cell = LastPositionCell {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        self.track_transferred_position(
            recovered.source,
            destination,
            actor,
            &checkpoint,
            fence.character_id,
        )
        .await
        .ok_or(Unavailable)?;
        self.states.command_ingress.admit(destination);
        Ok(Some(admitted))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn initialize_transferred_channel(
        &self,
        admitted: &AdmittedSession,
        saved: &LastPositionRecord,
    ) -> Option<()> {
        let actor = admitted.runtime_actor?;
        {
            let runtime = self.runtime.lock().await;
            if self
                .states
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, admitted.game_session_id)
                .is_some()
            {
                return Some(());
            }
        }
        let character = admitted.item_fence?.character_id;
        let (facts, revision) = match super::spell_character_facts::load_character_cast_facts(
            self.root,
            self.character,
            character,
        )
        .await
        {
            super::spell_character_facts::CastFactsLoad::Ready {
                facts,
                character_revision,
            }
            | super::spell_character_facts::CastFactsLoad::NoVocation {
                facts,
                character_revision,
            } => (facts, character_revision),
            super::spell_character_facts::CastFactsLoad::Unavailable => return None,
        };
        let monk = self.load_monk_state(character).await?;
        let build = self
            .root
            .read_character_build_state(self.character, character)
            .await
            .ok()?;
        let stamina = self
            .root
            .read_hunting_stamina(self.character, character)
            .await
            .ok()?;
        let gourmet = self
            .root
            .read_gourmet_conditions(self.character, character)
            .await
            .ok()?;
        let stance = self
            .root
            .read_character_stance(self.character, character)
            .await
            .ok()?;
        let effects = self
            .read_admission_effects(admitted.game_session_id)
            .await
            .ok()?;
        let latest = self
            .root
            .read_current_character(self.character, character)
            .await
            .ok()?;
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .ok()?;
        if latest.revision != revision
            || current.current_transport() != Some(admitted.controller?.transport)
            || super::item_fence_of(current) != admitted.item_fence
        {
            return None;
        }
        self.reserve_precondition().await.ok()?;
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let gourmet_created = !states
            .actors
            .iter()
            .any(|(held, session, _)| *held == actor && *session == admitted.game_session_id);
        states.initialize_admitted(
            &runtime,
            actor,
            admitted.game_session_id,
            facts,
            monk,
            self.owner_now(),
            saved.vitals.map(|v| (v.health(), v.mana())),
            saved.rest,
        )?;
        if gourmet_created {
            let state = states.get_mut(&runtime, actor, admitted.game_session_id)?;
            crate::spell::actor_conditions::restore_gourmet_food(
                state,
                gourmet,
                self.owner_now().get() / 1000,
            )
            .ok()?;
            super::item_use::gourmet::remember(
                &mut states,
                actor,
                admitted.game_session_id,
                gourmet,
                self.owner_now().get(),
            );
        }
        if !states.load_owned_stance(
            &runtime,
            actor,
            admitted.game_session_id,
            &stance,
            self.spells,
        ) || !states.load_owned_build(&runtime, actor, admitted.game_session_id, &build)
        {
            return None;
        }
        if let Some(formula) = self
            .active_generation
            .and_then(|a| a.native_gameplay())
            .and_then(|n| n.training_formula())
        {
            states
                .get_mut(&runtime, actor, admitted.game_session_id)?
                .enable_owned_training(&build, formula)
                .ok()?;
        }
        states.load_stamina(&runtime, actor, admitted.game_session_id, stamina);
        drop(states);
        self.install_admission_suppressions(&runtime, actor, admitted.game_session_id, effects)
            .await?;
        Some(())
    }
}
