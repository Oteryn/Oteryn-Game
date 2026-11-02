//! Same-node House transfers on the composed owner and the existing native carrier.
use super::connection::{
    AdmittedSession, ConnectionIdentifiers, FirstEntryOutcome, FreshAdmissionAuthority,
    SessionContinuity,
};
use super::{ComposedFreshAdmission, SecureIdentifiers, capabilities::SelectedCapabilities};
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

/// None is an uncertain outcome: close, preserving the durable destination for recovery.
pub(super) type TransferResult = Option<(Option<AdmittedSession>, Vec<u8>)>;

/// A new command stream does not make the same physical player active again
/// or replay its already delivered server messages.
pub(super) fn handoff_continuity(
    previous: SessionContinuity,
    selected_capabilities: SelectedCapabilities,
    connection_generation: u64,
) -> SessionContinuity {
    SessionContinuity {
        gameplay_idle: previous.gameplay_idle,
        server_message_revision: previous.server_message_revision,
        server_motd_sent: previous.server_motd_sent,
        selected_capabilities,
        connection_generation,
        ..SessionContinuity::FRESH
    }
}

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
        let _lane = self.spell_lane_permit().await?;
        let mut runtime = self.runtime.lock().await;
        let mut spells = self.states.spell_states.lock().await;
        let attack = self.states.attack.lock().await;
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
        // These are the same live vitals and cooldowns, under the same exclusive owner guards.
        for (held, session, _) in &mut spells.actors {
            if *held == actor && *session == admitted.game_session_id {
                *session = destination;
            }
        }
        drop(attack);
        drop(spells);
        drop(runtime);
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(destination)
            .await
            .ok()?;
        let mut next = admitted;
        next.game_session_id = destination;
        next.first_entry = FirstEntryOutcome::Positioned;
        next.item_fence = super::item_fence_of(current);
        next.continuity = handoff_continuity(
            admitted.continuity,
            SelectedCapabilities::select(self.offered_capabilities(), &ack.selection)?,
            ack.connection_generation,
        );
        if intent.exit {
            self.initialize_transferred_channel(&next, &saved_position)
                .await?;
            self.admit_progression_session(&mut next).await;
            self.admit_quest_session(&next).await;
        }
        self.states.gameplay_idle_presence.observe(next);
        self.states
            .gameplay_idle_presence
            .forget(admitted.game_session_id);
        // The physical connection's original guard owns this recipient across
        // handoffs, including cancellation before its result reaches the wire.
        self.states
            .server_messages
            .transfer_connection(admitted, next)
            .ok()?;
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
        let selected =
            SelectedCapabilities::select(self.offered_capabilities(), &candidate.ack.selection)
                .ok_or(Rejected)?;
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
        let position = LocalPosition {
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
        let origin = LocalPosition {
            x: stored.record.cell.x,
            y: stored.record.cell.y,
            floor: stored.record.cell.floor,
        };
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
        let actor = runtime
            .restore_transfer_actor(recovered.source, destination, lease, position, house)
            .map_err(|_| Unavailable)?;
        drop(runtime);
        let retained_idle = self
            .states
            .gameplay_idle_presence
            .retained_idle(destination, actor)
            .or_else(|| {
                self.states
                    .gameplay_idle_presence
                    .retained_idle(recovered.source, actor)
            });
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
        if let Some(idle) = retained_idle {
            admitted.continuity.gameplay_idle = idle;
        }
        if recovered.ack.house.is_none() {
            self.initialize_transferred_channel(&admitted, &recovered.placement.record)
                .await
                .ok_or(Unavailable)?;
            self.admit_progression_session(&mut admitted).await;
            self.admit_quest_session(&admitted).await;
        }
        self.states.command_ingress.admit(destination);
        self.states.gameplay_idle_presence.observe(admitted);
        self.states.gameplay_idle_presence.forget(recovered.source);
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
