//! Exact donor door storages resolved to their already-owned native track identities.
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
pub(super) struct Binding {
    pub action: u16,
    pub track: String,
    position: [u16; 3],
    #[serde(default)]
    pub minimum: Option<i64>,
}
#[derive(Deserialize)]
struct Bindings {
    bindings: Vec<Binding>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Aliases {
    schema: u8,
    aliases: Vec<Alias>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Alias {
    position: [u16; 3],
    action: u16,
    from: String,
    to: String,
    minimum: i64,
    sources: Vec<String>,
    rule: String,
}

pub(super) fn load(source: &str, aliases: &str) -> Result<Vec<Binding>, String> {
    let mut bindings: Bindings = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let aliases: Aliases = serde_json::from_str(aliases).map_err(|e| e.to_string())?;
    if aliases.schema != 1 || aliases.aliases.len() > 512 {
        return Err("door alias bounds".into());
    }
    let mut seen = BTreeSet::new();
    for alias in aliases.aliases {
        if !seen.insert((alias.position, alias.action))
            || alias.position[2] > 15
            || alias.action == 0
            || !oteryn_gameplay_quests::quest::valid_quest_key(&alias.to)
            || alias.from == alias.to
            || alias.sources.is_empty()
            || alias.sources.len() > 8
            || alias.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
            || alias.rule.is_empty()
            || alias.rule.len() > 1024
        {
            return Err("door alias source identity".into());
        }
        let mut matching = bindings.bindings.iter_mut().filter(|b| {
            b.position == alias.position && b.action == alias.action && b.track == alias.from
        });
        let binding = matching
            .next()
            .ok_or("door alias no longer matches source")?;
        if matching.next().is_some() {
            return Err("ambiguous source door".into());
        }
        binding.track = alias.to;
        binding.minimum = Some(alias.minimum);
    }
    Ok(bindings.bindings)
}

#[cfg(test)]
mod tests {
    use super::*;
    const SOURCE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/world-authoring/samples/quest-door-bindings.json"
    ));
    const ALIASES: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/world-authoring/quest-door-track-aliases.json"
    ));
    #[test]
    fn exact_source_alias_selects_the_existing_native_storage() -> Result<(), String> {
        let bindings = load(SOURCE, ALIASES)?;
        let outfox = bindings
            .iter()
            .find(|b| b.action == 41492)
            .ok_or("Outfox door")?;
        assert_eq!(
            outfox.track,
            "oteryn:quest-progress/quest/u8_1/to_outfox_afox_quest"
        );
        assert_eq!(outfox.minimum, Some(1));
        let sam = bindings
            .iter()
            .find(|b| b.action == 40603)
            .ok_or("Sam door")?;
        assert_eq!(sam.minimum, None);
        let mut changed: serde_json::Value =
            serde_json::from_str(ALIASES).map_err(|e| e.to_string())?;
        changed["aliases"][0]["from"] = "oteryn:quest-progress/another-source".into();
        assert!(load(SOURCE, &changed.to_string()).is_err());
        Ok(())
    }
}
