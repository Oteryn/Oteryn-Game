//! Current immutable Channel placements can gate stateless world callbacks.
use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectCondition {
    pub position: Position,
    pub appearance: u16,
}

pub(super) fn valid(callback: &Callback) -> bool {
    let conditions = &callback.object_conditions;
    if conditions.is_empty() {
        return true;
    }
    let unique: BTreeSet<_> = conditions
        .iter()
        .map(|c| (c.position, c.appearance))
        .collect();
    conditions.len() <= MAX_EFFECTS
        && unique.len() == conditions.len()
        && callback.storage.is_empty()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.minimum_level.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.native_rule.is_none()
        && conditions
            .iter()
            .all(|c| (-15..=0).contains(&c.position.floor) && c.appearance != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_object_conditions_are_bounded_stateless_and_source_positions() -> Result<(), String> {
        let document: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        let mut callback = document["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/behemoth")
            .ok_or("behemoth")?
            .clone();
        callback["object_conditions"] = serde_json::json!([
            {"position":{"x":32802,"y":31584,"floor":-1},"appearance":2772},
            {"position":{"x":32803,"y":31584,"floor":-1},"appearance":2773}
        ]);
        let encode = |c| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(super::super::parse(&encode(callback.clone())).is_ok());
        for case in 0..5 {
            let mut invalid = callback.clone();
            match case {
                0 => invalid["object_conditions"][0]["appearance"] = 0.into(),
                1 => invalid["object_conditions"][0]["position"]["floor"] = (-16).into(),
                2 => invalid["object_conditions"][1] = invalid["object_conditions"][0].clone(),
                3 => invalid["object_conditions"] = serde_json::json!((0..=MAX_EFFECTS).map(|x|
                    serde_json::json!({"position":{"x":x,"y":1,"floor":-1},"appearance":2772})).collect::<Vec<_>>()),
                _ => invalid["storage"] = serde_json::json!([{"track":"oteryn:quest-track/fixture", "minimum":-1,"maximum":1,"set":1}]),
            }
            assert!(
                super::super::parse(&encode(invalid)).is_err(),
                "condition bound{case}"
            );
        }
        Ok(())
    }
}
