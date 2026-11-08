//! Source-qualified learning requirements use immutable native quest completion.
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rule {
    family: u8,
    tier: u8,
    any_completed_quests: Vec<String>,
    sources: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    rules: Vec<Rule>,
}
fn parse(text: &str) -> Result<Vec<Rule>, String> {
    let doc: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if doc.schema != 1 || doc.rules.len() > 57 {
        return Err("imbuement knowledge catalogue bounds".into());
    }
    let mut seen = BTreeSet::new();
    for rule in &doc.rules {
        let quests: BTreeSet<_> = rule.any_completed_quests.iter().collect();
        if oteryn_gameplay_items::imbuement::imbuement_recipe(rule.family, rule.tier).is_none()
            || !seen.insert((rule.family, rule.tier))
            || quests.len() != rule.any_completed_quests.len()
            || !(1..=8).contains(&quests.len())
            || quests.iter().any(|key| {
                !key.starts_with("oteryn:quest.")
                    || !oteryn_gameplay_quests::quest::valid_quest_key(key)
            })
            || rule.sources.is_empty()
            || rule.sources.len() > 8
            || rule
                .sources
                .iter()
                .any(|source| source.is_empty() || source.len() > 1024)
        {
            return Err("invalid imbuement knowledge rule".into());
        }
    }
    Ok(doc.rules)
}
pub(super) fn requirement(family: u8, tier: u8) -> Result<Option<&'static Rule>, String> {
    static RULES: OnceLock<Result<Vec<Rule>, String>> = OnceLock::new();
    let rules = RULES
        .get_or_init(|| {
            parse(include_str!(
                "../../../../../../content/quests/imbuement-knowledge.json"
            ))
        })
        .as_ref()
        .map_err(Clone::clone)?;
    Ok(rules
        .iter()
        .find(|rule| rule.family == family && rule.tier == tier))
}
impl Rule {
    pub(super) fn permits(&self, completed: impl Fn(&str) -> bool) -> bool {
        self.any_completed_quests
            .iter()
            .any(|quest| completed(quest))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::json!({"schema":1,"rules":[{"family":1,"tier":3,"any_completed_quests":["oteryn:quest.heart_of_destruction_quest"],"sources":["canonical:test-source"]}]})
    }
    #[test]
    fn knowledge_uses_exact_completed_identity_and_keeps_basic_unrestricted() -> Result<(), String>
    {
        let rows = parse(&fixture().to_string())?;
        assert!(!rows[0].permits(|_| false));
        assert!(!rows[0].permits(|key| key == "oteryn:quest.unrelated"));
        assert!(rows[0].permits(|key| key == "oteryn:quest.heart_of_destruction_quest"));
        assert!(requirement(1, 1)?.is_none());
        Ok(())
    }
    #[test]
    fn knowledge_rejects_ambiguous_unbounded_unknown_or_unwitnessed_rules() -> Result<(), String> {
        for case in 0..9 {
            let mut doc = fixture();
            match case {
                0 => doc["schema"] = 2.into(),
                1 => doc["rules"][0]["family"] = 255.into(),
                2 => doc["rules"][0]["tier"] = 4.into(),
                3 => doc["rules"][0]["any_completed_quests"] = serde_json::json!([]),
                4 => {
                    doc["rules"][0]["any_completed_quests"] =
                        serde_json::json!(["canary:quest/source"])
                }
                5 => doc["rules"][0]["sources"] = serde_json::json!([]),
                6 => {
                    let row = doc["rules"][0].clone();
                    doc["rules"].as_array_mut().ok_or("rules")?.push(row);
                }
                7 => {
                    doc["rules"][0]["any_completed_quests"] = serde_json::json!([
                        "oteryn:quest.heart_of_destruction_quest",
                        "oteryn:quest.heart_of_destruction_quest"
                    ])
                }
                _ => doc["rules"] = serde_json::Value::Array(vec![doc["rules"][0].clone(); 58]),
            }
            assert!(parse(&doc.to_string()).is_err(), "case {case}");
        }
        Ok(())
    }
}
