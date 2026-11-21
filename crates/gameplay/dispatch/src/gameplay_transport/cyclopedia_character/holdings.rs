use super::*;
use crate::content::ReferenceItemField::Known;
use std::collections::BTreeMap;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn cyclopedia_holdings(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Result<Vec<u8>, ()> {
        use wire::items::filtered::{self, Query};
        if !admitted
            .continuity
            .selected_capabilities
            .contains(filtered::CAPABILITY)
        {
            return Err(());
        }
        let query = Query::decode(payload).map_err(|_| ())?;
        let native = self
            .active_generation
            .ok_or(())?
            .native_gameplay()
            .ok_or(())?;
        let mut names = BTreeMap::new();
        for record in native.item_profiles() {
            let Known(presentation) = &record.semantics.presentation else {
                continue;
            };
            let Known(name) = &presentation.name else {
                continue;
            };
            if !name.trim().is_empty() {
                names.insert(
                    record.production_definition.production_key.clone(),
                    name.clone(),
                );
            }
        }
        let fence = admitted.item_fence.ok_or(())?;
        let page = self
            .root
            .read_cyclopedia_item_summary_filtered(
                self.character,
                self.holder,
                fence,
                query,
                names.into_iter().collect(),
            )
            .await
            .map_err(|_| ())?;
        page.encode().map_err(|_| ())
    }
}
