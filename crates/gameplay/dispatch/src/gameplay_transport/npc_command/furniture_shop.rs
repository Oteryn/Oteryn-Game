//! Read-only presentation context. The economic writer rechecks durable stage18.
use super::*;
pub(super) const TRACK: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/questline";
impl NpcConversations {
    pub(super) fn refresh_furniture_shop_context(
        &self,
        session: GameSessionId,
        stage: Option<i64>,
    ) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.entry(session).or_default().furniture_questline = stage;
        }
    }
}
pub(super) fn trade_available(npc: &str, stage: Option<i64>) -> bool {
    npc != "oteryn:npc.hairycles" || stage.is_some_and(|value| value >= 18)
}
