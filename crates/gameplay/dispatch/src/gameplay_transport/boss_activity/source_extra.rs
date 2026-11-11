//! The shared native owner freezes both scopes before evaluating the source
//! onUseExtra prefix; source callbacks never infer permission from a client key.
use super::*;
use crate::durability::activity_handoff::boss_room::{
    BossRoomDefinition, QualifiedBossPadExtra, QualifiedBossPadPrefix, SourceBossLeverExtra,
};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn qualify_source_pad_prefix(
        &self,
        runtime: &ChannelRuntimeV1,
        destination: &ChannelRuntimeV1,
        definition: &BossRoomDefinition,
        request: &ActivityEnterRequest,
        wire: [u8; 32],
    ) -> Result<Result<Option<QualifiedBossPadPrefix>, ActivityRefusal>, ()> {
        if !matches!(
            definition.boss_key.as_str(),
            "oteryn:boss.goshnar_s_hatred" | "oteryn:boss.outburst"
        ) {
            return Ok(Ok(None));
        }
        runtime.owner_fence().map_err(|_| ())?;
        destination.owner_fence().map_err(|_| ())?;
        // Fresh private Instance isolation is the existing accepted policy;
        // inspect the selected physical owner rather than another room's roster.
        if destination
            .positioned_actor_census()
            .map_err(|_| ())?
            .iter()
            .any(|(_, _, session)| session.is_some())
        {
            return Ok(Err(ActivityRefusal::NoCapacity));
        }
        if !(definition.policy.minimum_players..=definition.policy.maximum_players)
            .contains(&request.participants.len())
        {
            return Ok(Err(ActivityRefusal::PartyCount));
        }
        let mut ordered = request.participants.clone();
        if ordered
            .iter()
            .any(|p| !definition.lever_positions.contains(&p.source_position.cell))
        {
            return Ok(Err(ActivityRefusal::Position));
        }
        ordered.sort_by_key(|p| {
            definition
                .lever_positions
                .iter()
                .position(|cell| *cell == p.source_position.cell)
        });
        let Some(principal) = ordered.iter().find(|p| {
            p.source.character_id.as_bytes() == request.initiating_character_id.as_bytes()
        }) else {
            return Ok(Err(ActivityRefusal::Position));
        };
        let command = CommandRef::new(
            principal.source.game_session_id,
            CommandId::new(request.command_id).map_err(|_| ())?,
        );
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let prefix = owner
            .read_source_pad_prefix(
                self.character,
                self.holder,
                definition,
                &ordered,
                command,
                wire,
            )
            .await
            .map_err(|_| ())?;
        let Some(prefix) = prefix else {
            return Ok(Err(ActivityRefusal::AuthorityRejected));
        };
        // The SQL proof describes this exact current source roster. Native
        // position, connection and pin checks remain independent after the read.
        for pad in prefix.pads() {
            self.current_source_extra_actor(runtime, pad).ok_or(())?;
        }
        Ok(Ok(Some(prefix)))
    }
    pub(in crate::gameplay_transport) fn current_source_extra_actor(
        &self,
        runtime: &ChannelRuntimeV1,
        pad: &QualifiedBossPadExtra,
    ) -> Option<ExactActorRef> {
        let fence = pad.source();
        let position = pad.position();
        let admitted = self.activity_runtimes.connection(fence.game_session_id)?;
        let actor = admitted.runtime_actor?;
        let pin = runtime.content_pin();
        let lease = runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || admitted.item_fence != Some(fence)
            || admitted.continuity.connection_generation != fence.connection_generation.get()
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || position.world_id != runtime.binding().world_id()
            || position.coordinate_frame_digest != pin.frame_binding_digest()
            || position.map_revision_digest != pin.map_revision_digest()
            || position.server_artifact_digest != pin.server_artifact_digest()
            || runtime
                .player_control_facts(actor, fence.game_session_id)
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        let at = runtime.read_actor_position(actor).ok()?.position();
        ((at.x, at.y, at.floor) == (position.cell.x, position.cell.y, position.cell.floor))
            .then_some(actor)
    }
    pub(in crate::gameplay_transport) async fn apply_source_pad_prefix(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut super::super::actor_spell::ChannelSpellStates,
        destination: &ChannelRuntimeV1,
        destination_states: &mut super::super::actor_spell::ChannelSpellStates,
        prefix: &QualifiedBossPadPrefix,
    ) -> Result<Option<ActivityRefusal>, ()> {
        let active = self.active_generation.ok_or(())?;
        let native = active.native_gameplay().ok_or(())?;
        let room = self.effective_rooms().gate_field_step_ingress().ok_or(())?;
        let pin = runtime.content_pin();
        if native.source_digest() != pin.server_artifact_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
        {
            return Err(());
        }
        // Refuse a stale physical/connection proof before publishing any extra.
        for pad in prefix.pads() {
            let actor = self.current_source_extra_actor(runtime, pad).ok_or(())?;
            if states.is_dead(actor)
                || states.has_pending_spell_commit(actor, pad.source().game_session_id)
            {
                return Err(());
            }
        }
        if let Some(first) = prefix.pads().first()
            && first.kind() == SourceBossLeverExtra::Hatred
        {
            let connection = self
                .activity_runtimes
                .connection(first.command().game_session_id())
                .ok_or(())?;
            destination_states
                .soul_war_callbacks
                .apply_hatred_source_prefix(
                    runtime,
                    states,
                    destination,
                    native,
                    prefix,
                    connection.continuity.connection_generation,
                )?;
            return Ok(prefix.refusal());
        }
        for pad in prefix.pads() {
            match pad.kind() {
                SourceBossLeverExtra::Hatred => return Err(()),
                SourceBossLeverExtra::Outburst => {
                    states
                        .native_callbacks
                        .apply_outburst_pad_extra(runtime, active, room, native, pad)
                        .await?
                }
            }
        }
        Ok(prefix.refusal())
    }
}
