//! Q16: fresh authentication restores a saved House through the original native
//! handoff. The resulting acknowledgement is a new immutable admission result;
//! its separate current-position projection continues through native movement.
use super::movement::PlacementCheck;
use super::*;
use crate::durability::character_last_position::ScopedLastPosition;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::{ExactActorRef, GameSessionState};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, ControllerBinding, FreshAdmissionAttempt,
};
use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;
#[path = "admission/source.rs"]
mod source;
#[path = "admission/source_progression.rs"]
mod source_progression;

struct RestoreCheck<'a, 'r, 'c, 's> {
    owner: &'a ComposedFreshAdmission<'r, 'c, 's>,
    runtime: &'a crate::foundation::ChannelRuntimeV1,
    candidate: HouseActorState,
    saved: &'a ScopedLastPosition,
    doors: &'a std::collections::BTreeSet<crate::content::LogicalCell>,
    actor: ExactActorRef,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called after genuine fresh COMMIT and before its Channel first-entry
    /// participants can publish or overwrite a recorded House position.
    pub(in crate::gameplay_transport) async fn restore_fresh_house(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
        actor: ExactActorRef,
        account: [u8; 16],
        request: &crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1,
    ) -> Result<Option<AdmittedSession>, AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (source, _) = store
            .current_session_at(attempt.game_session_id)
            .await
            .map_err(|_| Unavailable)?;
        let fence = crate::gameplay_transport::item_fence_of(source).ok_or(Unavailable)?;
        if source.session_state() != GameSessionState::Active
            || source.current_transport() != Some(attempt.transport)
            || fence.runtime_scope != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
            || fence.character_id.as_bytes() != attempt.character_id.as_bytes()
        {
            return Err(Rejected);
        }
        let character = self
            .root
            .read_current_character(self.character, fence.character_id)
            .await
            .map_err(|_| Unavailable)?;
        let gameplay = CurrentCharacterGameplayFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
            expected_character_revision: character.revision,
        };
        let Some(saved) = self
            .root
            .read_scoped_character_last_position(self.character, self.holder, gameplay)
            .await
            .map_err(|_| Unavailable)?
        else {
            return Ok(None);
        };
        let Some(history) = saved.house.as_ref() else {
            return Ok(None);
        };
        if history.origin_channel != self.channel_id || history.house.world_id() != self.world_id {
            return Err(Rejected);
        }
        let capabilities = self
            .required_capabilities
            .authenticate_support(attempt.supported_capabilities)
            .map_err(|_| AdmissionRefusal::RecoveryCapabilityRequired)?;
        let selected = crate::gameplay_transport::connection::fresh_capability_selection(
            self,
            attempt.supported_capabilities,
        )
        .ok_or(AdmissionRefusal::RecoveryCapabilityRequired)?;
        if !selected.contains(wire::CAPABILITY_HOUSE_TRANSFER_V1) {
            return Err(AdmissionRefusal::RecoveryCapabilityRequired);
        }
        // This identity really is the newly authenticated, committed Channel
        // source. It remains unpositioned and never accepts external commands.
        // Every Character initialization/settlement precedes its native handoff.
        let pending_source = AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::RefusedUnavailable,
            controller: Some(ControllerBinding {
                transport: attempt.transport,
                account_id: account,
            }),
            continuity: SessionContinuity {
                selected_capabilities: selected,
                ..SessionContinuity::FRESH
            },
            item_fence: Some(fence),
        };
        let Some((saved, prepared)) = self.prepare_house_source(pending_source, request).await?
        else {
            // The existing source owner selected respawn/arrival/tutorial or a
            // newer non-House record. Continue through ordinary first entry.
            return Ok(None);
        };
        let history = saved.house.as_ref().ok_or(Unavailable)?;
        if history.origin_channel != self.channel_id || history.house.world_id() != self.world_id {
            return Err(Rejected);
        }
        if !self.ensure_source_map_initialized().await {
            return Err(Unavailable);
        }
        let _lane = self.spell_lane_permit().await.ok_or(Unavailable)?;
        self.reserve_precondition().await?;
        let mut runtime = self.runtime.lock().await;
        let destination = SecureIdentifiers.game_session_id().ok_or(Unavailable)?;
        let transfer = *SecureIdentifiers
            .game_session_id()
            .ok_or(Unavailable)?
            .as_bytes();
        let lease = CharacterLease::new(
            attempt.character_id,
            fence
                .character_lease_generation
                .checked_add(1)
                .ok_or(Unavailable)?,
        )
        .map_err(|_| Unavailable)?;
        let position = LocalPosition {
            x: saved.position.record.cell.x,
            y: saved.position.record.cell.y,
            floor: saved.position.record.cell.floor,
        };
        let origin = LocalPosition {
            x: history.origin.x,
            y: history.origin.y,
            floor: history.origin.floor,
        };
        let mut original = saved.position.record;
        original.cell = history.origin;
        let request = HouseTransferRequest {
            fence,
            transport: attempt.transport,
            command: 0,
            house: history.house.clone(),
            origin: self.channel_id,
            ack: HouseTransferAck {
                transfer,
                session: *destination.as_bytes(),
                world: *self.world_id.as_bytes(),
                channel: *self.channel_id.as_bytes(),
                house: Some(history.house.house_key().into()),
                scope_generation: 1,
                connection_generation: 1,
                lease_generation: lease.generation(),
                selection: selected.as_slice().to_vec(),
                position: [position.x, position.y, i32::from(position.floor)],
            },
            last_position: original,
        };
        let candidate = HouseActorState {
            house: history.house.house_key().into(),
            scope: history.house.runtime_scope().map_err(|_| Unavailable)?,
            generation: ScopeOwnershipGeneration::new(1).map_err(|_| Unavailable)?,
            position,
            lease,
            origin_position: origin,
        };
        let doors = self.door.lock().await.blocking_cells().clone();
        let mut check = RestoreCheck {
            owner: self,
            runtime: &runtime,
            candidate,
            saved: &saved,
            doors: &doors,
            actor,
        };
        let (ack, _) = self
            .root
            .commit_saved_house_admission(
                self.character,
                self.holder,
                request,
                saved.clone(),
                capabilities.current_check(),
                &mut check,
                |read, check| {
                    let Ok(generation) =
                        ScopeOwnershipGeneration::new(read.interior.ownership_generation)
                    else {
                        return false;
                    };
                    check.candidate.generation = generation;
                    PlacementCheck {
                        owner: check.owner,
                        runtime: check.runtime,
                        candidate: &check.candidate,
                        record: &check.saved.position.record,
                        doors: check.doors,
                        except: Some(check.actor),
                        retain_hireling_overlap: false,
                        valid: false,
                    }
                    .validates(read)
                },
            )
            .await
            .map_err(|_| Unavailable)?;
        let mut house = check.candidate;
        house.generation =
            ScopeOwnershipGeneration::new(ack.scope_generation).map_err(|_| Unavailable)?;
        runtime
            .restore_admitted_house_actor(
                actor,
                attempt.game_session_id,
                destination,
                source.current_character_lease(),
                house,
            )
            .map_err(|_| Unavailable)?;
        drop(runtime);
        let (current, _) = store
            .current_session_at(destination)
            .await
            .map_err(|_| Unavailable)?;
        let admitted = AdmittedSession {
            game_session_id: destination,
            world_id: self.world_id,
            channel_id: self.channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: Some(ControllerBinding {
                transport: attempt.transport,
                account_id: account,
            }),
            continuity: SessionContinuity {
                selected_capabilities: selected,
                ..SessionContinuity::FRESH
            },
            item_fence: crate::gameplay_transport::item_fence_of(current),
        };
        crate::gameplay_transport::item_use::gourmet::restore_house(
            self,
            &admitted,
            &saved.position.record,
        )
        .await
        .ok_or(Unavailable)?;
        self.install_house_source_views(&admitted, prepared).await?;
        self.track_transferred_position(
            attempt.game_session_id,
            destination,
            actor,
            &saved.position.record,
            fence.character_id,
        )
        .await
        .ok_or(Unavailable)?;
        crate::gameplay_transport::hireling::recall_lost_houses(self)
            .await
            .ok_or(Unavailable)?;
        crate::gameplay_transport::hireling::refresh(
            self,
            actor,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: destination,
                command_id: admitted.continuity.next_command_id,
                item_fence: admitted.item_fence,
            },
        )
        .await
        .ok_or(Unavailable)?;
        self.states.command_ingress.admit(destination);
        Ok(Some(admitted))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// An independently current read under the original admission relation locks
    /// serializes behind a possible handoff COMMIT. Only an untouched ACTIVE
    /// source is a known rollback: terminal/changed/unknown sources remain on
    /// the existing immutable destination-receipt recovery path.
    pub(in crate::gameplay_transport) async fn refused_fresh_house_source(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
        actor: ExactActorRef,
        account: [u8; 16],
    ) -> Option<AdmittedSession> {
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(attempt.game_session_id)
            .await
            .ok()?;
        let fence = crate::gameplay_transport::item_fence_of(current)?;
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(attempt.transport)
            || current.current_control_loss_epoch().is_some()
            || fence.game_session_id != attempt.game_session_id
            || fence.connection_generation.get() != 1
            || fence.character_id.as_bytes() != attempt.character_id.as_bytes()
            || fence.runtime_scope != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
            || current.commit().game_session_id() != attempt.game_session_id
            || current.commit().character_lease_generation() != fence.character_lease_generation
        {
            return None;
        }
        let _lane = self.spell_lane_permit().await?;
        self.reserve_precondition().await.ok()?;
        // This also proves the exact committed slot/session. It adds only the
        // authenticated source lease, never a fallback position or House grant.
        self.runtime
            .lock()
            .await
            .bind_attacker_lease(
                actor,
                attempt.game_session_id,
                current.current_character_lease(),
            )
            .ok()?;
        Some(AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::RefusedUnavailable,
            controller: Some(ControllerBinding {
                transport: attempt.transport,
                account_id: account,
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: Some(fence),
        })
    }
}
