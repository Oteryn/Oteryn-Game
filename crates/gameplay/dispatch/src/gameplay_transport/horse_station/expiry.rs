use super::*;
use crate::foundation::GameSessionId;
#[derive(Default)]
pub(in crate::gameplay_transport) struct ExpiryClocks(
    std::sync::Mutex<std::collections::BTreeMap<GameSessionId, u64>>,
);
impl ExpiryClocks {
    fn due(&self, session: GameSessionId, now: u64) -> bool {
        let Ok(mut clocks) = self.0.lock() else {
            return false;
        };
        if let Some(last) = clocks.get_mut(&session) {
            if now.saturating_sub(*last) < 15_000_000 {
                return false;
            }
            *last = now;
            return true;
        }
        if clocks.len() >= 256 {
            return false;
        }
        clocks.insert(session, now);
        true
    }
    pub(in crate::gameplay_transport) fn forget(&self, session: GameSessionId) {
        if let Ok(mut clocks) = self.0.lock() {
            clocks.remove(&session);
        }
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Source expiry is independent of food/combat, including named-House residents.
    pub(in crate::gameplay_transport) async fn observe_horse_contract_expiry(
        &self,
        actor: crate::foundation::ExactActorRef,
        session: GameSessionId,
    ) {
        // Initial snapshots run before the existing private-message connection
        // joins. Do not consume its one source expiry notice before a recipient
        // exists; the first subsequent vitals cadence will observe it instead.
        let generation =
            match super::super::character_gameplay_fence::current(self, session, false).await {
                Some(f) => f.connection_generation.get(),
                None => return,
            };
        if !self
            .states
            .server_messages
            .horse_rental_recipient_current(session, generation)
        {
            return;
        }
        if !self
            .states
            .horse_expiry
            .due(session, self.owner_now().get())
        {
            return;
        }
        let Some(fence) =
            super::super::character_gameplay_fence::current(self, session, false).await
        else {
            return;
        };
        {
            let runtime = self.runtime.lock().await;
            let valid = if runtime.house_actor_state(actor, session).is_ok() {
                runtime.house_player_control_facts(actor, session)
            } else {
                runtime.player_control_facts(actor, session)
            };
            if !valid.is_ok_and(|facts| facts.control_loss.is_none()) {
                return;
            }
        }
        if matches!(
            self.root
                .observe_horse_rental_expiry(self.character, self.holder, fence.item_fence())
                .await,
            Ok(true)
        ) {
            self.states
                .server_messages
                .horse_rental_expired(session, fence.connection_generation.get());
        }
    }
}
