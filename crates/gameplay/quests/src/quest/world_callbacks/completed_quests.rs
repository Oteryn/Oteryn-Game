//! Completion prerequisites are bound to the callback occurrence, not account flags.
pub(super) fn valid(callback: &super::Callback) -> bool {
    let keys = &callback.required_completed_quests;
    keys.is_empty()
        || (keys.len() <= 16
            && callback.actor_kind == super::ActorKind::Player
            && callback.requires_world_admission
            && callback.has_personal_state()
            && !callback.reversible
            && !callback.opening_only
            && callback.native_rule.is_none()
            && keys.windows(2).all(|pair| pair[0] < pair[1])
            && keys.iter().all(|key| {
                key != &callback.quest
                    && key.starts_with("oteryn:quest.")
                    && super::super::valid_quest_key(key)
            }))
}

#[cfg(test)]
mod tests {
    use super::super::*;

    fn fixture() -> Result<serde_json::Value, String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/hero-of-rathleton/s1/entry")
            .ok_or("native prerequisite fixture")?;
        serde_json::to_value(callback).map_err(|e| e.to_string())
    }

    #[test]
    fn completed_quest_guard_is_admitted_and_changes_native_binding() -> Result<(), String> {
        let mut value = fixture()?;
        let baseline =
            parse(&serde_json::json!({"schema":1,"callbacks":[value.clone()]}).to_string())?;
        value["required_completed_quests"] =
            serde_json::json!(["oteryn:quest.the_ice_islands_quest"]);
        let guarded = parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string())?;
        assert_ne!(
            gated_binding(&baseline[0])?,
            gated_binding(&guarded[0])?,
            "completion requirements cannot be stripped from a native occurrence"
        );
        Ok(())
    }

    #[test]
    fn completed_quest_guard_rejects_unsafe_or_unbounded_prerequisites() -> Result<(), String> {
        for keys in [
            vec!["oteryn:quest.hero_of_rathleton_quest"],
            vec!["oteryn:quest.the_ice_islands_quest"; 17],
            vec!["oteryn:quest.the_ice_islands_quest"; 2],
            vec!["oteryn:item.tibia.i3031"],
            vec!["oteryn:quest. other"],
        ] {
            let mut value = fixture()?;
            value["required_completed_quests"] = serde_json::json!(keys);
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string()).is_err()
            );
        }
        for field in ["requires_world_admission", "actor_kind"] {
            let mut value = fixture()?;
            value["required_completed_quests"] =
                serde_json::json!(["oteryn:quest.the_ice_islands_quest"]);
            value[field] = if field == "actor_kind" {
                serde_json::json!("monster")
            } else {
                false.into()
            };
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string()).is_err()
            );
        }
        Ok(())
    }
}
