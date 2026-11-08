//! Main source departure callbacks may restore passages without a reversible lever.
use super::Callback;

pub(super) fn restores_only_passages(callback: &Callback) -> bool {
    !callback.passages.is_empty() && callback.effects.is_empty()
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn source_departure_restores_passages_without_durable_effects() -> Result<(), String> {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/world-callbacks/l6-world-puzzles.json"
        ))
        .map_err(|e| e.to_string())?;
        let source = document["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/l6/paradox-first-exit-32478")
            .ok_or("source passage departure")?;
        let wire =
            |c: &serde_json::Value| serde_json::json!({"schema": 1, "callbacks": [c]}).to_string();
        parse(&wire(source))?;
        for mutation in [
            "empty_passages",
            "teleport",
            "storage",
            "experience",
            "reward",
        ] {
            let mut changed = source.clone();
            match mutation {
                "empty_passages" => changed["passages"] = serde_json::json!([]),
                "teleport" => {
                    changed["teleport"] = serde_json::json!({"x":32478,"y":31906,"floor":-7})
                }
                "storage" => {
                    changed["storage"] = serde_json::json!([{"track":"oteryn:quest-progress/forbidden","minimum":-1,"maximum":-1,"set":1}])
                }
                "experience" => changed["experience"] = 1.into(),
                _ => {
                    changed["effects"] = serde_json::json!({"item_reward":{"item":"oteryn:item.tibia.i3031","count":1,"contents":[]}})
                }
            }
            assert!(parse(&wire(&changed)).is_err(), "{mutation}");
        }
        Ok(())
    }
}
