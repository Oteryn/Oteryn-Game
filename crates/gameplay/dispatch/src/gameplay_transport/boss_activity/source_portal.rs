//! Actual source STEP composes the existing scope-transfer owner. No wire lever.
use super::super::native_activity::boss_room_entry_native_ready;
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};
const BOSS: &str = "oteryn:boss.rakesh_moonfang";
const TEMPLATE: &str = "oteryn:room.rakesh_moonfang_source_portal";
const REVISION: &str =
    "source-portal-b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105";
impl ComposedFreshAdmission<'_, '_, '_> {
    /// The source movement owner captured this exact committed STEP. A wire
    /// pacing claim must finish before the existing native transfer can freeze it.
    pub(in crate::gameplay_transport) async fn committed_source_portal_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        snapshot: MovementPositionSnapshot,
    ) -> Result<(), ()> {
        let at = snapshot.position();
        if !matches!(
            (at.x, at.y, at.floor),
            (33039, 32964, 8) | (33043, 32986, -8)
        ) {
            return Ok(());
        }
        let admitted = self.activity_runtimes.connection(session).ok_or(())?;
        if admitted.runtime_actor != Some(actor) || admitted.item_fence.is_none() {
            return Err(());
        }
        if self
            .states
            .attack
            .lock()
            .await
            .activity_step_in_flight(actor)
        {
            self.activity_runtimes
                .defer_source_portal_step(admitted, snapshot)?;
            return Ok(());
        }
        let result = if at.floor == 8 {
            self.rakesh_source_step(actor, session, snapshot).await
        } else {
            self.rakesh_source_return_step(actor, session, snapshot)
                .await
        };
        if result.is_ok() {
            self.activity_runtimes
                .finish_source_portal_step(admitted, snapshot)?;
        }
        result
    }

    /// Existing pending-switch polling runs after the real pacing-clock record.
    /// Reuse the captured occurrence; never synthesize a new command or STEP.
    pub(in crate::gameplay_transport) async fn settle_source_portal_step(
        &self,
        admitted: super::super::connection::AdmittedSession,
    ) -> Result<(), ()> {
        let Some(snapshot) = self.activity_runtimes.source_portal_step(admitted)? else {
            return Ok(());
        };
        let actor = admitted.runtime_actor.ok_or(())?;
        Box::pin(self.committed_source_portal_step(actor, admitted.game_session_id, snapshot)).await
    }

    /// Called only after committing a real ordinary STEP and releasing its lane.
    pub(in crate::gameplay_transport) async fn rakesh_source_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        snapshot: MovementPositionSnapshot,
    ) -> Result<(), ()> {
        let at = snapshot.position();
        if (at.x, at.y, at.floor) != (33039, 32964, 8) {
            return Ok(());
        }
        super::super::source_portal_hold::OriginPortalBinding::qualify_complete(
            self.movement_cells,
        )
        .ok_or_else(|| super::super::operator_event("summer_source_origin_binding_refused"))?;
        let admitted = self
            .activity_runtimes
            .connection(session)
            .ok_or_else(|| super::super::operator_event("summer_source_connection_missing"))?;
        let fence = admitted
            .item_fence
            .ok_or_else(|| super::super::operator_event("summer_source_item_fence_missing"))?;
        if admitted.runtime_actor != Some(actor)
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(());
        }
        // A movement revision, scoped by the exact session and source kind, is
        // a server occurrence. It is never a fabricated client CommandRef.
        let occurrence = snapshot.revision();
        if let Some(outcome) = self
            .retry_native_activity(admitted, occurrence, BOSS)
            .await
            .map_err(|_| super::super::operator_event("summer_source_retained_attempt_pending"))?
        {
            return self.finish_source_portal_outcome(outcome);
        }
        let template = self
            .activity_runtimes
            .template(TEMPLATE, REVISION)
            .ok_or_else(|| super::super::operator_event("summer_source_template_missing"))?;
        if !template.source_portal() {
            super::super::operator_event("summer_source_template_kind_refused");
            return Err(());
        }
        {
            let mut runtime = self.runtime.lock().await;
            if runtime.borrow_movement_position().read(actor) != Ok(snapshot)
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || runtime.content_pin().frame_binding_digest() != template.origin_frame_digest()
                || runtime.map_revision_digest() != template.origin_map_digest()
            {
                super::super::operator_event("summer_source_origin_current_fence_refused");
                return Err(());
            }
        }
        let owner = ActivityHandoffOwner::new(self.root.clone());
        owner
            .publish_qualified_summer_portal(self.character, self.holder, fence, template)
            .await
            .map_err(|_| super::super::operator_event("summer_source_catalogue_publish_refused"))?;
        let definition = owner
            .read_boss_room(self.character, self.holder, fence, BOSS)
            .await
            .map_err(|_| super::super::operator_event("summer_source_catalogue_read_refused"))?
            .ok_or_else(|| super::super::operator_event("summer_source_catalogue_missing"))?;
        let existing = owner
            .existing_summer_portal_room(self.character, self.holder, fence)
            .await
            .map_err(|_| super::super::operator_event("summer_source_current_room_read_refused"))?;
        self.ensure_activity_template_runtime_with_source_room(&definition, existing.as_ref())
            .await
            .map_err(|_| {
                super::super::operator_event("summer_source_runtime_qualification_refused")
            })?;
        let mut after = None;
        let mut selected = None;
        if let Some(existing) = existing {
            let registered = self
                .activity_runtimes
                .get(
                    existing.allocation.scope,
                    existing.allocation.generation.get(),
                )
                .ok_or(())?;
            let ready = {
                let runtime = registered.runtime.lock().await;
                boss_room_entry_native_ready(&runtime, &definition)
            };
            if !ready {
                super::super::operator_event("summer_source_occupied_room_not_ready");
                return Err(());
            }
            selected = Some((registered, existing.allocation, existing.handoff_id));
        } else {
            loop {
                let allocations = owner
                    .ready_allocations(self.character, self.holder, admitted.world_id, after)
                    .await
                    .map_err(|_| {
                        super::super::operator_event("summer_source_ready_allocation_read_refused")
                    })?;
                for allocation in &allocations {
                    let Some(registered) = self
                        .activity_runtimes
                        .get(allocation.scope, allocation.generation.get())
                    else {
                        continue;
                    };
                    if registered.assignment_source_revision != allocation.source_revision
                        || registered.template.as_ref().is_none_or(|key| {
                            key.key != definition.room_template_key
                                || key.revision != definition.template_revision
                        })
                    {
                        continue;
                    }
                    let ready = {
                        let runtime = registered.runtime.lock().await;
                        boss_room_native_ready(&runtime, &definition)
                    };
                    if ready {
                        selected = Some((
                            registered,
                            allocation.clone(),
                            *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes(),
                        ));
                        break;
                    }
                }
                if selected.is_some() || allocations.len() < 15 {
                    break;
                }
                let Some(RuntimeScopeRefV1::Instance { instance_id, .. }) =
                    allocations.last().map(|allocation| allocation.scope)
                else {
                    return Err(());
                };
                if after.is_some_and(|previous| previous >= instance_id) {
                    return Err(());
                }
                after = Some(instance_id);
            }
        }
        let Some((destination, allocation, handoff_id)) = selected else {
            super::super::operator_event("summer_source_no_ready_allocation");
            return Ok(());
        };
        let (saved, exit) = {
            let mut runtime = self.runtime.lock().await;
            if runtime.borrow_movement_position().read(actor) != Ok(snapshot) {
                return Err(());
            }
            (
                self.placed_record(&runtime, at),
                self.placed_record(
                    &runtime,
                    MovementLocalPosition {
                        x: definition.exit_position.x,
                        y: definition.exit_position.y,
                        floor: definition.exit_position.floor,
                    },
                ),
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
            handoff_id,
            command_id: occurrence,
            initiating_character_id: CharacterId::decode(fence.character_id.as_bytes())
                .map_err(|_| ())?,
            activity_key: definition.boss_key,
            template_revision: definition.template_revision,
            content_revision: definition.content_revision,
            destination_scope: allocation.scope,
            destination_generation: allocation.generation,
            destination_assignment_source_revision: allocation.source_revision,
            destination_content,
            time_limit_ms: definition.policy.time_limit_ms,
            participants: vec![ActivityParticipant {
                source: fence,
                destination_game_session_id: SecureIdentifiers.game_session_id().ok_or(())?,
                destination_transport: Some(SecureIdentifiers.transport_ref().ok_or(())?),
                selected_capabilities: admitted
                    .continuity
                    .selected_capabilities
                    .as_slice()
                    .to_vec(),
                source_position: saved.record,
                source_position_write_sequence: saved.write_sequence,
                destination_cell: *definition.entry_positions.first().ok_or(())?,
                exit_position: exit.record,
            }],
        };
        self.finish_source_portal_outcome(
            self.enter_native_activity(request, true)
                .await
                .map_err(|_| {
                    super::super::operator_event("summer_source_native_handoff_pending")
                })?,
        )
    }
    pub(in crate::gameplay_transport) async fn rakesh_source_return_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        snapshot: MovementPositionSnapshot,
    ) -> Result<(), ()> {
        let at = snapshot.position();
        if (at.x, at.y, at.floor) != (33043, 32986, -8) {
            return Ok(());
        }
        let admitted = self.activity_runtimes.connection(session).ok_or(())?;
        let fence = admitted.item_fence.ok_or(())?;
        if admitted.runtime_actor != Some(actor)
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
        {
            return Err(());
        }
        let registered = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
            .ok_or(())?;
        if registered
            .template
            .as_ref()
            .is_none_or(|key| key.key != TEMPLATE || key.revision != REVISION)
        {
            return Err(());
        }
        let template = self
            .activity_runtimes
            .template(TEMPLATE, REVISION)
            .ok_or(())?;
        {
            let mut runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.borrow_movement_position().read(actor) != Ok(snapshot)
                || runtime.map_revision_digest() != template.room().map_revision_digest()
                || runtime.content_pin().frame_binding_digest()
                    != template.room().frame_binding().digest()
                || !runtime
                    .player_control_facts(actor, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Err(());
            }
        }
        self.finish_source_portal_outcome(self.return_source_portal_activity(admitted).await?)
    }

    fn finish_source_portal_outcome(&self, outcome: NativeActivityOutcome) -> Result<(), ()> {
        match outcome {
            NativeActivityOutcome::Committed(commit) => {
                self.publish_activity_commit(&commit).map_err(|_| {
                    super::super::operator_event(
                        "summer_source_committed_scope_publication_pending",
                    )
                })
            }
            NativeActivityOutcome::Refused(_) => {
                super::super::operator_event("summer_source_native_admission_refused");
                Ok(())
            }
        }
    }
}
