//! Optional Cake World state enriches the existing periodic owner; no new mutation root.
use super::now_seconds;
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::ComposedFreshAdmission;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn refresh_cake_regeneration(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) {
        let until = self
            .root
            .cake_world_bonus_until(self.character, self.world_id, now_seconds())
            .await
            .ok()
            .flatten();
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return;
        }
        let Some(state) = states.get_mut(&runtime, actor, session) else {
            return;
        };
        let now_wall = now_seconds();
        let now_native = self.owner_now().get();
        let until = until.and_then(|source_until| {
            let remaining = u64::try_from(source_until.checked_sub(now_wall)?).ok()?;
            if remaining == 0 {
                return None;
            }
            let native_until = now_native.checked_add(remaining.checked_mul(1_000_000)?)?;
            Some((source_until, native_until))
        });
        state.set_cake_regeneration_until(until);
    }
}
