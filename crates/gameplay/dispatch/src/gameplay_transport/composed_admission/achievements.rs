use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn observe_current_achievement_facts(
        &self,
        admitted: AdmittedSession,
    ) -> Option<connection::AchievementNoticeObservation> {
        let controller = admitted.controller?;
        if !self.achievement_notice_session_current(admitted).await {
            return None;
        }
        let character = self
            .root
            .read_current_character(self.character, admitted.item_fence?.character_id)
            .await
            .ok()?;
        if character.account_id.as_bytes() != &controller.account_id
            || character.world_id.as_bytes() != admitted.world_id.as_bytes()
        {
            return None;
        }
        let facts = self
            .root
            .read_account_achievements(
                self.character,
                controller.account_id,
                self.achievements.len().saturating_add(1),
            )
            .await
            .ok()?;
        let observation = notice_facts(self.achievements, &facts)?;
        // An account read is intentionally read-only. Recheck the exact controller after
        // that await so a replaced connection never publishes its successor's observations.
        self.achievement_notice_session_current(admitted)
            .await
            .then_some(observation)
    }

    async fn achievement_notice_session_current(&self, admitted: AdmittedSession) -> bool {
        let (Some(controller), Some(actor), Some(expected)) = (
            admitted.controller,
            admitted.runtime_actor,
            admitted.item_fence,
        ) else {
            return false;
        };
        if expected.game_session_id != admitted.game_session_id
            || expected.connection_generation.get() != admitted.continuity.connection_generation
        {
            return false;
        }
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
        else {
            return false;
        };
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(controller.transport)
            || item_fence_of(current) != Some(expected)
        {
            return false;
        }
        self.runtime
            .lock()
            .await
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
    }

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

/// Names and the complete watermark come from the same bounded fact read. Timestamps are
/// deliberately not cursors: two grants may share a millisecond or commit out of time order.
fn notice_facts(
    catalogue: &crate::achievement_catalogue::AchievementCatalogue,
    facts: &[crate::durability::account_achievement::EarnedAchievement],
) -> Option<connection::AchievementNoticeObservation> {
    if facts.len() > catalogue.len() {
        return None;
    }
    let watermark = achievement_watermark(catalogue, facts)?;
    let mut keys = std::collections::BTreeSet::new();
    let mut notices = Vec::with_capacity(facts.len());
    for fact in facts {
        if !keys.insert(&fact.achievement_key) {
            return None;
        }
        let row = catalogue
            .account_achievements_page(std::slice::from_ref(fact), 0)
            .ok()?
            .rows
            .into_iter()
            .next()?;
        notices.push(
            oteryn_protocol_oteryn::achievement_notices::AchievementEarned {
                key: row.key,
                name: row.name,
                watermark,
            },
        );
    }
    Some(connection::AchievementNoticeObservation {
        watermark,
        facts: Some(notices),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::account_achievement::EarnedAchievement;

    #[test]
    fn p11_notice_facts_use_catalogue_metadata_without_a_timestamp_cursor()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = crate::achievement_catalogue::AchievementCatalogue::embedded()
            .map_err(|error| format!("{error:?}"))?;
        let mut facts = [
            EarnedAchievement {
                achievement_key: "oteryn:achievement/here_fishy_fishy".into(),
                earned_at_unix_ms: 1_234,
            },
            EarnedAchievement {
                achievement_key: "oteryn:achievement/skin_deep".into(),
                earned_at_unix_ms: 1_234,
            },
        ];
        let same_time = notice_facts(&catalogue, &facts).ok_or("same-time facts")?;
        assert_eq!(same_time.watermark.fact_count, 2);
        assert_eq!(same_time.watermark.total_points, 5);
        let names: Vec<_> = same_time
            .facts
            .as_ref()
            .ok_or("complete facts")?
            .iter()
            .map(|fact| fact.name.as_str())
            .collect();
        assert_eq!(names, ["Here, Fishy Fishy!", "Skin-Deep"]);
        facts[1].earned_at_unix_ms = 1;
        assert_eq!(
            notice_facts(&catalogue, &facts),
            Some(same_time),
            "a later observation can contain an older timestamp"
        );
        assert!(notice_facts(&catalogue, &[facts[0].clone(), facts[0].clone()]).is_none());
        facts[1].achievement_key = "oteryn:achievement/not_in_the_catalogue".into();
        assert!(
            notice_facts(&catalogue, &facts).is_none(),
            "an unknown key cannot silently understate the watermark"
        );
        Ok(())
    }
}
