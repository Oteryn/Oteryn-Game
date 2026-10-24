//! Pure Wheel of Destiny allocation, eligibility and stage rules.

use std::collections::BTreeMap;

/// Slot values exceed the Wheel input bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelInputError;

/// `WHEEL0-RL-01`: 36 slots, 4 domains of 9.
pub const WHEEL_SLOTS: usize = 36;
/// The largest slot capacity (the ruleset's capacities are 50, 75, 100, 150 and 200).
pub const MAX_SLOT_POINTS: u16 = 200;
/// `WHEEL0-RL-02`: domain points of revelation stages 1, 2 and 3.
pub const REVELATION_THRESHOLDS: [u32; 3] = [250, 500, 1000];
/// `WHEEL0-RL-03`: the highest augment stage.
pub const MAX_AUGMENT_STAGE: u8 = 2;
/// `WHEEL0-RL-04`: the Wheel opens above this level, with 1 point per level above it.
pub const WHEEL_LEVEL_OFFSET: u32 = 50;
/// `WHEEL0-RL-05`: the caller's `at_temple` fact is a protection zone within this many tiles of a
/// town temple position (§7.3).
pub const TEMPLE_REMOVAL_RADIUS: u32 = 10;

/// The four domains, in the order `WheelStages` reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WheelDomain {
    Green,
    Red,
    Purple,
    Blue,
}

impl WheelDomain {
    pub const ALL: [Self; 4] = [Self::Green, Self::Red, Self::Purple, Self::Blue];

    fn parse(value: &str) -> Option<Self> {
        match value {
            "green" => Some(Self::Green),
            "red" => Some(Self::Red),
            "purple" => Some(Self::Purple),
            "blue" => Some(Self::Blue),
            _ => None,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// The points of the 36 slots, each at most [`MAX_SLOT_POINTS`]. Slot `n` (1..=36) is index
/// `n - 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WheelSlots([u16; WHEEL_SLOTS]);

impl Default for WheelSlots {
    fn default() -> Self {
        Self::ZERO
    }
}

impl WheelSlots {
    pub const ZERO: Self = Self([0; WHEEL_SLOTS]);

    /// Rejects a slot above the largest capacity (a capacity of its own slot is checked at
    /// commit).
    pub fn new(points: [u16; WHEEL_SLOTS]) -> Result<Self, WheelInputError> {
        if points.iter().any(|value| *value > MAX_SLOT_POINTS) {
            return Err(WheelInputError);
        }
        Ok(Self(points))
    }

    #[must_use]
    pub const fn points(&self) -> &[u16; WHEEL_SLOTS] {
        &self.0
    }

    /// The sum of the slots (at most 7,200).
    #[must_use]
    pub fn total(&self) -> u32 {
        self.0.iter().map(|value| u32::from(*value)).sum()
    }

    fn raises_any(&self, before: &Self) -> bool {
        self.0
            .iter()
            .zip(before.0)
            .any(|(after, before)| *after > before)
    }

    fn lowers_any(&self, before: &Self) -> bool {
        self.0
            .iter()
            .zip(before.0)
            .any(|(after, before)| *after < before)
    }
}

/// One slot of the active ruleset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelSlotRule {
    pub domain: WheelDomain,
    pub capacity: u16,
    /// The available points the Character needs before the slot takes any point.
    pub minimum_points: u32,
    /// The slot opens when one of these slots (1..=36) is full and open itself; empty for the
    /// four centre slots.
    pub unlock_from: Vec<u8>,
}

/// One vocation's perks of the active ruleset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelVocationRules {
    /// The revelation perk key of each domain, in [`WheelDomain::ALL`] order.
    pub revelations: [String; 4],
    /// The conviction perk key of each slot.
    pub convictions: Vec<String>,
    /// The spells each augment conviction key names.
    pub augment_targets: BTreeMap<String, Vec<String>>,
}

/// The ruleset is not the closed shape this owner reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelRulesetError(&'static str);

impl std::fmt::Display for WheelRulesetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for WheelRulesetError {}

/// The active Wheel ruleset revision: slot topology and the perks of the five vocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelRuleset {
    revision: String,
    slots: Vec<WheelSlotRule>,
    vocations: BTreeMap<String, WheelVocationRules>,
}

const VOCATIONS: [&str; 5] = ["druid", "knight", "monk", "paladin", "sorcerer"];

impl WheelRuleset {
    /// Reads the `data` object of the Wheel catalogue (`wheel.json`) under `revision`. Rejects
    /// anything but 36 slots numbered in order with known domains, capacities and adjacency,
    /// nine slots per domain, and the five vocations with 36 slots and four revelations each.
    pub fn from_catalogue(
        revision: &str,
        wheel: &serde_json::Value,
    ) -> std::result::Result<Self, WheelRulesetError> {
        let reject = |reason| WheelRulesetError(reason);
        if !revision_well_formed(revision) {
            return Err(reject("malformed Wheel ruleset revision"));
        }
        let topology = wheel["topology"]
            .as_array()
            .filter(|rows| rows.len() == WHEEL_SLOTS)
            .ok_or(reject("the Wheel topology must have 36 slots"))?;
        let mut slots = Vec::with_capacity(WHEEL_SLOTS);
        for (index, row) in topology.iter().enumerate() {
            if row["state_slot"].as_u64() != Some(index as u64 + 1) {
                return Err(reject("Wheel slots must be numbered 1..36 in order"));
            }
            let domain = row["domain"]
                .as_str()
                .and_then(WheelDomain::parse)
                .ok_or(reject("unknown Wheel domain"))?;
            let capacity = row["capacity"]
                .as_u64()
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| [50, 75, 100, 150, 200].contains(value))
                .ok_or(reject("unknown Wheel slot capacity"))?;
            let minimum_points = row["minimum_available_points"]
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(reject("missing Wheel slot minimum points"))?;
            let unlock_from = row["unlock_from_any_full_slot"]
                .as_array()
                .ok_or(reject("missing Wheel slot adjacency"))?
                .iter()
                .map(|slot| {
                    slot.as_u64()
                        .filter(|slot| (1..=WHEEL_SLOTS as u64).contains(slot))
                        .and_then(|slot| u8::try_from(slot).ok())
                        .filter(|slot| usize::from(*slot) != index + 1)
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(reject("Wheel adjacency names an unknown slot"))?;
            slots.push(WheelSlotRule {
                domain,
                capacity,
                minimum_points,
                unlock_from,
            });
        }
        for domain in WheelDomain::ALL {
            if slots.iter().filter(|slot| slot.domain == domain).count() != 9 {
                return Err(reject("every Wheel domain has nine slots"));
            }
        }
        let catalogue = wheel["vocations"]
            .as_object()
            .filter(|vocations| vocations.len() == VOCATIONS.len())
            .ok_or(reject("the Wheel catalogue must have the five vocations"))?;
        let mut vocations = BTreeMap::new();
        for vocation in VOCATIONS {
            let entry = catalogue
                .get(vocation)
                .ok_or(reject("the Wheel catalogue must have the five vocations"))?;
            vocations.insert(vocation.to_owned(), vocation_rules(entry)?);
        }
        Ok(Self {
            revision: revision.to_owned(),
            slots,
            vocations,
        })
    }

    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    #[must_use]
    pub fn slots(&self) -> &[WheelSlotRule] {
        &self.slots
    }

    /// The rules of a vocation key: a base vocation or its promoted name.
    #[must_use]
    pub fn vocation(&self, vocation: &str) -> Option<&WheelVocationRules> {
        self.vocations.get(base_vocation(vocation)?)
    }

    pub fn capacities(&self) -> [u16; WHEEL_SLOTS] {
        let mut capacities = [0; WHEEL_SLOTS];
        for (capacity, slot) in capacities.iter_mut().zip(&self.slots) {
            *capacity = slot.capacity;
        }
        capacities
    }

    /// Which slots are open under `slots`: a centre slot always, another one when a slot it
    /// unlocks from is full and open itself. A cycle of full slots away from the centre opens
    /// nothing.
    fn open_slots(&self, slots: &WheelSlots) -> [bool; WHEEL_SLOTS] {
        let mut open = [false; WHEEL_SLOTS];
        loop {
            let mut changed = false;
            for (index, rule) in self.slots.iter().enumerate() {
                if open[index] {
                    continue;
                }
                let opens = rule.unlock_from.is_empty()
                    || rule.unlock_from.iter().any(|from| {
                        let from = usize::from(*from) - 1;
                        open[from] && slots.0[from] == self.slots[from].capacity
                    });
                if opens {
                    open[index] = true;
                    changed = true;
                }
            }
            if !changed {
                return open;
            }
        }
    }

    /// The allocation rules of a change from `before` to `after` (§4 "Validation", WHEEL0-PT-1),
    /// for an eligible Character with `available` points. The first failing rule is returned.
    pub fn validate(
        &self,
        before: &WheelSlots,
        after: &WheelSlots,
        available: u32,
        at_temple: bool,
    ) -> std::result::Result<(), WheelRefusal> {
        if after
            .0
            .iter()
            .zip(&self.slots)
            .any(|(points, rule)| *points > rule.capacity)
        {
            return Err(WheelRefusal::OverCapacity);
        }
        let open = self.open_slots(after);
        if after
            .0
            .iter()
            .zip(self.slots.iter().zip(open))
            .any(|(points, (rule, open))| *points > 0 && (!open || available < rule.minimum_points))
        {
            return Err(WheelRefusal::NotAdjacent);
        }
        // A strict decrease is admitted whatever the available points (level loss).
        if after.raises_any(before) && after.total() > available {
            return Err(WheelRefusal::OverPoints);
        }
        if after.lowers_any(before) && !at_temple {
            return Err(WheelRefusal::RemovalNotAtTemple);
        }
        Ok(())
    }
}

fn vocation_rules(
    entry: &serde_json::Value,
) -> std::result::Result<WheelVocationRules, WheelRulesetError> {
    let reject = |reason| WheelRulesetError(reason);
    let slots = entry["slots"]
        .as_array()
        .filter(|slots| slots.len() == WHEEL_SLOTS)
        .ok_or(reject("a Wheel vocation must have 36 slots"))?;
    let mut convictions = Vec::with_capacity(WHEEL_SLOTS);
    let mut augment_targets = BTreeMap::new();
    for (index, slot) in slots.iter().enumerate() {
        if slot["state_slot"].as_u64() != Some(index as u64 + 1) {
            return Err(reject(
                "Wheel vocation slots must be numbered 1..36 in order",
            ));
        }
        let key = slot["conviction"]["key"]
            .as_str()
            .filter(|key| !key.is_empty())
            .ok_or(reject("a Wheel slot must name its conviction perk"))?;
        let targets = slot["conviction"]["augment_targets"]
            .as_array()
            .ok_or(reject(
                "a Wheel conviction perk must list its augment targets",
            ))?
            .iter()
            .map(|target| target.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or(reject("a Wheel augment target must be a spell name"))?;
        if !targets.is_empty() {
            match augment_targets.get(key) {
                Some(known) if *known != targets => {
                    return Err(reject("one augment perk names two target lists"));
                }
                _ => {
                    augment_targets.insert(key.to_owned(), targets);
                }
            }
        }
        convictions.push(key.to_owned());
    }
    let revelations = entry["revelations"]
        .as_array()
        .filter(|revelations| revelations.len() == WheelDomain::ALL.len())
        .ok_or(reject("a Wheel vocation must have four revelations"))?;
    let mut keys: [Option<String>; 4] = Default::default();
    for revelation in revelations {
        let domain = revelation["domain"]
            .as_str()
            .and_then(WheelDomain::parse)
            .ok_or(reject("unknown revelation domain"))?;
        let key = revelation["key"]
            .as_str()
            .filter(|key| !key.is_empty())
            .ok_or(reject("a revelation must name its perk"))?;
        if keys[domain.index()].replace(key.to_owned()).is_some() {
            return Err(reject("one revelation per domain"));
        }
    }
    let [Some(green), Some(red), Some(purple), Some(blue)] = keys else {
        return Err(reject("one revelation per domain"));
    };
    Ok(WheelVocationRules {
        revelations: [green, red, purple, blue],
        convictions,
        augment_targets,
    })
}

/// The base vocation of a vocation key, or `None` for `none` and any unknown key.
#[must_use]
pub fn base_vocation(vocation: &str) -> Option<&'static str> {
    match vocation {
        "druid" | "elder_druid" => Some("druid"),
        "knight" | "elite_knight" => Some("knight"),
        "monk" | "exalted_monk" => Some("monk"),
        "paladin" | "royal_paladin" => Some("paladin"),
        "sorcerer" | "master_sorcerer" => Some("sorcerer"),
        _ => None,
    }
}

fn revision_well_formed(revision: &str) -> bool {
    let bytes = revision.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

/// `max(0, level - 50)` plus extra points, 0 until scrolls, the monk quest and Grade IV mods exist
/// (§6.3).
#[must_use]
pub const fn available_points(level: u32) -> u32 {
    level.saturating_sub(WHEEL_LEVEL_OFFSET)
}

/// Why a Character is not eligible (§6.1, the `WHEEL_QUERY` reasons).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelIneligibility {
    NoVocation,
    Level,
    NotPromoted,
    /// Not applied until PREM-1 delivers `premium_current` (owner answer W1 a, §6.2).
    NotPremium,
}

/// §6.1 at one use: a vocation, level above 50 and promotion. The Premium condition is not
/// applied until Premium is delivered (W1 a).
pub fn eligibility<'r>(
    ruleset: &'r WheelRuleset,
    vocation: &str,
    level: u32,
    promoted: bool,
) -> std::result::Result<&'r WheelVocationRules, WheelIneligibility> {
    let rules = ruleset
        .vocation(vocation)
        .ok_or(WheelIneligibility::NoVocation)?;
    if level <= WHEEL_LEVEL_OFFSET {
        return Err(WheelIneligibility::Level);
    }
    if !promoted {
        return Err(WheelIneligibility::NotPromoted);
    }
    Ok(rules)
}

/// One Character's stored allocation as the runtime actor caches it (§4 "Load"). Never an
/// eligibility result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WheelAllocation {
    /// The revision the allocation is pinned to; `None` without a row (every slot 0).
    pub wheel_ruleset_revision: Option<String>,
    /// 0 without a row, +1 per committed change.
    pub wheel_revision: u64,
    pub slots: WheelSlots,
    /// Whether the allocation is read under the active revision: its revision is the active one,
    /// or every later revision up to the active one is value-only, or there is no row. When not,
    /// every stage derives as 0 and changes are refused (fail closed, §4 WHEEL0-RST-1).
    pub current: bool,
}

/// The stages a cast reads (§3, WHEEL0-EL-1): per revelation perk 0..3, per augment perk 0..2.
/// The default is all 0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WheelStages {
    revelations: BTreeMap<String, u8>,
    augments: BTreeMap<String, u8>,
    spell_augments: BTreeMap<String, u8>,
}

impl WheelStages {
    /// The stages of `allocation` for a Character with `vocation`, `level` and `promoted` read
    /// at this use. Not current or not eligible: all 0, and the allocation stays stored.
    #[must_use]
    pub fn derive(
        ruleset: &WheelRuleset,
        allocation: &WheelAllocation,
        vocation: &str,
        level: u32,
        promoted: bool,
    ) -> Self {
        if !allocation.current {
            return Self::default();
        }
        let Ok(rules) = eligibility(ruleset, vocation, level, promoted) else {
            return Self::default();
        };
        let mut domains = [0_u32; 4];
        for (points, rule) in allocation.slots.0.iter().zip(&ruleset.slots) {
            domains[rule.domain.index()] += u32::from(*points);
        }
        // Only stages above 0 are kept, so an all-zero result equals the default.
        let revelations = WheelDomain::ALL
            .iter()
            .filter_map(|domain| {
                let points = domains[domain.index()];
                let stage = REVELATION_THRESHOLDS
                    .iter()
                    .filter(|threshold| points >= **threshold)
                    .count();
                (stage > 0).then(|| {
                    (
                        rules.revelations[domain.index()].clone(),
                        u8::try_from(stage).unwrap_or(3),
                    )
                })
            })
            .collect();
        let mut augments = BTreeMap::<String, u8>::new();
        for ((points, rule), key) in allocation
            .slots
            .0
            .iter()
            .zip(&ruleset.slots)
            .zip(&rules.convictions)
        {
            if *points == rule.capacity && rules.augment_targets.contains_key(key) {
                let stage = augments.entry(key.clone()).or_default();
                *stage = (*stage + 1).min(MAX_AUGMENT_STAGE);
            }
        }
        let mut spell_augments = BTreeMap::<String, u8>::new();
        for (key, stage) in &augments {
            for spell in rules.augment_targets.get(key).into_iter().flatten() {
                let entry = spell_augments.entry(spell.clone()).or_default();
                *entry = (*entry).max(*stage);
            }
        }
        Self {
            revelations,
            augments,
            spell_augments,
        }
    }

    /// `revelation_stage(perk)`, 0..3.
    #[must_use]
    pub fn revelation_stage(&self, perk: &str) -> u8 {
        self.revelations.get(perk).copied().unwrap_or(0)
    }

    /// `augment_stage` of an augment conviction perk key, 0..2.
    #[must_use]
    pub fn augment_stage(&self, perk: &str) -> u8 {
        self.augments.get(perk).copied().unwrap_or(0)
    }

    /// The augment stage of a spell, by the spell name the ruleset's augment perks target, 0..2.
    #[must_use]
    pub fn spell_augment_stage(&self, spell: &str) -> u8 {
        self.spell_augments.get(spell).copied().unwrap_or(0)
    }
}

/// A change refused before any write (§7.1 results).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelRefusal {
    NotEligible(WheelIneligibility),
    OverCapacity,
    OverPoints,
    NotAdjacent,
    RemovalNotAtTemple,
    RulesetNotCurrent,
    StaleRevision,
    NoChange,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    const WHEEL: &str =
        include_str!("../../../../rulesets/progression/wheel-of-destiny/wheel.json");

    fn ruleset() -> WheelRuleset {
        let file: serde_json::Value = serde_json::from_str(WHEEL).expect("catalogue");
        WheelRuleset::from_catalogue(file["revision"].as_str().expect("revision"), &file["data"])
            .expect("ruleset")
    }

    fn slots(points: &[(usize, u16)]) -> WheelSlots {
        let mut out = [0; WHEEL_SLOTS];
        for (slot, value) in points {
            out[slot - 1] = *value;
        }
        WheelSlots::new(out).expect("slots")
    }

    fn current(slots: WheelSlots) -> WheelAllocation {
        WheelAllocation {
            wheel_ruleset_revision: Some("r".into()),
            wheel_revision: 1,
            slots,
            current: true,
        }
    }

    /// Every slot of `domain` full.
    fn full(ruleset: &WheelRuleset, domain: WheelDomain) -> Vec<(usize, u16)> {
        ruleset
            .slots()
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.domain == domain)
            .map(|(index, rule)| (index + 1, rule.capacity))
            .collect()
    }

    #[test]
    fn the_embedded_catalogue_is_the_migrated_first_revision() {
        let ruleset = ruleset();
        assert_eq!(ruleset.revision(), "wheel-authoring-candidate-r1");
        // The 0070 seed, slot by slot.
        assert_eq!(
            ruleset.capacities(),
            [
                200, 150, 100, 100, 150, 200, 150, 100, 75, 75, 100, 150, 100, 75, 50, 50, 75, 100,
                100, 75, 50, 50, 75, 100, 150, 100, 75, 75, 100, 150, 200, 150, 100, 100, 150, 200
            ]
        );
        for domain in WheelDomain::ALL {
            let total: u16 = full(&ruleset, domain)
                .iter()
                .map(|(_, points)| points)
                .sum();
            assert_eq!(total, 1000, "{domain:?}");
        }
    }

    #[test]
    fn a_malformed_catalogue_is_refused() {
        let file: serde_json::Value = serde_json::from_str(WHEEL).expect("catalogue");
        let mut data = file["data"].clone();
        data["topology"][3]["capacity"] = 60.into();
        assert!(WheelRuleset::from_catalogue("r1", &data).is_err());
        let mut data = file["data"].clone();
        data["topology"][3]["unlock_from_any_full_slot"] = serde_json::json!([37]);
        assert!(WheelRuleset::from_catalogue("r1", &data).is_err());
        let mut data = file["data"].clone();
        data["vocations"]
            .as_object_mut()
            .expect("vocations")
            .remove("monk");
        assert!(WheelRuleset::from_catalogue("r1", &data).is_err());
        assert!(WheelRuleset::from_catalogue("bad revision", &file["data"]).is_err());
    }

    #[test]
    fn revelation_stages_follow_the_thresholds() {
        let ruleset = ruleset();
        // Green: the centre 15 (50) then 9 and 14 (75 each), 3 and 8 (100 each), ...
        let green = full(&ruleset, WheelDomain::Green);
        for (points, stage) in [(249, 0), (250, 1), (500, 2), (999, 2), (1000, 3)] {
            let mut left = points;
            let mut allocation = Vec::new();
            for (slot, capacity) in &green {
                let take = left.min(*capacity);
                allocation.push((*slot, take));
                left -= take;
            }
            let stages = WheelStages::derive(
                &ruleset,
                &current(slots(&allocation)),
                "sorcerer",
                2000,
                true,
            );
            assert_eq!(stages.revelation_stage("gift_of_life"), stage, "{points}");
        }
    }

    #[test]
    fn the_same_allocation_gives_each_vocations_perk() {
        let ruleset = ruleset();
        let allocation = current(slots(&full(&ruleset, WheelDomain::Red)));
        for (vocation, perk) in [
            ("knight", "executioner_s_throw"),
            ("royal_paladin", "divine_grenade"),
            ("sorcerer", "beam_mastery"),
            ("druid", "blessing_of_the_grove"),
            ("monk", "spiritual_outburst"),
        ] {
            let stages = WheelStages::derive(&ruleset, &allocation, vocation, 1100, true);
            assert_eq!(stages.revelation_stage(perk), 3, "{vocation}");
        }
    }

    #[test]
    fn augments_count_full_slots_only() {
        let ruleset = ruleset();
        // Sorcerer: augmented_focus_spells on slots 6 (200) and 21 (50).
        let one = WheelStages::derive(
            &ruleset,
            &current(slots(&[(21, 50)])),
            "sorcerer",
            400,
            true,
        );
        assert_eq!(one.augment_stage("augmented_focus_spells"), 1);
        assert_eq!(one.spell_augment_stage("Hell's Core"), 1);
        let two = WheelStages::derive(
            &ruleset,
            &current(slots(&[(21, 50), (6, 200)])),
            "sorcerer",
            400,
            true,
        );
        assert_eq!(two.augment_stage("augmented_focus_spells"), 2);
        assert_eq!(two.spell_augment_stage("Rage of the Skies"), 2);
        let partial = WheelStages::derive(
            &ruleset,
            &current(slots(&[(21, 49)])),
            "sorcerer",
            400,
            true,
        );
        assert_eq!(partial.augment_stage("augmented_focus_spells"), 0);
    }

    #[test]
    fn an_ineligible_or_not_current_allocation_derives_zero() {
        let ruleset = ruleset();
        let allocation = current(slots(&full(&ruleset, WheelDomain::Green)));
        let stage = |allocation: &WheelAllocation, vocation, level, promoted| {
            WheelStages::derive(&ruleset, allocation, vocation, level, promoted)
                .revelation_stage("gift_of_life")
        };
        assert_eq!(stage(&allocation, "druid", 1050, true), 3);
        assert_eq!(stage(&allocation, "none", 1050, true), 0);
        assert_eq!(stage(&allocation, "druid", 50, true), 0);
        assert_eq!(stage(&allocation, "druid", 1050, false), 0);
        let stale = WheelAllocation {
            current: false,
            ..allocation
        };
        assert_eq!(stage(&stale, "druid", 1050, true), 0);
        assert_eq!(
            WheelStages::derive(&ruleset, &WheelAllocation::default(), "druid", 1050, true),
            WheelStages::default()
        );
        assert_eq!(
            eligibility(&ruleset, "none", 100, true),
            Err(WheelIneligibility::NoVocation)
        );
        assert_eq!(
            eligibility(&ruleset, "knight", 50, true),
            Err(WheelIneligibility::Level)
        );
        assert_eq!(
            eligibility(&ruleset, "knight", 51, false),
            Err(WheelIneligibility::NotPromoted)
        );
        assert!(eligibility(&ruleset, "elite_knight", 51, true).is_ok());
    }

    #[test]
    fn validation_applies_each_rule() {
        let ruleset = ruleset();
        let zero = WheelSlots::ZERO;
        // A centre slot needs no neighbour; slot 9 (75) opens from the full centre 15.
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(15, 50)]), 50, false),
            Ok(())
        );
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(15, 50), (9, 1)]), 51, false),
            Ok(())
        );
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(15, 49), (9, 1)]), 50, false),
            Err(WheelRefusal::NotAdjacent)
        );
        // Slot 9 needs 50 available points.
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(15, 1)]), 1, false),
            Ok(())
        );
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(9, 1), (15, 50)]), 49, false),
            Err(WheelRefusal::NotAdjacent)
        );
        // Full slots far from the centre do not open each other (3, 8, 9 without 15).
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(3, 100), (8, 100), (9, 75)]), 1000, false),
            Err(WheelRefusal::NotAdjacent)
        );
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(15, 51)]), 100, false),
            Err(WheelRefusal::OverCapacity)
        );
        assert_eq!(
            ruleset.validate(&zero, &slots(&[(15, 50), (16, 1)]), 50, false),
            Err(WheelRefusal::OverPoints)
        );
        let held = slots(&[(15, 50), (16, 50)]);
        assert_eq!(
            ruleset.validate(&held, &slots(&[(15, 50), (16, 40)]), 100, false),
            Err(WheelRefusal::RemovalNotAtTemple)
        );
        assert_eq!(
            ruleset.validate(&held, &slots(&[(15, 50), (16, 40)]), 100, true),
            Ok(())
        );
        // WHEEL0-PT-1: over-allocated after a level loss, a strict decrease is admitted; a
        // raise with a decrease is a raise.
        assert_eq!(
            ruleset.validate(&held, &slots(&[(15, 50), (16, 45)]), 90, true),
            Ok(())
        );
        assert_eq!(
            ruleset.validate(&held, &slots(&[(15, 45), (16, 50), (21, 1)]), 90, true),
            Err(WheelRefusal::OverPoints)
        );
        assert_eq!(available_points(50), 0);
        assert_eq!(available_points(51), 1);
        assert_eq!(available_points(1), 0);
    }
}
