//! Jorge's source-ordered, repeatable20-token exchange catalogue.
use super::npc_rules::NpcRuleError;
use serde::Deserialize;
use std::collections::BTreeSet;

pub const NPC: &str = "oteryn:npc.jorge";
pub const TOKEN: u16 = 19083;
pub const COST: u32 = 20;

const DOCUMENT: &str =
    include_str!("../../../../../content/dialogues/quest-native/hunting-for-tokens.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    quest: String,
    npc: String,
    source: Source,
    input: Input,
    #[serde(rename = "repeat")]
    repeat_policy: String,
    offers: Vec<Offer>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    revision: String,
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    id: u16,
    count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    id: u16,
    name: String,
}

fn parse(document: &str) -> Result<Vec<(u16, String)>, NpcRuleError> {
    let invalid = || NpcRuleError::Invalid("Jorge source exchange table".into());
    let document: Document = serde_json::from_str(document).map_err(|_| invalid())?;
    if document.schema != "OTERYN_RAID_TOKEN_EXCHANGES/v1"
        || document.quest != "oteryn:quest.hunting_for_tokens"
        || document.npc != NPC
        || document.source.revision != "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
        || document.source.path != "data-global/npc/jorge.lua"
        || document.source.sha256
            != "127ff40ceb3fd0612663cbd952176840977e1ba9b9f22534ba52b2d059963db0"
        || document.input.id != TOKEN
        || document.input.count != COST
        || document.repeat_policy != "unlimited"
        || document.offers.len() != 37
    {
        return Err(invalid());
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for offer in &document.offers {
        if offer.id == 0
            || offer.id == TOKEN
            || !ids.insert(offer.id)
            || offer.name.is_empty()
            || offer.name.len() > 80
            || !offer.name.is_ascii()
            || offer.name.trim() != offer.name
            || !names.insert(offer.name.to_ascii_lowercase())
        {
            return Err(invalid());
        }
    }
    Ok(document
        .offers
        .into_iter()
        .map(|offer| (offer.id, offer.name))
        .collect())
}

/// Preserve source order: a message naming two replicas selects the first source match.
pub fn offers() -> Result<Vec<(u16, String)>, NpcRuleError> {
    parse(DOCUMENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jorge_source_has_all37_repeatable_replicas_at_twenty_silver_tokens()
    -> Result<(), Box<dyn std::error::Error>> {
        let offers = offers()?;
        assert_eq!(offers.len(), 37);
        assert_eq!(offers.first(), Some(&(19151, "Abacus".into())));
        assert_eq!(offers.last(), Some(&(43510, "Tibiora's Box".into())));
        let original: serde_json::Value = serde_json::from_str(DOCUMENT)?;
        for (field, value) in [
            ("count", serde_json::json!(19)),
            ("id", serde_json::json!(19082)),
        ] {
            let mut changed = original.clone();
            changed["input"][field] = value;
            assert!(parse(&serde_json::to_string(&changed)?).is_err());
        }
        let mut changed = original.clone();
        changed["repeat"] = serde_json::json!("once");
        assert!(parse(&serde_json::to_string(&changed)?).is_err());
        changed = original.clone();
        changed["offers"][1]["id"] = changed["offers"][0]["id"].clone();
        assert!(parse(&serde_json::to_string(&changed)?).is_err());
        changed = original;
        changed["source"]["sha256"] = serde_json::json!("0".repeat(64));
        assert!(parse(&serde_json::to_string(&changed)?).is_err());
        Ok(())
    }
}
