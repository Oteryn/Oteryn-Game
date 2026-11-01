//! World PvP cleanup under the same current Channel scope as party expiry.
use super::ComposedFreshAdmission;
use crate::durability::pvp_state::PvpCleanup;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_source_pvp_expiry(&self) -> Option<PvpCleanup> {
        let permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().ok()?;
        let binding = runtime.binding();
        self.root
            .drain_pvp_expiry(
                &permit,
                self.character,
                self.holder,
                binding.runtime_scope(),
                binding.scope_generation().get(),
            )
            .await
            .ok()
    }
}
