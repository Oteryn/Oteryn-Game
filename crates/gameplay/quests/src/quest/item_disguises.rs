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
    #[serde(default)]
    pub requires: Vec<super::reward_callbacks::TrackRequirement>,
    #[serde(default)]
    pub activation: Option<Position>,
    pub restore: Option<Position>,
}
pub(super) fn valid(rules: &[Rule]) -> bool {
    let mut keys = std::collections::BTreeSet::new();
    rules.len() <= 32
        && rules.iter().all(|rule| {
            rule.key.len() <= 128
                && rule.key.starts_with("quest.")
                && keys.insert(&rule.key)
                && rule.item.starts_with("oteryn:item.tibia.i")
                && rule.item.len() <= 128
                && !rule.use_text.is_empty()
                && rule.use_text.len() <= 512
                && (rule.restore.is_none() || !rule.restore_text.is_empty())
                && rule.restore_text.len() <= 512
                && rule.use_effect != 0
                && rule.speech_type != 0
                && rule.appearance != 0
                && rule.track.starts_with("oteryn:quest-progress/")
                && rule.track.len() <= 128
                && !rule.sources.is_empty()
                && rule.sources.len() <= 4
                && rule.sources.iter().all(|source| {
                    (source.starts_with("crystal:") || source.starts_with("wiki:"))
                        && source.len() <= 512
                })
                && rule
                    .restore
                    .is_none_or(|position| (-15..=0).contains(&position.floor))
                && rule
                    .activation
                    .is_none_or(|position| (-15..=0).contains(&position.floor))
                && rule.requires.len() <= 8
                && rule.requires.iter().all(|required| {
                    required.track.starts_with("oteryn:quest-progress/")
                        && required.track.len() <= 128
                        && required.minimum <= required.maximum
                })
        })
}

/// Shared source catalogue; paced movement must not parse content on every step.
pub fn rules() -> Option<&'static [Rule]> {
    static RULES: std::sync::OnceLock<Result<Vec<Rule>, String>> = std::sync::OnceLock::new();
    RULES
        .get_or_init(|| {
            let mut rules = super::reward_callbacks::item_disguises()?;
            rules.extend(super::last_creep_native::disguises());
            if !valid(&rules) {
                return Err("invalid combined item disguise rules".into());
            }
            Ok(rules)
        })
        .as_ref()
        .ok()
        .map(Vec::as_slice)
}
