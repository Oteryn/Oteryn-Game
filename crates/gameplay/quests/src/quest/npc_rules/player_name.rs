//! A source-owned selector for the current Character's name, never an authored literal name.
use super::*;

pub(super) fn is_false(value: &bool) -> bool {
    !*value
}

pub(super) fn matches(
    message: &str,
    player_name: Option<&str>,
    mode: NpcKeywordMode,
) -> Result<bool, NpcRuleError> {
    match player_name.filter(|name| !name.is_empty()) {
        Some(name) => keyword::matches(message, name, mode),
        None => Ok(false),
    }
}

impl NpcRuleCatalog {
    /// A phase-specific name answer takes precedence over greeting heuristics.
    /// For example, a current name containing the word "Hi" is still an answer.
    pub fn matches_player_name_message(
        &self,
        npc: &str,
        message: &str,
        topic: i32,
        player_name: Option<&str>,
    ) -> Result<bool, NpcRuleError> {
        for rule in self
            .programs
            .iter()
            .flat_map(|p| &p.document.rules)
            .filter(|rule| {
                rule.npc == npc
                    && rule.trigger == NpcRuleTrigger::Say
                    && rule.match_player_name
                    && rule
                        .topics
                        .as_ref()
                        .is_some_and(|topics| topics.contains(&topic))
            })
        {
            let mut excluded = false;
            for word in &rule.exclude_keywords {
                excluded |= keyword::matches(message, word, NpcKeywordMode::MsgContains)?;
            }
            if !excluded && matches(message, player_name, rule.keyword_mode)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_name_selector_is_not_a_literal_or_unconditional_match() -> Result<(), NpcRuleError> {
        let source = format!("{SOURCE_PIN}data/npclib/npc.lua:13");
        let text = serde_json::json!({"schema":1,"quest":"oteryn:quest.name_fixture",
            "tracks":[{"key":"oteryn:quest-progress/name_fixture/stage","quest":"oteryn:quest.name_fixture",
                "initial":0,"min":0,"max":1,"sources":[source]}],
            "rules":[{"key":"oteryn:quest-npc/name_fixture/accept","quest":"oteryn:quest.name_fixture",
                "npc":"oteryn:npc.fixture","trigger":"say","match_player_name":true,"topics":[3],
                "exclude_keywords":["yes"],"guards":[],"reply":["Accepted."],
                "effects":[{"kind":"set","track":"oteryn:quest-progress/name_fixture/stage","value":1}],"sources":[source]}]
        }).to_string();
        let source = NpcRuleCatalog::from_json_documents(&[&text])?;
        assert!(source.programs()[0].document.rules[0].keywords.is_empty());
        let tracks = BTreeMap::new();
        let inventory = NpcInventorySnapshot::default();
        let outfits = BTreeMap::new();
        for (name, message, topic, expected) in [
            (None, "Ada Scholar", 3, false),
            (Some("Ada Scholar"), "Other Scholar", 3, false),
            (Some("Ada Scholar"), "Ada Scholar", 2, false),
            (Some("Ada Scholar"), "Ada Scholar", 3, true),
            (Some("Hi Scholar"), "HI SCHOLAR", 3, true),
            (Some("Ada Scholar"), "xAda Scholar", 3, false),
            (Some("Ada Scholar"), "yes Ada Scholar", 3, false),
        ] {
            let input = NpcRuleInput {
                npc: "oteryn:npc.fixture",
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: name,
                topic,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &outfits,
            };
            assert_eq!(source.plan_storage(&input)?.is_some(), expected);
            assert_eq!(
                source.matches_player_name_message(input.npc, message, topic, name)?,
                expected
            );
        }
        // Existing documents keep their original serialized shape and digest inputs.
        let mut legacy = source.programs()[0].document.rules[0].clone();
        legacy.match_player_name = false;
        assert!(
            serde_json::to_value(&legacy)
                .map_err(|e| NpcRuleError::Json(e.to_string()))?
                .get("match_player_name")
                .is_none()
        );
        Ok(())
    }
}
