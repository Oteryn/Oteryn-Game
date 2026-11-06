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
    match key {
        "thawing" => day.thawing,
        "their_masters_voice" => day.their_masters_voice,
        "oriental_trader" => day.oriental_trader.is_some(),
        "grimvale" => day.grimvale_full_moon,
        "stampede" => day.stampede,
        "fury_gates" => day.fury_gates.is_some_and(|v| v < 10),
        "nightmare_isles" => day.nightmare_isles.is_some_and(|v| v < 3),
        // Flower Month's waves are still held; the NPC alone is not completion.
        _ => false,
    }
}
