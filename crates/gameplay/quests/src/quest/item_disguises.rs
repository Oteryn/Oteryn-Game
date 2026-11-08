//! Source-bound temporary item looks; eligibility reads never advance quest progress.
use super::world_callbacks::Position;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub key: String,
    pub sources: Vec<String>,
    pub item: String,
    pub use_text: String,
    pub restore_text: String,
    pub use_effect: u16,
    pub speech_type: u8,
    pub appearance: u16,
    pub track: String,
    pub required: i64,
    pub restore: Position,
}
pub(super) fn valid(rules: &[Rule]) -> bool {
    let mut items = std::collections::BTreeSet::new();
    rules.len() <= 32
        && rules.iter().all(|rule| {
            rule.key.len() <= 128
                && rule.key.starts_with("quest.")
                && items.insert(&rule.item)
                && rule.item == format!("oteryn:item.tibia.i{}", rule.appearance)
                && !rule.use_text.is_empty()
                && rule.use_text.len() <= 512
                && !rule.restore_text.is_empty()
                && rule.restore_text.len() <= 512
                && rule.use_effect != 0
                && rule.speech_type != 0
                && rule.appearance != 0
                && rule.track.starts_with("oteryn:quest-progress/")
                && rule.track.len() <= 128
                && !rule.sources.is_empty()
                && rule.sources.len() <= 4
                && rule
                    .sources
                    .iter()
                    .all(|source| source.starts_with("crystal:") && source.len() <= 512)
                && (-15..=0).contains(&rule.restore.floor)
        })
}

/// Shared source catalogue; paced movement must not parse content on every step.
pub fn rules() -> Option<&'static [Rule]> {
    static RULES: std::sync::OnceLock<Result<Vec<Rule>, String>> = std::sync::OnceLock::new();
    RULES
        .get_or_init(super::reward_callbacks::item_disguises)
        .as_ref()
        .ok()
        .map(Vec::as_slice)
}
