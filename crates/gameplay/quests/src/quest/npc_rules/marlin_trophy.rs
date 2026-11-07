//! Only Pemaret's fixed fish exchange has this inventory-owned planner.
use super::*;

impl NpcRuleCatalog {
    /// The transaction supplies locked fish counts; other native guards stay refused.
    pub fn plan_marlin_items(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        if input.npc != "oteryn:npc.pemaret" {
            return Err(NpcRuleError::Invalid("not Pemaret".into()));
        }
        if self
            .program("oteryn:quest.marlin_trophy_quest")
            .is_none_or(|program| {
                program
                    .document
                    .rules
                    .iter()
                    .flat_map(|rule| &rule.guards)
                    .any(|guard| {
                        !matches!(
                            guard,
                            NpcRuleGuard::Storage { .. } | NpcRuleGuard::Item { .. }
                        )
                    })
            })
        {
            return Err(NpcRuleError::MissingNativeResolution(
                "Pemaret guard".into(),
            ));
        }
        let plan = evaluate::plan(self, input, &BTreeMap::new(), false)?;
        if plan.as_ref().is_some_and(|plan| {
            plan.quest != "oteryn:quest.marlin_trophy_quest"
                || plan.effects.iter().any(|effect| {
                    !matches!(
                        effect,
                        NpcRuleEffect::Set { .. }
                            | NpcRuleEffect::SetTopic { .. }
                            | NpcRuleEffect::TakeItem { .. }
                            | NpcRuleEffect::GiveItem { .. }
                    )
                })
        }) {
            return Err(NpcRuleError::MissingNativeResolution(
                "Pemaret effect".into(),
            ));
        }
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = include_str!(
        "../../../../../../content/dialogues/quest-rules/quest-batch3-a-marlin_trophy.json"
    );

    #[test]
    fn pemaret_old_source_requires_exact_custody() -> Result<(), Box<dyn std::error::Error>> {
        NpcRuleCatalog::from_json_documents(&[SOURCE])?;
        let mut document: serde_json::Value = serde_json::from_str(SOURCE)?;
        document["source_files"][0]["sha256"] = serde_json::json!("0".repeat(64));
        assert!(
            NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&document)?]).is_err()
        );
        document["source_files"] = serde_json::json!([]);
        assert!(
            NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&document)?]).is_err()
        );
        Ok(())
    }

    #[test]
    fn pemaret_source_revision_cannot_authorize_another_quest()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut document: serde_json::Value = serde_json::from_str(SOURCE)?;
        let quest = "oteryn:quest.exchange_quests";
        document["quest"] = serde_json::json!(quest);
        for track in document["tracks"].as_array_mut().ok_or("tracks")? {
            track["quest"] = serde_json::json!(quest);
        }
        for rule in document["rules"].as_array_mut().ok_or("rules")? {
            rule["quest"] = serde_json::json!(quest);
        }
        assert!(
            NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&document)?]).is_err()
        );
        Ok(())
    }
}
