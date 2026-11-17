//! Shared rules for the existing reset/event owner. Channel effects remain in map/daily.
use crate::daily::DailyWorldChanges;

/// Authored evidence/hold inventory. Held source recipes are never selection inputs.
pub const CATALOGUE_JSON: &str =
    include_str!("../../../../content/encounters/world-changes/owner-catalogue.json");

/// Resource bookkeeping only. A quest must fence and debit its resource in the same
/// transaction; a tally alone never grants transport, rewards or activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionKind {
    Coal,
    Medicine,
    Votes,
}

impl ContributionKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Coal => "coal",
            Self::Medicine => "medicine",
            Self::Votes => "votes",
        }
    }
}

pub fn add_contribution(tally: u64, amount: u32) -> Option<u64> {
    (amount > 0)
        .then(|| tally.checked_add(u64::from(amount)))
        .flatten()
}

/// Annual UTC calendar window, inclusive start and exclusive end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeasonalWindow {
    pub start: (u8, u8),
    pub end: (u8, u8),
}

impl SeasonalWindow {
    pub fn contains(self, month: u8, day: u8) -> bool {
        let valid = |(m, d): (u8, u8)| {
            (1..=12).contains(&m)
                && d > 0
                && d <= match m {
                    2 => 29,
                    4 | 6 | 9 | 11 => 30,
                    _ => 31,
                }
        };
        let date = (month, day);
        valid(date)
            && valid(self.start)
            && valid(self.end)
            && if self.start <= self.end {
                self.start <= date && date < self.end
            } else {
                self.start <= date || date < self.end
            }
    }
}

/// Only donor-backed, physically composed daily changes can qualify an active gate.
/// Unknown/held keys fail closed, including the requested incomplete quest titles.
pub fn is_active(day: &DailyWorldChanges, key: &str) -> bool {
    if day.reset_epoch == 0 {
        return false;
    }
    if let Some((key, stage)) = key.rsplit_once('@') {
        return crate::counter::CounterKind::from_key(key).is_some_and(|kind| {
            counter_admitted(kind)
                && (kind != crate::counter::CounterKind::TheirMastersVoice
                    || day.their_masters_voice)
                && day.world_counters
                && stage
                    .parse::<u8>()
                    .is_ok_and(|stage| stage > 0 && day.counter_stages[kind.index()] == stage)
        });
    }
    if key != "their_masters_voice"
        && let Some(kind) = crate::counter::CounterKind::from_key(key)
    {
        if !day.world_counters || !counter_admitted(kind) {
            return false;
        }
        return match kind {
            crate::counter::CounterKind::FireEarth => day.counter_stages[kind.index()] == 1,
            crate::counter::CounterKind::FireFeathered => day.counter_stages[kind.index()] == 3,
            crate::counter::CounterKind::Overhunting => day.counter_stages[kind.index()] == 2,
            crate::counter::CounterKind::DemonWars => {
                (1..=5).contains(&day.counter_stages[kind.index()])
            }
            _ => (1..=3).contains(&day.counter_stages[kind.index()]),
        };
    }
    match key {
        "thawing" => day.thawing,
        "their_masters_voice" => day.their_masters_voice,
        "oriental_trader" => day.oriental_trader.is_some(),
        "grimvale" => day.grimvale_full_moon,
        "stampede" => day.stampede,
        // Preserve main's independently admitted, epoch-bound static layers.
        "fury_gates" => day.fury_gates.is_some(),
        key if crate::daily::FURY_LAYERS.contains(&key) => {
            crate::bindings::NativeBindings::compile(day)
                .is_ok_and(|bindings| bindings.admits_entry_layer(key))
        }
        // Flower Month's waves are still held; the NPC alone is not completion.
        _ => false,
    }
}

/// Content admission is independent of a persisted progress counter. Held mechanics
/// can accumulate test/upgrade state but cannot produce a physical activation.
pub fn counter_admitted(kind: crate::counter::CounterKind) -> bool {
    static KEYS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(CATALOGUE_JSON) else {
            return Vec::new();
        };
        value["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|entry| entry["status"] == "QUALIFIED")
            .filter_map(|entry| entry["key"].as_str().map(str::to_owned))
            .collect()
    })
    .iter()
    .any(|key| key == kind.key())
}

/// TibiaWiki Overhunting World Change/Spoiler rev 1084984 (infobox rev 844823).
/// No warning cutoff is sourced: any daily loss below the authored threshold is dwindling;
/// reaching that threshold announces departure, while stage two reports wolves.
pub fn overhunting_guide_reply(stage: u8, white_deer_kills: u64, threshold: u64) -> &'static str {
    if stage == 2 {
        "Starving wolves are roaming the region near Ab'Dendriel. As long as they are there, no white deer will return."
    } else if white_deer_kills >= threshold {
        "Too many white deer have already been slain near Ab'Dendriel. Their population will leave the region soon."
    } else if white_deer_kills > 0 {
        "The number of white deer near Ab'Dendriel seems to be dwindling. If that continues, we will have to watch out for some starving wolves."
    } else {
        "There are white deer roaming the region near Ab'Dendriel. Don't slay too many of them, or they will leave the region."
    }
}

#[cfg(test)]
mod imported_layer_tests {
    use super::*;

    #[test]
    fn m01b_exact_fury_entry_requires_selected_city_and_committed_epoch() {
        let mut day = DailyWorldChanges::roll(1, 6, 13, |_| 0);
        day.fury_exit = "fury_gates/thais";
        day.fury_gates = Some(day.fury_exit);
        assert!(is_active(&day, "fury_gates/thais"));
        assert!(!is_active(&day, "fury_gates/venore"));
        assert!(!is_active(&day, "fury_gates/unknown"));
        day.fury_gates = None;
        assert!(
            !is_active(&day, "fury_gates/thais"),
            "retained exit is not an entrance"
        );
        day.fury_gates = Some(day.fury_exit);
        day.reset_epoch = 0;
        assert!(!is_active(&day, "fury_gates/thais"));
    }
}

#[cfg(test)]
mod overhunting_guide_tests {
    #[test]
    fn guides_report_all_four_overhunting_states() {
        use super::overhunting_guide_reply as reply;
        assert_eq!(
            reply(1, 0, 1000),
            "There are white deer roaming the region near Ab'Dendriel. Don't slay too many of them, or they will leave the region."
        );
        assert_eq!(
            reply(1, 1, 1000),
            "The number of white deer near Ab'Dendriel seems to be dwindling. If that continues, we will have to watch out for some starving wolves."
        );
        assert_eq!(
            reply(1, 1000, 1000),
            "Too many white deer have already been slain near Ab'Dendriel. Their population will leave the region soon."
        );
        assert_eq!(
            reply(2, 0, 1000),
            "Starving wolves are roaming the region near Ab'Dendriel. As long as they are there, no white deer will return."
        );
    }
}
