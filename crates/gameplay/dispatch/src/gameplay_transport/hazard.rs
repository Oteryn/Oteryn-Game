//! Hazard level selection, composed with the admitted Character and native durability.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::hazard as wire;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn admit_hazard(&self, admitted: AdmittedSession) -> bool {
        let Some(actor) = admitted.runtime_actor else {
            return false;
        };
        let Some(fence) =
            super::character_gameplay_fence::current(self, admitted.game_session_id, false).await
        else {
            return false;
        };
        let Ok(Some((_, selected, _))) = self
            .root
            .character_hazard(self.character, self.holder, fence, None)
            .await
        else {
            return false;
        };
        self.runtime
            .lock()
            .await
            .install_hazard_level(actor, admitted.game_session_id, selected)
            .is_ok()
    }

    pub(super) async fn hazard_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> super::guild_intent::GuildReply {
        use super::guild_intent::GuildReply;
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return GuildReply::Rejected;
        };
        if id == 0
            || kind != wire::COMMAND_TYPE_HAZARD
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_HAZARD_V1)
            || wire::decode_request(payload).is_err()
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return GuildReply::Rejected;
        }
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if admitted.world_id != binding.world_id()
                || admitted.channel_id != binding.channel_id()
                || fence.runtime_scope
                    != crate::foundation::RuntimeScopeRefV1::channel(
                        binding.world_id(),
                        binding.channel_id(),
                    )
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
            {
                return GuildReply::Rejected;
            }
        }
        let Some(fence) =
            super::character_gameplay_fence::current(self, admitted.game_session_id, false).await
        else {
            return GuildReply::Rejected;
        };
        if fence.connection_generation.get() != admitted.continuity.connection_generation
            || admitted.item_fence.is_none_or(|expected| {
                expected.character_id != fence.character_id
                    || expected.character_lease_generation != fence.character_lease_generation
                    || expected.runtime_scope != fence.runtime_scope
                    || expected.scope_ownership_generation != fence.scope_ownership_generation
            })
        {
            return GuildReply::Rejected;
        }
        let Ok(selection) = wire::decode_request(payload) else {
            return GuildReply::Rejected;
        };
        match self
            .root
            .character_hazard(
                self.character,
                self.holder,
                fence,
                selection.map(|level| (id, level)),
            )
            .await
        {
            Ok(Some((_, selected, response))) => {
                let mut runtime = self.runtime.lock().await;
                if runtime
                    .install_hazard_level(actor, admitted.game_session_id, selected)
                    .is_err()
                {
                    return GuildReply::Unavailable;
                }
                GuildReply::Payload(response)
            }
            Ok(None) => GuildReply::Rejected,
            Err(_) => GuildReply::Unavailable,
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn hazard_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::guild_intent::GuildReply>
        {
            async { $crate::gameplay_transport::guild_intent::GuildReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn hazard_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> crate::gameplay_transport::guild_intent::GuildReply {
            self.hazard_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;

/// Both positions must belong to the qualified zone. Movement recomputes membership on use.
pub(super) fn effects(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    target: crate::foundation::ExactActorRef,
) -> Option<oteryn_gameplay_encounters::bosses::hazard_primal::HazardAreaEffects> {
    use oteryn_gameplay_encounters::bosses::hazard_primal::{
        creature_qualifies, gnomprona_contains, provisional_effects,
    };
    let creature = runtime.companion_snapshot_including_dead(target).ok()?;
    if creature.state.master.is_some() || !creature_qualifies(&creature.state.policy.definition_key)
    {
        return None;
    }
    for actor in [actor, target] {
        let at = runtime.read_actor_position(actor).ok()?.position();
        if !gnomprona_contains(at.x, at.y, at.floor) {
            return None;
        }
    }
    provisional_effects(oteryn_gameplay_combat::hazard::effective_level(
        runtime, actor, session,
    )?)
    .ok()
}

pub(super) fn outgoing_damage(
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    target: crate::foundation::ExactActorRef,
    damage: i64,
    occurrence: (u64, u16),
) -> i64 {
    use sha2::{Digest, Sha256};
    let Some(effects) = effects(runtime, actor, session, target) else {
        return damage;
    };
    let digest = Sha256::new()
        .chain_update(b"HAZARD/dodge/v1")
        .chain_update(actor.placement_identity())
        .chain_update(target.placement_identity())
        .chain_update(target.actor_local_generation().to_be_bytes())
        .chain_update(session.as_bytes())
        .chain_update(occurrence.0.to_be_bytes())
        .chain_update(occurrence.1.to_be_bytes())
        .finalize();
    let draw = u64::from_be_bytes(digest[..8].try_into().unwrap_or_default()) % 1_000_000;
    if effects.creature_dodges(draw as u32) {
        0
    } else {
        damage
    }
}
