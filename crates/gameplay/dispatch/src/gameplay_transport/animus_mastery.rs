//! ANIMUS-1 admitted, character-owned mastery query.

use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut result = base.to_vec();
    if available && !result.iter().any(|entry| entry.id == 168) {
        let at = result.partition_point(|entry| entry.id < 168);
        result.insert(
            at,
            OfferedCapability {
                id: 168,
                requires: &[],
            },
        );
    }
    result
}

macro_rules! authority_methods {
    () => {
        fn animus_mastery_available(&self) -> bool {
            false
        }
        fn animus_mastery_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _page: oteryn_protocol_oteryn::animus_mastery::AnimusPage,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn animus_mastery_page(
        &self,
        admitted: AdmittedSession,
        page: oteryn_protocol_oteryn::animus_mastery::AnimusPage,
    ) -> Option<Vec<u8>> {
        if !admitted.continuity.selected_capabilities.contains(168) {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let masteries = self
            .root
            .read_animus_masteries(self.character, fence.character_id)
            .await
            .ok()?;
        let total = u16::try_from(masteries.len()).ok()?;
        let start = usize::from(page.offset).min(masteries.len());
        let end = start
            .saturating_add(usize::from(page.limit))
            .min(masteries.len());
        oteryn_protocol_oteryn::animus_mastery::encode_page(total, &masteries[start..end])
    }
}

macro_rules! composed_methods {
    () => {
        fn animus_mastery_available(&self) -> bool {
            true
        }
        async fn animus_mastery_query(
            &self,
            admitted: connection::AdmittedSession,
            page: oteryn_protocol_oteryn::animus_mastery::AnimusPage,
        ) -> Option<Vec<u8>> {
            self.animus_mastery_page(admitted, page).await
        }
    };
}
pub(super) use composed_methods;
