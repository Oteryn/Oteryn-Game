//! Bounded personal quest predicates for existing atomic once-only RewardClaims.
//! Conditions are read under the same character-root lock as the reward MINT.
use super::{QuestStateCatalogue, valid_quest_key};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub claim: String,
    pub quest: String,
    pub conditions: Vec<Condition>,
    pub sources: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    gates: Vec<Gate>,
}

pub fn parse(raw: &str) -> Result<Vec<Gate>, String> {
    let document: Document = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.gates.len() > 128 {
        return Err("reward claim gate schema/bound".into());
    }
    let mut claims = BTreeSet::new();
    for gate in &document.gates {
        if !gate.claim.starts_with("oteryn:reward-claim.")
            || !valid_quest_key(&gate.claim)
            || !valid_quest_key(&gate.quest)
            || !claims.insert(&gate.claim)
            || gate.conditions.is_empty()
            || gate.conditions.len() > 8
            || gate.sources.is_empty()
            || gate.sources.len() > 8
            || gate.sources.iter().any(|s| s.is_empty() || s.len() > 512)
        {
            return Err("invalid/duplicate reward claim gate".into());
        }
        let mut tracks = BTreeSet::new();
        for condition in &gate.conditions {
            if !valid_quest_key(&condition.track)
                || !tracks.insert(&condition.track)
                || condition.minimum > condition.maximum
            {
                return Err("invalid/duplicate reward claim condition".into());
            }
        }
    }
    Ok(document.gates)
}

pub fn load() -> Result<Vec<Gate>, String> {
    parse(include_str!(
        "../../../../../content/quests/reward-claim-gates.json"
    ))
}

pub fn validate_catalogue(catalogue: &QuestStateCatalogue) -> Result<(), String> {
    for gate in load()? {
        for condition in &gate.conditions {
            let track = catalogue
                .track(&condition.track)
                .ok_or("unknown reward claim gate track")?;
            if track.quest != gate.quest
                || condition.minimum < track.min
                || condition.maximum > track.max
            {
                return Err("foreign/out-of-bounds reward claim condition".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reward_claim_gates_reject_duplicates_empty_and_invalid_bounds() -> Result<(), String> {
        let gate = serde_json::json!({"claim":"oteryn:reward-claim.kv.test","quest":"oteryn:quest.test","conditions":[{"track":"oteryn:quest-progress/test/contact","minimum":1,"maximum":1}],"sources":["owner:selected-test-rule"]});
        let document = |gates| serde_json::json!({"schema":1,"gates":gates}).to_string();
        assert_eq!(parse(&document(vec![gate.clone()]))?.len(), 1);
        assert!(parse(&document(vec![gate.clone(), gate.clone()])).is_err());
        let mut bad = gate.clone();
        bad["conditions"] = serde_json::json!([]);
        assert!(parse(&document(vec![bad])).is_err());
        let mut bad = gate.clone();
        bad["conditions"][0]["maximum"] = 0.into();
        assert!(parse(&document(vec![bad])).is_err());
        let mut bad = gate.clone();
        bad["conditions"][0]["track"] = "canary:foreign".into();
        assert!(parse(&document(vec![bad])).is_err());
        Ok(())
    }
}
