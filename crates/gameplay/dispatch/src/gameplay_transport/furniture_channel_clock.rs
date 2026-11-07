//! Existing source-qualified timed rows, including off-screen Channel Ground.
use super::ComposedFreshAdmission;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn drain_ground_furniture_timers(&self) {
        let Some(permit) = self.spell_lane_permit().await else {
            return;
        };
        let (scope, generation) = {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return;
            }
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        let mut changed = super::loop_move::GroundChange(Some(&self.states.loop_ground));
        if matches!(
            self.root
                .expire_ground_furniture_timers(&permit, self.holder, scope, generation)
                .await,
            Ok(0)
        ) {
            changed.0 = None;
        }
    }
}
