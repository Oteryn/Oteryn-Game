//! Premium session registrations and the promotion display snapshot captured at login.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn admit_promotion(&self, session: GameSessionId) {
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        let _slot = self.revision_sequencer.acquire(fence.character_id).await;
        let item_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: fence.character_id,
            game_session_id: fence.game_session_id,
            connection_generation: fence.connection_generation,
            character_lease_generation: fence.character_lease_generation,
            runtime_scope: fence.runtime_scope,
            scope_ownership_generation: fence.scope_ownership_generation,
        };
        let Ok(promoted) = self
            .root
            .read_promotion_for_session(self.character, self.holder, item_fence)
            .await
        else {
            return;
        };
        let account = self
            .states
            .premium_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.account_of(session));
        let premium = match (account, self.premium_coordinator) {
            (Some(account), Some(coordinator)) => {
                coordinator.premium_gate(account).await.premium_current()
            }
            _ => false,
        };
        if let Ok(mut sessions) = self.states.premium_sessions.lock() {
            sessions.1.insert(
                session,
                oteryn_gameplay_progression::promotion::login_projection(promoted, premium),
            );
        }
    }
}

/// PREM-1b (D476): the sessions of this node holding each account's Premium registration.
#[derive(Default)]
pub struct PremiumSessions(
    std::collections::HashMap<[u8; 16], std::collections::HashSet<GameSessionId>>,
    pub(super) std::collections::HashMap<GameSessionId, bool>,
);

impl PremiumSessions {
    pub(super) fn admit(&mut self, account_id: [u8; 16], session: GameSessionId) {
        self.0.entry(account_id).or_default().insert(session);
    }

    /// Drops `session`'s registration. True only when it held one and was the account's last,
    /// so a repeated release, or one racing a successor's admission, stops no pulls.
    /// PREM-WIRE-1: the account `session` was admitted for, if it holds a registration.
    pub(super) fn account_of(&self, session: GameSessionId) -> Option<[u8; 16]> {
        self.0
            .iter()
            .find_map(|(account, sessions)| sessions.contains(&session).then_some(*account))
    }

    pub(super) fn release(&mut self, account_id: [u8; 16], session: GameSessionId) -> bool {
        self.1.remove(&session);
        let Some(sessions) = self.0.get_mut(&account_id) else {
            return false;
        };
        if !sessions.remove(&session) || !sessions.is_empty() {
            return false;
        }
        self.0.remove(&account_id);
        true
    }
}
