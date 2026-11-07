//! The pinned Soulpit roster enters through the shared physical and durable owner.
use super::super::connection::{ConnectionIdentifiers, UseOutcome};
use super::super::native_activity::NativeActivityOutcome;
use super::super::{ComposedFreshAdmission, SecureIdentifiers};
use super::{SoulpitEntryIntent, outcome};
use crate::durability::activity_handoff::{
    ActivityContentPin, ActivityEnterRequest, ActivityParticipant, ActivityReturnReason,
};
use crate::durability::character_last_position::LastPositionCell;
use crate::durability::soulpit::{
    SOULPIT_ACTIVITY_KEY, SoulpitEntryRequest, SoulpitParticipantFact,
};
use crate::foundation::{
    CharacterId, CommandId, CommandRef, MovementLocalPosition, RuntimeScopeRefV1,
};
use oteryn_gameplay_content::content::bosses::activity_template::PinnedActivitySourceRoom;
use oteryn_gameplay_domain::domain::soulpit::{EXIT_POSITION, MAX_PARTICIPANTS, entry_destination};
use oteryn_protocol_oteryn::world_object::UseDisposition;

impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn enter_shared_soulpit(
        &self,
        intent: SoulpitEntryIntent,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(admitted) = self
            .activity_runtimes
            .connection(intent.command.game_session_id)
        else {
            return Ok(intent
                .selected
                .as_ref()
                .map(|_| outcome(UseDisposition::Rejected)));
        };
        if let Some(result) = self
            .retry_native_soulpit_activity(
                admitted,
                intent.command.command_id,
                &intent.encoded_intent,
            )
            .await?
        {
            return self.soulpit_entry_result(result).map(Some);
        }
        // A historical probe never invents a new allocation or consumes custody.
        let Some(core) = intent.selected else {
            return Ok(None);
        };
        let Some(fence) = intent.command.item_fence else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        if admitted.runtime_actor != Some(intent.actor)
            || admitted.item_fence != Some(fence)
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let destination = match self
            .ensure_pinned_source_runtime(PinnedActivitySourceRoom::Soulpit, None, 128)
            .await
        {
            Ok(owner) => owner,
            Err(_) => return Ok(Some(outcome(UseDisposition::Rejected))),
        };
        let (participants, facts) = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let census = runtime.positioned_actor_census().map_err(|_| ())?;
            let mut participants = Vec::new();
            let mut facts = Vec::new();
            for slot in 0..MAX_PARTICIPANTS {
                let mut occupants = census.iter().filter(|(_, at, session)| {
                    let at = at.position();
                    session.is_some()
                        && at.x == 32375
                        && at.y == 31158 + slot as i32
                        && matches!(at.floor, -8 | 8)
                });
                let Some((actor, at, Some(session))) = occupants.next() else {
                    continue;
                };
                if occupants.next().is_some() {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                }
                let member = self.activity_runtimes.connection(*session).ok_or(())?;
                let Some(source) = member.item_fence else {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                };
                if member.runtime_actor != Some(*actor)
                    || member.controller.is_none()
                    || source.runtime_scope != fence.runtime_scope
                    || source.scope_ownership_generation != fence.scope_ownership_generation
                    || !runtime
                        .player_control_facts(*actor, *session)
                        .is_ok_and(|p| p.control_loss.is_none())
                    || !self.states.player_trades.physical(*session, None)
                    || (slot == 0 && *actor != intent.actor)
                {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                }
                let position = at.position();
                let saved = self.placed_record(&runtime, position);
                let exit = self.placed_record(
                    &runtime,
                    MovementLocalPosition {
                        x: i32::from(EXIT_POSITION.x),
                        y: i32::from(EXIT_POSITION.y),
                        floor: position.floor,
                    },
                );
                let target = entry_destination(slot as u8).map_err(|_| ())?;
                participants.push(ActivityParticipant {
                    source,
                    destination_game_session_id: SecureIdentifiers.game_session_id().ok_or(())?,
                    destination_transport: Some(SecureIdentifiers.transport_ref().ok_or(())?),
                    selected_capabilities: member
                        .continuity
                        .selected_capabilities
                        .as_slice()
                        .to_vec(),
                    source_position: saved.record,
                    source_position_write_sequence: saved.write_sequence,
                    destination_cell: LastPositionCell {
                        x: i32::from(target.x),
                        y: i32::from(target.y),
                        floor: 8,
                    },
                    exit_position: exit.record,
                });
                facts.push(SoulpitParticipantFact {
                    character: source.character_id,
                    game_master: false,
                });
            }
            (participants, facts)
        };
        if !participants
            .iter()
            .any(|p| p.source == fence && p.source_position.cell.y == 31158)
        {
            return Ok(Some(outcome(UseDisposition::TooFar)));
        }
        // The active Character interpretation is independent of the entry
        // artifact's display revision. Type2 still checks every current root.
        let content_revision = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
            .map(|formula| formula.content_revision().to_owned())
            .ok_or(())?;
        let content = {
            let runtime = destination.runtime.lock().await;
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let handoff = *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes();
        let core = SoulpitEntryRequest {
            handoff_id: handoff,
            command: CommandRef::new(
                intent.command.game_session_id,
                CommandId::new(intent.command.command_id).map_err(|_| ())?,
            ),
            source: core.item_instance_id,
            intent: intent.encoded_intent,
            started_at_micros: self.owner_now().get(),
            participants: facts,
        };
        let request = ActivityEnterRequest {
            handoff_id: handoff,
            command_id: intent.command.command_id,
            initiating_character_id: CharacterId::decode(fence.character_id.as_bytes())
                .map_err(|_| ())?,
            activity_key: SOULPIT_ACTIVITY_KEY.into(),
            template_revision: "crystal-201189f".into(),
            content_revision,
            destination_scope: destination.scope,
            destination_generation: crate::foundation::ScopeOwnershipGeneration::new(
                destination.generation,
            )
            .map_err(|_| ())?,
            destination_assignment_source_revision: destination.assignment_source_revision,
            destination_content: content,
            time_limit_ms: 600_000,
            participants,
        };
        self.soulpit_entry_result(self.enter_native_soulpit_activity(request, core).await?)
            .map(Some)
    }

    fn soulpit_entry_result(&self, result: NativeActivityOutcome) -> Result<UseOutcome, ()> {
        match result {
            NativeActivityOutcome::Refused(reason) => {
                #[cfg(test)]
                eprintln!("SRX3_SOULPIT native_activity_refused={reason:?}");
                let _ = reason;
                Ok(outcome(UseDisposition::Rejected))
            }
            NativeActivityOutcome::Committed(commit) => {
                self.publish_activity_commit(&commit)?;
                Ok(outcome(UseDisposition::Committed))
            }
        }
    }

    pub(super) async fn return_shared_soulpit(
        &self,
        actor: crate::foundation::ExactActorRef,
        command: super::super::connection::UseCommand,
    ) -> Result<(), ()> {
        let admitted = self
            .activity_runtimes
            .connection(command.game_session_id)
            .ok_or(())?;
        if admitted.runtime_actor != Some(actor) || admitted.item_fence != command.item_fence {
            return Err(());
        }
        let fence = admitted.item_fence.ok_or(())?;
        let bound = self
            .root
            .read_soulpit_scope_binding(
                self.character,
                self.holder,
                fence.runtime_scope,
                fence.scope_ownership_generation,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if !bound.members.iter().any(|member| {
            member.destination_session == command.game_session_id
                && member.character.as_bytes() == fence.character_id.as_bytes()
        }) {
            return Err(());
        }
        let NativeActivityOutcome::Committed(commit) = self
            .return_native_activity(admitted, command.command_id, ActivityReturnReason::Exit)
            .await?
        else {
            return Err(());
        };
        self.publish_activity_commit(&commit)
    }
}
