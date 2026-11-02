//! ANIMUS-1 admitted, character-owned mastery query.

use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::animus_mastery::{CAPABILITY_ANIMUS_MASTERY_V1, MAX_ANIMUS_PAGE_KEYS};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut result: Vec<_> = base
        .iter()
        .copied()
        .filter(|entry| entry.id != CAPABILITY_ANIMUS_MASTERY_V1)
        .collect();
    if available {
        let at = result.partition_point(|entry| entry.id < CAPABILITY_ANIMUS_MASTERY_V1);
        result.insert(
            at,
            OfferedCapability {
                id: CAPABILITY_ANIMUS_MASTERY_V1,
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
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ANIMUS_MASTERY_V1)
            || !(1..=MAX_ANIMUS_PAGE_KEYS).contains(&page.limit)
        {
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
            .read_current_animus_masteries(self.character, self.holder, fence)
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
            // Query readiness belongs to the durable reader. Already-earned masteries
            // remain visible while the separate Soulpit acquisition owner is unavailable.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_animus_owner_removes_the_capability_from_scope_offers() {
        let base = [167, 168, 169].map(|id| OfferedCapability { id, requires: &[] });
        let actual = offered(&base, false);
        assert_eq!(
            actual.iter().map(|offer| offer.id).collect::<Vec<_>>(),
            [167, 169]
        );
    }

    #[test]
    fn available_animus_owner_inserts_one_sorted_offer() {
        let base = [167, 169].map(|id| OfferedCapability { id, requires: &[] });
        let actual = offered(&base, true);
        assert_eq!(
            actual.iter().map(|offer| offer.id).collect::<Vec<_>>(),
            [167, 168, 169]
        );
        assert_eq!(
            offered(&actual, true)
                .iter()
                .map(|offer| offer.id)
                .collect::<Vec<_>>(),
            [167, 168, 169]
        );
        assert!(actual[1].requires.is_empty());
    }
}
