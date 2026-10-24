use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_account_achievement_notices(
        &self,
        account_id: [u8; 16],
    ) -> Option<oteryn_protocol_oteryn::achievement_notices::AchievementWatermark> {
        let limit = self.achievements.len().saturating_add(1);
        let facts = self
            .root
            .read_account_achievements(self.character, account_id, limit)
            .await
            .ok()?;
        achievement_watermark(self.achievements, &facts)
    }

    pub(in crate::gameplay_transport) async fn query_account_achievements(
        &self,
        request: AccountAchievementsRequest,
    ) -> AccountAchievementsReply {
        let limit = self.achievements.len().saturating_add(1);
        match self
            .root
            .read_account_achievements(self.character, request.account_id, limit)
            .await
        {
            Ok(facts) => {
                account_achievements_reply(self.achievements, &facts, &request, &mut operator_event)
            }
            Err(_) => AccountAchievementsReply::Rejected,
        }
    }
}
