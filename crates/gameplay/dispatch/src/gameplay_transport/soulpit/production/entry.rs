use super::*;

pub(super) async fn enter(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    request: SoulpitEntryIntent,
    _test_handoff_id: Option<[u8; 16]>,
) -> Result<Option<UseOutcome>, ()> {
    let fence = request.command.item_fence.ok_or(())?;
    let key = crate::gameplay_transport::native_activity::ActivityAttemptKey {
        session: request.command.game_session_id,
        connection_generation: fence.connection_generation.get(),
        command: request.command.command_id,
    };
    let retained = admission
        .activity_runtimes
        .soulpit_entries
        .lock()
        .map_err(|_| ())?
        .get(&key)
        .cloned();
    if let Some(retained) = retained {
        if retained.actor != request.actor
            || retained.core.intent != request.encoded_intent
            || request
                .selected
                .as_ref()
                .is_some_and(|selection| selection.item_instance_id != retained.core.source)
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        return finish_entry(admission, key, retained).await.map(Some);
    }
    let Some(core_selection) = request.selected else {
        return Ok(None);
    };
    let Some(template) = admission.activity_runtimes.soulpit_template() else {
        return Ok(Some(outcome(UseDisposition::NotPossible)));
    };
    let active = admission.active_generation.ok_or(())?;
    active.native_gameplay().ok_or(())?;
    let content = admission
        .root
        .read_character_root_interpretation(admission.character, fence.character_id)
        .await
        .map_err(|_| ())?
        .ok_or(())?;
    let source_scope = fence.runtime_scope;
    if !matches!(source_scope, RuntimeScopeRefV1::Channel { .. }) {
        return Ok(Some(outcome(UseDisposition::Rejected)));
    }
    let destination = match admission.ensure_soulpit_runtime().await {
        Ok(destination) => destination,
        Err(()) => return Ok(Some(outcome(UseDisposition::NotPossible))),
    };
    let mut participants = Vec::new();
    let mut members = Vec::new();
    {
        let runtime = admission.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.binding().runtime_scope() != source_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let census = runtime.positioned_actor_census().map_err(|_| ())?;
        if census
            .iter()
            .any(|(_, position, session)| session.is_some() && in_arena(position.position()))
        {
            return Ok(Some(outcome(UseDisposition::NotPossible)));
        }
        for (slot, staging) in template.staging_positions().iter().enumerate() {
            let mut occupants = census.iter().filter(|(_, position, session)| {
                let at = position.position();
                session.is_some()
                    && (at.x, at.y, i32::from(at.floor)) == (staging.x, staging.y, staging.z)
            });
            let Some((actor, position, Some(session))) = occupants.next() else {
                continue;
            };
            if occupants.next().is_some() {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let admitted = admission.activity_runtimes.connection(*session).ok_or(())?;
            let member_fence = admitted.item_fence.ok_or(())?;
            if admitted.runtime_actor != Some(*actor)
                || member_fence.runtime_scope != source_scope
                || member_fence.scope_ownership_generation != fence.scope_ownership_generation
                || !admitted
                    .continuity
                    .selected_capabilities
                    .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
                || !runtime
                    .player_control_facts(*actor, *session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let saved = admission.placed_record(&runtime, position.position());
            let exit = template.exit_position();
            let exit = admission.placed_record(
                &runtime,
                MovementLocalPosition {
                    x: exit.x,
                    y: exit.y,
                    floor: exit.z as i16,
                },
            );
            let destination = template.entry_positions()[slot];
            participants.push(ActivityParticipant {
                source: member_fence,
                destination_game_session_id: SecureIdentifiers.game_session_id().ok_or(())?,
                destination_transport: Some(SecureIdentifiers.transport_ref().ok_or(())?),
                selected_capabilities: admitted
                    .continuity
                    .selected_capabilities
                    .as_slice()
                    .to_vec(),
                source_position: saved.record,
                source_position_write_sequence: saved.write_sequence,
                destination_cell: crate::durability::character_last_position::LastPositionCell {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.z as i16,
                },
                exit_position: exit.record,
            });
            members.push((
                CharacterId::decode(member_fence.character_id.as_bytes()).map_err(|_| ())?,
                slot as u8,
            ));
        }
    }
    if !participants
        .iter()
        .any(|participant| participant.source == fence)
    {
        return Ok(Some(outcome(UseDisposition::Rejected)));
    }
    let destination_content = {
        let runtime = destination.runtime.lock().await;
        ActivityContentPin {
            coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
            map_revision_digest: runtime.map_revision_digest(),
            server_artifact_digest: runtime.content_pin().server_artifact_digest(),
        }
    };
    #[cfg(test)]
    let handoff_id = match _test_handoff_id {
        Some(id) => id,
        None => *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes(),
    };
    #[cfg(not(test))]
    let handoff_id = *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes();
    let core = SoulpitEntryRequest {
        handoff_id,
        command: CommandRef::new(
            request.command.game_session_id,
            CommandId::new(request.command.command_id).map_err(|_| ())?,
        ),
        source: core_selection.item_instance_id,
        intent: request.encoded_intent,
        started_at_micros: admission.owner_now().get(),
        participants: members
            .iter()
            .map(|(character, _)| {
                Ok(SoulpitParticipantFact {
                    character: crate::domain::CharacterId::from_bytes(*character.as_bytes())
                        .map_err(|_| ())?,
                    game_master: false,
                })
            })
            .collect::<Result<Vec<_>, ()>>()?,
    };
    let entry = ActivityEnterRequest {
        handoff_id,
        command_id: request.command.command_id,
        initiating_character_id: CharacterId::decode(fence.character_id.as_bytes())
            .map_err(|_| ())?,
        activity_key: SOULPIT_ACTIVITY_KEY.into(),
        template_revision: template.template_revision().into(),
        content_revision: content.revisions()[2].into(),
        destination_scope: destination.scope,
        destination_generation: ScopeOwnershipGeneration::new(destination.generation)
            .map_err(|_| ())?,
        destination_assignment_source_revision: destination.assignment_source_revision,
        destination_content,
        time_limit_ms: 600_000,
        participants,
    };
    let prepared = PreparedEntry {
        actor: request.actor,
        entry,
        core,
        refused: false,
    };
    {
        let mut runs = admission
            .activity_runtimes
            .soulpit_runs
            .lock()
            .map_err(|_| ())?;
        let key = ScopeKey::new(destination.scope, destination.generation);
        if runs.contains_key(&key) {
            return Ok(Some(outcome(UseDisposition::NotPossible)));
        }
        runs.insert(
            key,
            SoulpitRun {
                binding: SoulpitActivityBinding {
                    handoff_id,
                    scope: destination.scope,
                    generation: ScopeOwnershipGeneration::new(destination.generation)
                        .map_err(|_| ())?,
                },
                members,
            },
        );
    }
    admission
        .activity_runtimes
        .soulpit_entries
        .lock()
        .map_err(|_| ())?
        .insert(key, prepared.clone());
    finish_entry(admission, key, prepared).await.map(Some)
}

async fn finish_entry(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    key: crate::gameplay_transport::native_activity::ActivityAttemptKey,
    prepared: PreparedEntry,
) -> Result<UseOutcome, ()> {
    match admission
        .enter_native_activity_with_soulpit(prepared.entry.clone(), false, Some(prepared.core))
        .await?
    {
        NativeActivityOutcome::Committed(commit) => {
            if let Some(retained) = admission
                .activity_runtimes
                .soulpit_entries
                .lock()
                .map_err(|_| ())?
                .get_mut(&key)
                .filter(|retained| retained.entry.handoff_id == prepared.entry.handoff_id)
            {
                retained.refused = false;
            }
            admission.publish_activity_commit(&commit)?;
            Ok(outcome(UseDisposition::Committed))
        }
        NativeActivityOutcome::Refused(_) => {
            if let Some(retained) = admission
                .activity_runtimes
                .soulpit_entries
                .lock()
                .map_err(|_| ())?
                .get_mut(&key)
                .filter(|retained| retained.entry.handoff_id == prepared.entry.handoff_id)
            {
                retained.refused = true;
            }
            admission
                .activity_runtimes
                .soulpit_runs
                .lock()
                .map_err(|_| ())?
                .remove(&ScopeKey::new(
                    prepared.entry.destination_scope,
                    prepared.entry.destination_generation.get(),
                ));
            Ok(outcome(UseDisposition::Rejected))
        }
    }
}

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    async fn ensure_soulpit_runtime(
        &self,
    ) -> Result<crate::gameplay_transport::activity_runtime::RegisteredRuntime<'a>, ()> {
        let _transition = self.activity_runtimes.transitions.lock().await;
        let template = self.activity_runtimes.soulpit_template().ok_or(())?;
        let active = self.active_generation.ok_or(())?;
        let native = active.native_gameplay().ok_or(())?;
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let mut after = None;
        loop {
            let allocations = owner
                .ready_allocations(self.character, self.holder, self.world_id, after)
                .await
                .map_err(|_| ())?;
            for allocation in &allocations {
                if let Some(registered) = self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                {
                    if registered.template.as_ref().is_some_and(|key| {
                        key.key == template.template_key()
                            && key.revision == template.template_revision()
                    }) {
                        return Ok(registered);
                    }
                    continue;
                }
                let assignment = self
                    .root
                    .read_runtime_scope_assignment(allocation.scope)
                    .await
                    .map_err(|_| ())?
                    .ok_or(())?;
                if assignment.ownership_generation != allocation.generation.get()
                    || assignment.source_revision != allocation.source_revision
                {
                    continue;
                }
                let pin = template.issue_runtime_pin(active).map_err(|_| ())?;
                // The collision owner permits at most one actor per source arena cell.
                let [x0, y0, x1, y1, _, _] = crate::content::bosses::soulpit_template::ARENA_BOUNDS;
                let capacity = usize::try_from((x1 - x0 + 1) * (y1 - y0 + 1)).map_err(|_| ())?
                    + crate::domain::soulpit::MAX_PARTICIPANTS;
                let mut runtime =
                    crate::foundation::ChannelRuntimeV1::from_committed_scope_assignment(
                        allocation.scope,
                        self.holder.fact().node_id(),
                        self.holder.fact().registration_revision(),
                        assignment.ownership_generation,
                        assignment.source_revision,
                        &assignment.decision_identity,
                        capacity,
                        pin,
                    )
                    .map_err(|_| ())?;
                runtime
                    .install_companion_policies(native.policies().clone())
                    .map_err(|_| ())?;
                let room = template.room();
                let reference =
                    crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                        .map_err(|_| ())?;
                let content_fence =
                    crate::world_runtime::ScopeContentGenerationFence::for_activation(
                        allocation.scope,
                        allocation.generation,
                        reference,
                    );
                let door = crate::world_runtime::bind_native_entry_door(
                    room.door(),
                    &content_fence,
                    allocation.scope,
                    allocation.generation,
                )
                .map_err(|_| ())?;
                self.register_activity_runtime(
                    ActivityTemplateKey {
                        key: template.template_key().into(),
                        revision: template.template_revision().into(),
                    },
                    &assignment,
                    runtime,
                    door,
                    ActivityRuntimeAssets {
                        movement_cells: room.movement_cells(),
                        rooms: SeamRooms::new(None, Some(room)),
                    },
                )
                .await
                .map_err(|_| ())?;
                return self
                    .activity_runtimes
                    .get(allocation.scope, allocation.generation.get())
                    .ok_or(());
            }
            if allocations.len() < 15 {
                return Err(());
            }
            let Some(RuntimeScopeRefV1::Instance { instance_id, .. }) =
                allocations.last().map(|row| row.scope)
            else {
                return Err(());
            };
            if after.is_some_and(|previous| previous >= instance_id) {
                return Err(());
            }
            after = Some(instance_id);
        }
    }
}
