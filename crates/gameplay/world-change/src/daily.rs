//! W05b: the daily world changes the server save rolls for the next day.
//!
//! Donor: zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8, whose world changes
//! are `onStartup` global events that run once per daily server save. Each rule below cites the
//! donor script; [`DailyWorldChanges::npcs`] lists the effects this repository applies to the
//! live map. Thawing is applied through the Channel callback item layer. The chance draws come
//! from the caller, keyed on the committed reset epoch, so a restart within the same day rolls
//! the same day again without a second state owner.
//!
//! Not applied (no live-map owner yet; the roll is still taken so the day is fixed):
//! - Flower Month dryad waves (`raids/world_changes/flower_month.lua:28-55`).
//!
//! SP02 projects fungus and trader tiles before map boot and adds Grimvale activation sources.

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
/// `thawing.lua:1`: the ice flower Item placed by Thawing.
pub const ICE_FLOWER: u16 = 13_842;
/// `thawing.lua:4-19`, in donor order: `(x, y, z)`.
pub const THAWING_ICE_FLOWERS: [(u16, u16, i8); 14] = [
    (32_255, 31_079, 5),
    (32_255, 31_066, 5),
    (32_204, 31_054, 5),
    (32_192, 31_048, 5),
    (32_163, 31_030, 5),
    (32_145, 31_036, 5),
    (32_136, 31_049, 5),
    (32_122, 31_060, 5),
    (32_112, 31_071, 5),
    (32_120, 31_082, 5),
    (32_149, 31_061, 5),
    (32_155, 31_085, 5),
    (32_183, 31_091, 5),
    (32_217, 31_091, 5),
];

/// Where Yasir would arrive (`oriental_trader.lua:4-24`, picked at :44).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrientalTraderTown {
    Ankrahmun,
    Carlin,
    LibertyBay,
}

impl OrientalTraderTown {
    pub fn layer(self) -> &'static str {
        match self {
            Self::Ankrahmun => "oriental_trader/ankrahmun",
            Self::Carlin => "oriental_trader/carlin",
            Self::LibertyBay => "oriental_trader/libertybay",
        }
    }

    /// `oriental_trader.lua:13,17,21`: Yasir's deck position, in source coordinates.
    pub fn yasir_position(self) -> (u16, u16, i8) {
        match self {
            Self::Ankrahmun => (33102, 32884, 6),
            Self::Carlin => (32400, 31815, 6),
            Self::LibertyBay => (32314, 32895, 6),
        }
    }
}

/// The draws a day takes, in this order, each its own purpose.
pub const DRAW_PURPOSES: [&str; 7] = [
    "thawing",
    "their-masters-voice",
    "oriental-trader",
    "oriental-trader-town",
    "fury-gates-active",
    "fury-gates-town",
    "nightmare-isles",
];

// Crystal `fury_gates.lua:5-16` and `nightmare_isles.lua:5-9` source order.
pub(crate) const FURY_LAYERS: [&str; 10] = [
    "fury_gates/abdendriel",
    "fury_gates/ankrahmun",
    "fury_gates/carlin",
    "fury_gates/darashia",
    "fury_gates/edron",
    "fury_gates/kazordoon",
    "fury_gates/libertybay",
    "fury_gates/porthope",
    "fury_gates/thais",
    "fury_gates/venore",
];
pub(crate) const NIGHTMARE_LAYERS: [&str; 3] = [
    "nightmare_isle/ankrahmun-north",
    "nightmare_isle/darashia-north",
    "nightmare_isle/darashia-west",
];

/// The world changes of one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyWorldChanges {
    pub reset_epoch: u64,
    pub fury_gates: Option<&'static str>,
    pub fury_exit: &'static str,
    pub nightmare_isles: &'static str,
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
        let [thawing, voice, trader, town, fury, fury_town, nightmare] = DRAW_PURPOSES;
        let thawing = percent_roll(draw(thawing), THAWING_PERCENT);
        let their_masters_voice = percent_roll(draw(voice), THEIR_MASTERS_VOICE_PERCENT);
        let oriental_trader = percent_roll(draw(trader), ORIENTAL_TRADER_PERCENT).then(|| {
            match choice(draw(town), 3) {
                0 => OrientalTraderTown::Ankrahmun,
                1 => OrientalTraderTown::Carlin,
                _ => OrientalTraderTown::LibertyBay,
            }
        });
        // EVENTS-WORLD-1 W10/W15 intentionally replaces donor unconditional availability.
        let fury_active = (u128::from(draw(fury)) * 1000) >> 64 < 343;
        let fury_exit = FURY_LAYERS[choice(draw(fury_town), 10) as usize];
        let fury_gates = fury_active.then_some(fury_exit);
        let nightmare_isles = NIGHTMARE_LAYERS[choice(draw(nightmare), 3) as usize];
        Self {
            reset_epoch,
            fury_gates,
            fury_exit,
            nightmare_isles,
            thawing,
            their_masters_voice,
            oriental_trader,
            grimvale_full_moon: day == GRIMVALE_DAY,
            flower_month: month == FLOWER_MONTH,
        }
    }

    /// Compiled entrance layers selected for this epoch, never paths loaded by the runtime.
    pub fn layers(&self) -> Vec<&'static str> {
        self.fury_gates
            .into_iter()
            .chain([self.nightmare_isles])
            .collect()
    }

    /// Return choice stays available when the entrance is inactive (W10's343permille).
    pub fn exit_layers(&self) -> [&'static str; 1] {
        [self.fury_exit]
    }

    /// The NPCs this day adds to the live map: `(key, (x, y, z))`.
    pub fn npcs(&self) -> impl Iterator<Item = (&'static str, (u16, u16, i8))> {
        (self.reset_epoch != 0 && self.flower_month)
            .then_some(ROSEMARIE)
            .into_iter()
    }

    /// The Items this day adds to the live map: `(appearance, (x, y, z))`.
    pub fn items(&self) -> impl Iterator<Item = (u16, (u16, u16, i8))> {
        self.thawing
            .then_some(THAWING_ICE_FLOWERS)
            .into_iter()
            .flatten()
            .map(|position| (ICE_FLOWER, position))
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
        let roll = |draws: [u64; 7], month, day| {
            DailyWorldChanges::roll(9, month, day, |purpose| {
                let index = DRAW_PURPOSES.iter().position(|p| *p == purpose);
                draws[index.unwrap_or(0)]
            })
        };
        let all = roll([0, 0, 0, at(2, 3), 0, 0, 0], 6, 13);
        assert_eq!(
            all,
            DailyWorldChanges {
                reset_epoch: 9,
                fury_gates: Some("fury_gates/abdendriel"),
                fury_exit: "fury_gates/abdendriel",
                nightmare_isles: "nightmare_isle/ankrahmun-north",
                thawing: true,
                their_masters_voice: true,
                oriental_trader: Some(OrientalTraderTown::LibertyBay),
                grimvale_full_moon: true,
                flower_month: true,
            }
        );
        let none = roll([u64::MAX; 7], 5, 12);
        assert_eq!(none.fury_gates, None);
        assert_eq!(none.exit_layers(), ["fury_gates/venore"]);
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
        assert_eq!(roll([0, 0, 0, at(2, 3), 0, 0, 0], 6, 13), all);
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

    #[test]
    fn w05b_thawing_places_fourteen_ice_flowers_only_on_a_thawing_day() {
        let day = |draws: [u64; 4]| {
            let mut draws = draws.into_iter();
            DailyWorldChanges::roll(1, 1, 1, |_| draws.next().unwrap_or(u64::MAX))
        };
        let items = day([0, u64::MAX, u64::MAX, u64::MAX])
            .items()
            .collect::<Vec<_>>();
        assert_eq!(items.len(), 14);
        assert!(
            items
                .iter()
                .all(|(appearance, (_, _, z))| { *appearance == ICE_FLOWER && *z == 5 })
        );
        assert_eq!(items.first(), Some(&(ICE_FLOWER, (32_255, 31_079, 5))));
        assert_eq!(items.last(), Some(&(ICE_FLOWER, (32_217, 31_091, 5))));
        let positions = items
            .iter()
            .map(|(_, position)| position)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(positions.len(), 14);
        assert_eq!(day([u64::MAX; 4]).items().count(), 0);
    }
}

/// Canonical occurrence draws shared by durable decisions and physical projections.
pub fn roll_calendar(world: &[u8; 16], reset_epoch: u64, month: u8, day: u8) -> DailyWorldChanges {
    let (root, occurrence) = decision_context(world, reset_epoch);
    DailyWorldChanges::roll(reset_epoch, month, day, |purpose| {
        draw(&root, occurrence, purpose)
    })
}

pub(crate) fn draw_for_epoch(world: &[u8; 16], epoch: u64, purpose: &str) -> u64 {
    let (root, occurrence) = decision_context(world, epoch);
    draw(&root, occurrence, purpose)
}

fn decision_context(
    world: &[u8; 16],
    epoch: u64,
) -> (
    oteryn_simulation_determinism::GameplayDecisionRoot,
    [u8; 16],
) {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_change.daily.v1\0");
    hash.update(world);
    let root =
        oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(hash.finalize().into());
    let mut occurrence = [0; 16];
    occurrence[8..].copy_from_slice(&epoch.to_be_bytes());
    (root, occurrence)
}

fn draw(
    root: &oteryn_simulation_determinism::GameplayDecisionRoot,
    occurrence: [u8; 16],
    purpose: &str,
) -> u64 {
    oteryn_simulation_determinism::deterministic_decision_u64(
        root,
        oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(occurrence),
        purpose,
        0,
    )
    .unwrap_or(u64::MAX)
}
