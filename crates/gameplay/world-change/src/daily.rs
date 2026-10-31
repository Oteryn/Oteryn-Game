//! W05b: the daily world changes the server save rolls for the next day.
//!
//! Donor: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8, whose world changes
//! are `onStartup` global events that run once per daily server save. Each rule below cites the
//! donor script; [`DailyWorldChanges::npcs`] lists the effects this repository applies to the
//! live map. The chance draws come from the caller, keyed on the committed reset epoch, so a
//! restart within the same day rolls the same day again without a second state owner.
//!
//! Not applied (no live-map owner yet; the roll is still taken so the day is fixed):
//! - Thawing places Ice Flowers (13842) on 14 tiles (`thawing.lua:4-19,34-36`): the live map
//!   has no owner that adds a volatile item to a tile (`BundleFacts::added_entry` returns
//!   `None` for a volatile item, `crates/gameplay/world/src/map/facts.rs`).
//! - Their Master's Voice turns 30% of the fungus 12065 on floor 9 into 12059..=12063
//!   (`their_masters_voice.lua:5-14`): 12065 is a Terrain ground in this bundle
//!   (`content/world/terrain/terrain-03000-03499.json`), and the map rotation owner
//!   (`BundleFacts::rotate_base`) changes only fixed item placements.
//! - Oriental Trader loads its ship map and Yasir stands on it (`oriental_trader.lua:61-65`):
//!   no world-change map layer import exists, so Yasir would stand on a tile the map lacks.
//! - Grimvale (`grimvale_respawn_event.lua:21-36`) and the Flower Month dryad waves
//!   (`raids/world_changes/flower_month.lua:28-55`) change monster spawns.

/// The donor `math.random(100) <= percent`, from one uniform 64-bit draw.
fn percent_roll(draw: u64, percent: u64) -> bool {
    (u128::from(draw) * 100) >> 64 < u128::from(percent)
}

/// One of `count` choices, from one uniform 64-bit draw (the donor `math.random(count)`).
fn choice(draw: u64, count: u64) -> u64 {
    u64::try_from((u128::from(draw) * u128::from(count)) >> 64).unwrap_or(0)
}

/// `thawing.lua:2`: 10%.
const THAWING_PERCENT: u64 = 10;
/// `their_masters_voice.lua:4`: 20%.
const THEIR_MASTERS_VOICE_PERCENT: u64 = 20;
/// `oriental_trader.lua:3`: 33%.
const ORIENTAL_TRADER_PERCENT: u64 = 33;
/// `grimvale_respawn_event.lua:3`: the 13th of each month.
const GRIMVALE_DAY: u8 = 13;
/// `raids/world_changes/flower_month.lua:63`: June.
const FLOWER_MONTH: u8 = 6;
/// `raids/world_changes/flower_month.lua:3,64`.
const ROSEMARIE: (&str, (u16, u16, i8)) = ("oteryn:npc.rosemarie", (32_534, 32_827, 7));

/// Where Yasir would arrive (`oriental_trader.lua:4-24`, picked at :44).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrientalTraderTown {
    Ankrahmun,
    Carlin,
    LibertyBay,
}

/// The draws a day takes, in this order, each its own purpose.
pub const DRAW_PURPOSES: [&str; 4] = [
    "thawing",
    "their-masters-voice",
    "oriental-trader",
    "oriental-trader-town",
];

/// The world changes of one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyWorldChanges {
    pub reset_epoch: u64,
    pub thawing: bool,
    pub their_masters_voice: bool,
    pub oriental_trader: Option<OrientalTraderTown>,
    /// Wereboars and werebadgers instead of bandits, badgers and butterflies.
    pub grimvale_full_moon: bool,
    pub flower_month: bool,
}

impl DailyWorldChanges {
    /// Rolls the day after the reset `reset_epoch`, on the UTC calendar `month` and `day`.
    /// `draw(purpose)` returns the uniform 64-bit draw of that purpose for this epoch.
    pub fn roll(reset_epoch: u64, month: u8, day: u8, mut draw: impl FnMut(&str) -> u64) -> Self {
        let [thawing, voice, trader, town] = DRAW_PURPOSES;
        let thawing = percent_roll(draw(thawing), THAWING_PERCENT);
        let their_masters_voice = percent_roll(draw(voice), THEIR_MASTERS_VOICE_PERCENT);
        let oriental_trader = percent_roll(draw(trader), ORIENTAL_TRADER_PERCENT).then(|| {
            match choice(draw(town), 3) {
                0 => OrientalTraderTown::Ankrahmun,
                1 => OrientalTraderTown::Carlin,
                _ => OrientalTraderTown::LibertyBay,
            }
        });
        Self {
            reset_epoch,
            thawing,
            their_masters_voice,
            oriental_trader,
            grimvale_full_moon: day == GRIMVALE_DAY,
            flower_month: month == FLOWER_MONTH,
        }
    }

    /// The NPCs this day adds to the live map: `(key, (x, y, z))`.
    pub fn npcs(&self) -> impl Iterator<Item = (&'static str, (u16, u16, i8))> {
        self.flower_month.then_some(ROSEMARIE).into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The smallest draw at or above `numerator / denominator` of the 64-bit range.
    fn at(numerator: u128, denominator: u128) -> u64 {
        u64::try_from(((u128::from(u64::MAX) + 1) * numerator).div_ceil(denominator))
            .unwrap_or(u64::MAX)
    }

    #[test]
    fn w05b_chances_follow_the_donor_percentages() {
        assert!(percent_roll(at(9, 100), 10) && percent_roll(at(10, 100) - 1, 10));
        assert!(!percent_roll(at(10, 100), 10));
        assert!(percent_roll(at(19, 100), 20) && !percent_roll(at(20, 100), 20));
        assert!(percent_roll(at(32, 100), 33) && !percent_roll(at(33, 100), 33));
        assert_eq!(
            [at(0, 3), at(1, 3), at(2, 3), u64::MAX].map(|draw| choice(draw, 3)),
            [0, 1, 2, 2]
        );
    }

    #[test]
    fn w05b_the_day_takes_each_roll_from_its_own_draw() {
        let roll = |draws: [u64; 4], month, day| {
            DailyWorldChanges::roll(9, month, day, |purpose| {
                let index = DRAW_PURPOSES.iter().position(|p| *p == purpose);
                draws[index.unwrap_or(0)]
            })
        };
        let all = roll([0, 0, 0, at(2, 3)], 6, 13);
        assert_eq!(
            all,
            DailyWorldChanges {
                reset_epoch: 9,
                thawing: true,
                their_masters_voice: true,
                oriental_trader: Some(OrientalTraderTown::LibertyBay),
                grimvale_full_moon: true,
                flower_month: true,
            }
        );
        let none = roll([u64::MAX; 4], 5, 12);
        assert_eq!(
            (
                none.thawing,
                none.their_masters_voice,
                none.oriental_trader,
                none.grimvale_full_moon,
                none.flower_month
            ),
            (false, false, None, false, false)
        );
        // The same epoch and draws roll the same day again.
        assert_eq!(roll([0, 0, 0, at(2, 3)], 6, 13), all);
    }

    #[test]
    fn w05b_rosemarie_stands_only_in_june() {
        let day = |month| DailyWorldChanges::roll(1, month, 1, |_| u64::MAX);
        assert_eq!(
            day(6).npcs().collect::<Vec<_>>(),
            [("oteryn:npc.rosemarie", (32_534, 32_827, 7))]
        );
        for month in [5, 7] {
            assert_eq!(day(month).npcs().count(), 0);
        }
    }
}
