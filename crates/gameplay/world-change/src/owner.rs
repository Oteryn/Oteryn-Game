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
    SteamshipCoal,
    Votes,
}

impl ContributionKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Coal => "coal",
            Self::Medicine => "medicine",
            Self::SteamshipCoal => "steamship_coal",
            Self::Votes => "votes",
        }
    }
}

pub fn add_contribution(tally: u64, amount: u32) -> Option<u64> {
    (amount > 0)
        .then(|| tally.checked_add(u64::from(amount)))
        .flatten()
}

pub const COAL_ITEM: &str = "oteryn:item.tibia.i12600";

/// Exact station witnesses from the retained WVAR Awash recipe.
pub fn dronk_pump(position: (u16, u16, i8), appearance: u16) -> bool {
    matches!(
        (position, appearance),
        ((32558, 32007, 9), 12666) | ((32560, 32014, 9), 12667) | ((32567, 32024, 9), 12666)
    )
}

pub fn junkar(npc: &str) -> bool {
    matches!(
        npc,
        "oteryn:npc.junkar_mines" | "oteryn:npc.junkar_robsons" | "oteryn:npc.junkar_thais"
    )
}

/// The prior day's coal total across all three Dronk pumps (TibiaWiki rev 1149364).
pub const fn awash_at_save(coal: u64) -> bool {
    coal >= 202
}

/// Every qualifying coal total funds the single saved day that follows it.
pub const fn steamship_at_save(coal: u64) -> bool {
    coal >= 200
}

/// The accepted cure threshold in WPKG-CONTRACT-NOTES.md applies at the next save.
pub const fn swamp_epidemic_at_save(pouches: u64) -> bool {
    pouches < 100
}

pub const FEVERISH_CITIZENS_RAID: &str = "raid:venore.feverishcitizens";

/// The epidemic accelerates the existing raid. The ordinary cadence is sourced
/// from Crystal data-global/raids/raids.xml:181 and stays in the raid catalogue.
pub const fn feverish_raid_interval(day: &DailyWorldChanges) -> Option<u32> {
    if day.swamp_epidemic {
        Some(30 * 60)
    } else {
        None
    }
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

/// Evaluate admission gates from the persisted daily decision or live counter phase.
/// Each supported key has an explicit rule.
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
        "awash" => day.awash,
        "thawing" => day.thawing,
        "their_masters_voice" => day.their_masters_voice,
        "oriental_trader" => day.oriental_trader.is_some(),
        "grimvale" => day.grimvale_full_moon,
        "stampede" => day.stampede,
        "steamship" => day.steamship_open,
        "swamp_fever" => day.swamp_epidemic,
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
mod contribution_tests {
    use super::*;

    #[test]
    fn steamship_needs_a_fresh_qualifying_total_each_save() {
        for (epoch, coal, active) in [
            (1, 199, false),
            (2, 200, true),
            (3, 201, true),
            (4, 0, false),
        ] {
            let mut day = DailyWorldChanges::roll(epoch, 11, 17, |_| 0);
            day.steamship_open = steamship_at_save(coal);
            assert_eq!(is_active(&day, "steamship"), active);
        }
    }

    #[test]
    fn swamp_fever_cures_for_one_day_and_changes_raid_interval() {
        for (epoch, pouches, epidemic) in [
            (1, 99, true),
            (2, 100, false),
            (3, 101, false),
            (4, 0, true),
        ] {
            let mut day = DailyWorldChanges::roll(epoch, 11, 17, |_| 0);
            day.swamp_epidemic = swamp_epidemic_at_save(pouches);
            assert_eq!(is_active(&day, "swamp_fever"), epidemic);
            assert_eq!(feverish_raid_interval(&day), epidemic.then_some(1800));
        }
    }

    #[test]
    fn awash_uses_only_the_retiring_days_coal() {
        for (coal, active) in [(0, false), (201, false), (202, true), (203, true)] {
            let mut day = DailyWorldChanges::roll(1, 11, 17, |_| 0);
            day.awash = awash_at_save(coal);
            assert_eq!(is_active(&day, "awash"), active);
        }
        let mut next = DailyWorldChanges::roll(2, 11, 18, |_| 0);
        next.awash = awash_at_save(0);
        assert!(!is_active(&next, "awash"));
    }
}
