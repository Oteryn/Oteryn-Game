//! Journal root catalogue captured from the admitted World at the real release caller.
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::{durability::journal::LogoutCatalogue, foundation::WorldId};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn journal_logout_store(
        &self,
        store: FreshAdmissionStore,
        world: WorldId,
    ) -> FreshAdmissionStore {
        let Some(map) = self.bundle_map() else {
            return store;
        };
        match LogoutCatalogue::new(world, map.facts.journal_depot_towns()) {
            Ok(catalogue) => store.with_journal_catalogue(catalogue),
            Err(_) => store,
        }
    }
}
