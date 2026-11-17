//! MAPDISC: the admitted Character's Cyclopedia map discovery data and Measuring Tibia progress
//! (`cyclopedia-discovery-v1.md`), a fenced read of the Character Cyclopedia owner. Discovery
//! itself is written on the accepted movement path (`cyclopedia/step.rs`).
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::durability::character_cyclopedia::DiscoveryAreas;
use crate::foundation::CommandId;
use oteryn_protocol_oteryn::cyclopedia_discovery as wire;

/// Like its prerequisite 57 (`cyclopedia::offered`), inserted only with admitted Cyclopedia
/// content; no static offer set carries it.
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_CYCLOPEDIA_DISCOVERY_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_CYCLOPEDIA_DISCOVERY_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_CYCLOPEDIA_DISCOVERY_V1,
                requires: &[oteryn_protocol_oteryn::cyclopedia::CAPABILITY_CYCLOPEDIA_MAP_V1],
            },
        );
    }
    offers
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn cyclopedia_discovery_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        wire::decode_query(payload).ok()?;
        self.cyclopedia_content()?;
        let fence = admitted.item_fence?;
        CommandId::new(id).ok()?;
        let areas = DiscoveryAreas::new(&self.bundle_map()?.overlay.base().subareas)?;
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_CYCLOPEDIA_DISCOVERY_V1)
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || admitted.world_id != binding.world_id()
                || Some(admitted.channel_id) != binding.channel_id()
                || fence.runtime_scope != binding.runtime_scope()
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
            {
                return None;
            }
        }
        self.root
            .read_cyclopedia_discovery(self.character, self.holder, fence, areas)
            .await
            .ok()
            .flatten()
    }
}
macro_rules! authority_methods {
    () => {
        fn cyclopedia_discovery_query(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn cyclopedia_discovery_query(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.cyclopedia_discovery_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;
