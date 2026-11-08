//! Boss lever commands compose authored content, native participants and the
//! shared scope handoff. A client key never supplies an actor, position or grant.
use super::connection::{AdmittedSession, ConnectionIdentifiers};
use super::native_activity::{NativeActivityOutcome, boss_room_native_ready};
use super::{ComposedFreshAdmission, SecureIdentifiers};
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterRequest, ActivityHandoffOwner, ActivityParticipant,
    ActivityRefusal, ActivityReturnReason,
};
use crate::durability::character_progression::CharacterProgressionError;
use crate::foundation::{CharacterId, CommandStatus, MovementLocalPosition, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::boss_encounter as wire;

pub(super) type BossActivityResult = Option<(CommandStatus, Vec<u8>)>;

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn boss_activity_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> BossActivityResult {
        let malformed = || Some((CommandStatus::Rejected, Vec::new()));
        if id == 0
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_BOSS_ENCOUNTER_V1)
            || !matches!(
                kind,
                wire::COMMAND_TYPE_BOSS_LEVER
                    | wire::COMMAND_TYPE_BOSS_DEFEATED
                    | wire::COMMAND_TYPE_BOSS_TIMEOUT
            )
        {
            return malformed();
        }
        let Ok(mut intent) = wire::decode_boss_key(payload) else {
            return malformed();
        };
        if let Some(canonical) = super::endgame_entry::canonical_boss_key(&intent.boss_key) {
            intent.boss_key = canonical;
        }
        let outburst = super::outburst_entry::is_outburst(&intent.boss_key);
        if outburst {
            intent.boss_key = "oteryn:boss.outburst".into();
        }
        let primal = super::primal_entry::is_primal(&intent.boss_key);
        if primal {
            intent.boss_key = super::primal_entry::BOSS.into();
        }
        let soul_war = super::soul_war_entry::is_soul_war(&intent.boss_key);
        if let Some(canonical) = soul_war {
            intent.boss_key = canonical.into();
        }
        let rotten = super::rotten_blood_entry::is_rotten_blood(&intent.boss_key);
        if let Some(canonical) = rotten {
            intent.boss_key = canonical.into();
        }
        let (Some(actor), Some(fence), Some(_)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            admitted.controller,
        ) else {
            return malformed();
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || actor.runtime_scope() != fence.runtime_scope
        {
            return malformed();
        }
        // Session ingress executes this route only for a newly accepted command;
        // actual replays return its existing cached result before this call.
        self.prune_refused_activity_attempts(
            super::activity_runtime::ActivityRefusalPrune::BeforeCommand {
                session: admitted.game_session_id,
                connection_generation: admitted.continuity.connection_generation,
                command: id,
            },
        )
        .await
        .ok()?;
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && let Some(retained) = self
                .retry_native_activity(admitted, id, &intent.boss_key)
                .await
                .ok()?
        {
            return self.boss_activity_result(retained);
        }
        let origin_pin = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return malformed();
            }
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && outburst
            && let Err(reason) = self
                .prepare_outburst_catalogue(admitted, &intent.boss_key)
                .await
        {
            return refusal(reason);
        }
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && primal
            && let Err(reason) = self.prepare_primal_catalogue(admitted).await
        {
            return refusal(reason);
        }
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && soul_war.is_some()
            && let Err(reason) = self
                .prepare_soul_war_catalogue(admitted, &intent.boss_key)
                .await
        {
            return refusal(reason);
        }
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && rotten.is_some()
            && let Err(reason) = self
                .prepare_rotten_blood_catalogue(admitted, &intent.boss_key)
                .await
        {
            return refusal(reason);
        }
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && !primal
            && soul_war.is_none()
            && rotten.is_none()
            && super::endgame_entry::encounter_for_boss_key(&intent.boss_key).is_some()
        {
            match self.endgame_boss_entry(admitted, payload).await {
                Ok(Err(reason)) => {
                    return refusal(reason.into());
                }
                Err(CharacterProgressionError::AuthorityRejected) => {
                    return refusal(ActivityRefusal::AuthorityRejected);
                }
                Ok(Ok(())) => {}
                Err(_) => return None,
            };
        }
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let Some(definition) = owner
            .read_boss_room_with_pin(
                self.character,
                self.holder,
                fence,
                &intent.boss_key,
                origin_pin,
            )
            .await
            .ok()?
        else {
            return refusal(ActivityRefusal::ContentUnavailable);
        };
        if kind == wire::COMMAND_TYPE_BOSS_LEVER
            && !super::endgame_room_policy::matches_source_cooldown(&definition)
        {
            return refusal(ActivityRefusal::ContentChanged);
        }
        if kind != wire::COMMAND_TYPE_BOSS_LEVER {
            let Some(scope_owner) = self
                .activity_runtimes
                .get(fence.runtime_scope, fence.scope_ownership_generation.get())
            else {
                return refusal(ActivityRefusal::AuthorityRejected);
            };
            if scope_owner.template.as_ref().is_none_or(|template| {
                template.key != definition.room_template_key
                    || template.revision != definition.template_revision
            }) {
                return refusal(ActivityRefusal::ContentChanged);
            }
            // These are requests to the deadline/state owner. The writer checks
            // the recorded native defeat or the actual server deadline itself.
            let reason = if kind == wire::COMMAND_TYPE_BOSS_DEFEATED {
                ActivityReturnReason::Defeated
            } else {
                ActivityReturnReason::Timeout
            };
            let result = self
                .return_native_activity(admitted, id, reason)
                .await
                .ok()?;
            return self.boss_activity_result(result);
        }
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
            return refusal(ActivityRefusal::AuthorityRejected);
        }
        let template_ready = if outburst {
            self.ensure_outburst_definition_runtime(&definition).await
        } else if rotten.is_some() {
            self.ensure_rotten_blood_definition_runtime(&definition)
                .await
        } else if soul_war.is_some() {
            self.ensure_soul_war_definition_runtime(&definition).await
        } else {
            self.ensure_activity_template_runtime(&definition).await
        };
        if let Err(reason) = template_ready {
            return refusal(reason);
        }
        let loaded = self.activity_runtimes.activity_owners().ok()?;
        if !loaded.iter().any(|registered| {
            registered.template.as_ref().is_some_and(|template| {
                template.key == definition.room_template_key
                    && template.revision == definition.template_revision
            })
        }) {
            return refusal(ActivityRefusal::ContentUnavailable);
        }
        let mut after = None;
        let mut selected = None;
        loop {
            let allocations = owner
                .ready_allocations(self.character, self.holder, admitted.world_id, after)
                .await
                .ok()?;
            for allocation in &allocations {
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
                if runtime.owner_fence().is_ok() && boss_room_native_ready(&runtime, &definition) {
                    drop(runtime);
                    selected = Some((registered, allocation.clone()));
                    break;
                }
            }
            if selected.is_some() || allocations.len() < 15 {
                break;
            }
            let Some(RuntimeScopeRefV1::Instance { instance_id, .. }) =
                allocations.last().map(|allocation| allocation.scope)
            else {
                return None;
            };
            if after.is_some_and(|previous| previous >= instance_id) {
                return None;
            }
            after = Some(instance_id);
        }
        let Some((destination, allocation)) = selected else {
            return refusal(ActivityRefusal::NoCapacity);
        };
        let mut participants = Vec::new();
        {
            let runtime = self.runtime.lock().await;
            let census = runtime.positioned_actor_census().ok()?;
            let partition = if definition.boss_key == "oteryn:boss.bakragore" {
                let cell = runtime.read_actor_position(actor).ok()?.position();
                let ordinal = definition
                    .lever_positions
                    .iter()
                    .position(|p| (p.x, p.y, p.floor) == (cell.x, cell.y, cell.floor));
                let Some(ordinal) = ordinal else {
                    return refusal(ActivityRefusal::Position);
                };
                Some(ordinal / 5)
            } else {
                None
            };
            for (ordinal, lever_cell) in definition.lever_positions.iter().enumerate() {
                if partition.is_some_and(|selected| ordinal / 5 != selected) {
                    continue;
                }

                let mut occupants = census.iter().filter(|(_, position, session)| {
                    let cell = position.position();
                    session.is_some()
                        && (cell.x, cell.y, cell.floor)
                            == (lever_cell.x, lever_cell.y, lever_cell.floor)
                });
                let Some((member_actor, position, Some(session))) = occupants.next() else {
                    continue;
                };
                if occupants.next().is_some() {
                    return refusal(ActivityRefusal::Position);
                }
                let Some(member) = self.activity_runtimes.connection(*session) else {
                    return refusal(ActivityRefusal::AuthorityRejected);
                };
                let (Some(member_fence), Some(_controller)) =
                    (member.item_fence, member.controller)
                else {
                    return refusal(ActivityRefusal::AuthorityRejected);
                };
                if member.runtime_actor != Some(*member_actor)
                    || member_fence.runtime_scope != fence.runtime_scope
                    || member_fence.scope_ownership_generation != fence.scope_ownership_generation
                    || !member
                        .continuity
                        .selected_capabilities
                        .contains(wire::CAPABILITY_BOSS_ENCOUNTER_V1)
                    || !runtime
                        .player_control_facts(*member_actor, *session)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                {
                    return refusal(ActivityRefusal::AuthorityRejected);
                }
                let saved = self.placed_record(&runtime, position.position());
                let exit = self.placed_record(
                    &runtime,
                    MovementLocalPosition {
                        x: definition.exit_position.x,
                        y: definition.exit_position.y,
                        floor: definition.exit_position.floor,
                    },
                );
                participants.push(ActivityParticipant {
                    source: member_fence,
                    destination_game_session_id: SecureIdentifiers.game_session_id()?,
                    destination_transport: Some(SecureIdentifiers.transport_ref()?),
                    selected_capabilities: member
                        .continuity
                        .selected_capabilities
                        .as_slice()
                        .to_vec(),
                    source_position: saved.record,
                    source_position_write_sequence: saved.write_sequence,
                    destination_cell: *definition.entry_positions.get(ordinal)?,
                    exit_position: exit.record,
                });
            }
        }
        if !participants
            .iter()
            .any(|participant| participant.source == fence)
        {
            return refusal(ActivityRefusal::Position);
        }
        if !(definition.policy.minimum_players..=definition.policy.maximum_players)
            .contains(&participants.len())
        {
            return refusal(ActivityRefusal::PartyCount);
        }
        // Crystal 201189f boss_lever.lua:183-218 validates the entire lever
        // roster. Endgame quest gates therefore apply to every participant.
        if super::endgame_entry::encounter_for_boss_key(&intent.boss_key).is_some() {
            for participant in &participants {
                let member = self
                    .activity_runtimes
                    .connection(participant.source.game_session_id)?;
                match self.endgame_boss_entry(member, payload).await {
                    Ok(Ok(())) => {}
                    Ok(Err(reason)) => return refusal(reason.into()),
                    Err(_) => return refusal(ActivityRefusal::AuthorityRejected),
                }
            }
        }
        let destination_content = {
            let runtime = destination.runtime.lock().await;
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let request = ActivityEnterRequest {
            handoff_id: *SecureIdentifiers.game_session_id()?.as_bytes(),
            command_id: id,
            initiating_character_id: CharacterId::decode(fence.character_id.as_bytes()).ok()?,
            activity_key: definition.boss_key,
            template_revision: definition.template_revision,
            content_revision: definition.content_revision,
            destination_scope: allocation.scope,
            destination_generation: allocation.generation,
            destination_assignment_source_revision: allocation.source_revision,
            destination_content,
            time_limit_ms: definition.policy.time_limit_ms,
            participants,
        };
        use sha2::Digest;
        let wire_binding = sha2::Sha256::digest(payload).into();
        self.boss_activity_result(
            self.enter_native_boss_activity(request, wire_binding)
                .await
                .ok()?,
        )
    }

    fn boss_activity_result(&self, result: NativeActivityOutcome) -> BossActivityResult {
        match result {
            NativeActivityOutcome::Refused(reason) => refusal(reason),
            NativeActivityOutcome::Committed(commit) => {
                self.publish_activity_commit(&commit).ok()?;
                let reply = wire::BossActivityReply::Accepted {
                    handoff: commit.handoff_id,
                    deadline_unix_ms: commit
                        .deadline_unix_ms
                        .map(u64::try_from)
                        .transpose()
                        .ok()?,
                };
                Some((
                    CommandStatus::Accepted,
                    wire::encode_boss_activity_reply(&reply).ok()?,
                ))
            }
        }
    }
}

fn refusal(reason: ActivityRefusal) -> BossActivityResult {
    use wire::BossActivityRefusal as Refusal;
    let reason = match reason {
        ActivityRefusal::InvalidInput => Refusal::InvalidInput,
        ActivityRefusal::AuthorityRejected => Refusal::AuthorityRejected,
        ActivityRefusal::NoCapacity => Refusal::NoCapacity,
        ActivityRefusal::ContentUnavailable => Refusal::ContentUnavailable,
        ActivityRefusal::ContentChanged => Refusal::ContentChanged,
        ActivityRefusal::ReplayConflict => Refusal::ReplayConflict,
        ActivityRefusal::NotDue => Refusal::NotDue,
        ActivityRefusal::AlreadyReturned => Refusal::AlreadyReturned,
        ActivityRefusal::OwnerStillCurrent => Refusal::OwnerStillCurrent,
        ActivityRefusal::PartyCount => Refusal::PartyCount,
        ActivityRefusal::Position => Refusal::Position,
        ActivityRefusal::Level => Refusal::Level,
        ActivityRefusal::Cooldown => Refusal::Cooldown,
    };
    Some((
        CommandStatus::Rejected,
        wire::encode_boss_activity_reply(&wire::BossActivityReply::Refused(reason)).ok()?,
    ))
}

#[path = "boss_activity/source_extra.rs"]
mod source_extra;
