//! The character's current admitted region/subarea, read through the real connection.
use super::guild_intent::GuildReply;
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::cyclopedia as wire;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_SUBAREA_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_SUBAREA_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_SUBAREA_V1,
                requires: &[],
            },
        );
    }
    offers
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn current_subarea(
        &self,
        admitted: AdmittedSession,
        kind: u32,
        payload: &[u8],
    ) -> GuildReply {
        if kind != wire::COMMAND_TYPE_SUBAREA_QUERY
            || !payload.is_empty()
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_SUBAREA_V1)
        {
            return GuildReply::Rejected;
        }
        let Some(actor) = admitted.runtime_actor else {
            return GuildReply::Rejected;
        };
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if admitted.world_id != binding.world_id()
            || admitted.channel_id != binding.channel_id()
            || runtime.owner_fence().is_err()
            || !runtime
                .player_control_facts(actor, admitted.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
        {
            return GuildReply::Rejected;
        }
        let Some(position) = super::bundle_map_position(&mut runtime, self.movement_cells, actor)
        else {
            return GuildReply::Rejected;
        };
        let base = position.map.overlay.base();
        if base.subareas.masks.is_empty() {
            return GuildReply::Rejected;
        }
        let membership = u16::try_from(position.at.x)
            .ok()
            .zip(u16::try_from(position.at.y).ok())
            .zip(i8::try_from(position.at.floor).ok())
            .and_then(|((x, y), floor)| base.subareas.resolve(x, y, floor));
        GuildReply::Payload(wire::encode_current_subarea(membership).to_vec())
    }
}

macro_rules! authority_methods {
    () => {
        fn subarea_available(&self) -> bool {
            false
        }
        fn subarea_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
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
        fn subarea_available(&self) -> bool {
            self.bundle_map()
                .is_some_and(|m| !m.overlay.base().subareas.masks.is_empty())
        }
        async fn subarea_command(
            &self,
            admitted: connection::AdmittedSession,
            kind: u32,
            payload: &[u8],
        ) -> guild_intent::GuildReply {
            self.current_subarea(admitted, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
