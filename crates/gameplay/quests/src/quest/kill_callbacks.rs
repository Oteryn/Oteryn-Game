//! Source creature deaths bind existing typed quest transitions to native damaging players.
use super::{QuestEffectKind, QuestStateCatalogue};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KillCallback {
    pub creature: String,
    pub transition: String,
    pub sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    callbacks: Vec<KillCallback>,
}

fn parse(source: &str) -> Result<Vec<KillCallback>, String> {
    let document: Document = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let mut seen = BTreeSet::new();
    if document.schema != 1 || document.callbacks.len() > 256 {
        return Err("invalid creature callback document".into());
    }
    for callback in &document.callbacks {
        let slug = callback
            .creature
            .strip_prefix("oteryn:creature.")
            .unwrap_or("");
        if slug.is_empty()
            || slug.len() > 128
            || !slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || !super::valid_quest_key(&callback.transition)
            || callback.sources.is_empty()
            || callback.sources.iter().any(String::is_empty)
            || !seen.insert((&callback.creature, &callback.transition))
        {
            return Err("invalid creature callback binding".into());
        }
    }
    Ok(document.callbacks)
}

pub fn load() -> Result<Vec<KillCallback>, String> {
    parse(include_str!(
        "../../../../../content/quests/shared-kill-callbacks.json"
    ))
}

pub fn validate(catalogue: &QuestStateCatalogue) -> Result<(), String> {
    for callback in load()? {
        let transition = catalogue
            .transition(&callback.transition)
            .ok_or("unknown creature quest transition")?;
        if transition.experience.is_some()
            || transition.effects.is_empty()
            || transition
                .effects
                .iter()
                .any(|e| matches!(e.effect, QuestEffectKind::Native(_)))
        {
            return Err("creature callback must use ordinary typed progress effects".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creature_bindings_reject_duplicates_unknown_fields_and_invalid_identity()
    -> Result<(), String> {
        let callback = serde_json::json!({"creature":"oteryn:creature.renegade_orc","transition":"oteryn:quest-transition/death","sources":["source:1:sha256:witness"]});
        let document =
            |callbacks| serde_json::json!({"schema":1,"callbacks":callbacks}).to_string();
        assert!(parse(&document(vec![callback.clone()])).is_ok());
        assert!(parse(&document(vec![callback.clone(), callback.clone()])).is_err());
        for (field, value) in [
            ("creature", "oteryn:creature."),
            ("transition", "invalid"),
            ("unsupported", "field"),
        ] {
            let mut invalid = callback.clone();
            invalid[field] = value.into();
            assert!(parse(&document(vec![invalid])).is_err());
        }
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        validate(lowered.catalogue())?;
        Ok(())
    }
}
