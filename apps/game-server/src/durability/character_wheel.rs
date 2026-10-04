//! Durable Wheel of Destiny allocation (WHEEL-W1, migration 0070).
//!
//! `docs/architecture/reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md`
//! §4 on the Wheel state candidate §3.2-§3.4: each Character's 36 slot points, pinned to the
//! Wheel ruleset revision they were validated under.
//!
//! [`DurabilityRoot::commit_character_wheel`] commits one full replacement of the slot vector as
//! one CharacterRevision with one `ALLOCATION` receipt, or writes nothing and returns a
//! [`WheelRefusal`]. It is fenced exactly like `commit_character_experience` (composition rule 2:
//! the session fence, then `character_root` FOR UPDATE, then the Wheel rows). The request carries
//! the expected `wheel_revision`, not the CharacterRevision: a stale one is `STALE_REVISION`, and
//! the binding excludes the CharacterRevision, so a `CharacterRevisionMismatch` reloads the cursor
//! and retries once in the revision slot. The receipt is keyed by (Character, occurrence) and
//! binds the request only: the same occurrence and binding replay the first outcome without
//! reacquiring session authority; another binding conflicts.
//!
//! [`DurabilityRoot::reset_character_wheel`] is the admission Wheel reset (WHEEL0-RST-1): when a
//! `RESET` revision lies after the stored revision up to the active one, one `RULESET_RESET`
//! receipt clears the allocation under the active revision. Its occurrence is derived from the
//! Character and both revisions, so a replay finds the stored revision active and writes
//! nothing. Runtime callers reach both writers only through a
//! [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
//!
//! [`DurabilityRoot::read_character_wheel`] loads the allocation for the runtime actor, and
//! [`WheelStages::derive`] turns it into the stages a cast reads (WHEEL0-EL-1): from the cached
//! allocation and the eligibility read at that moment, never a cached stage.
//!
//! The Wheel ruleset is the embedded `rulesets/progression/wheel-of-destiny/wheel.json`
//! catalogue ([`WheelRuleset::from_catalogue`]); its revision and its 36 capacities must be the
//! ones migration 0070 (or a later W-R migration) registered, or the allocation fails closed.

use std::collections::{BTreeMap, BTreeSet};

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root, uuid_text,
};
use super::character_revision_sequencer::CharacterRevisionSequencer;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, CharacterProgressionError>;
type Transaction<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

const BINDING_VERSION: u8 = 1;

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
    pub fn new(points: [u16; WHEEL_SLOTS]) -> Result<Self> {
        if points.iter().any(|value| *value > MAX_SLOT_POINTS) {
            return Err(CharacterProgressionError::InvalidInput);
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

    fn columns(&self) -> Vec<i16> {
        // Each value is at most 200.
        self.0
            .iter()
            .map(|value| i16::try_from(*value).unwrap_or(i16::MAX))
            .collect()
    }

    fn stored(values: &[i16]) -> std::result::Result<Self, DurabilityError> {
        let mut points = [0_u16; WHEEL_SLOTS];
        if values.len() != WHEEL_SLOTS {
            return Err(DurabilityError::InvalidStoredState);
        }
        for (slot, value) in points.iter_mut().zip(values) {
            *slot = u16::try_from(*value).map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        Self::new(points).map_err(|_| DurabilityError::InvalidStoredState)
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

    fn capacities(&self) -> [u16; WHEEL_SLOTS] {
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

/// The idempotency identity of one allocation change (UUIDv7), issued once by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WheelOccurrence([u8; 16]);

impl WheelOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// `WHEEL_INTENT set_allocation` (§7.1): a full replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelChangeRequest {
    pub occurrence: WheelOccurrence,
    pub expected_wheel_revision: u64,
    pub slots: WheelSlots,
}

/// Runtime facts read at the change: the Character's promotion (PREM-2; `false` until a durable
/// promotion owner exists) and whether it stands where points may be removed (§7.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WheelChangeFacts {
    pub promoted: bool,
    pub at_temple: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelReceiptKind {
    Allocation,
    RulesetReset,
}

impl WheelReceiptKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Allocation => "ALLOCATION",
            Self::RulesetReset => "RULESET_RESET",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedWheelChange {
    pub occurrence: [u8; 16],
    pub kind: WheelReceiptKind,
    pub character_id: CharacterId,
    pub original_character_revision: CharacterRevision,
    pub committed_character_revision: CharacterRevision,
    pub before_wheel_revision: u64,
    pub after_wheel_revision: u64,
    pub before: WheelSlots,
    pub after: WheelSlots,
    pub before_wheel_ruleset_revision: String,
    pub wheel_ruleset_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelCommitOutcome {
    Committed(CommittedWheelChange),
    AlreadyCommitted(CommittedWheelChange),
    Refused(WheelRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelResetOutcome {
    Reset(Box<CommittedWheelChange>),
    /// Nothing to reset: no row, the active revision, or only value-only revisions after it.
    Current,
    /// The stored or active revision is not registered in that order (a newer stored revision,
    /// an unregistered active one or other capacities): nothing is written and the allocation
    /// fails closed.
    Unresolved,
}

/// The stored revision against the active one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Currency {
    Current,
    NeedsReset,
    Unresolved,
}

impl DurabilityRoot {
    /// Commit one full replacement of the Character's slot vector. Exact occurrence replay
    /// returns the retained receipt without reacquiring session authority; another binding of
    /// the occurrence conflicts. A refusal writes nothing and consumes no occurrence.
    /// Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot) (CHAR-REV-SEQ-1).
    pub async fn commit_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: WheelChangeRequest,
        ruleset: std::sync::Arc<WheelRuleset>,
        facts: WheelChangeFacts,
    ) -> Result<WheelCommitOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let binding = request_binding(fence.character_id, &request);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_occurrence(&mut tx, &request.occurrence.0).await?;

                    if let Some(row) =
                        load_receipt(&mut tx, fence.character_id, &request.occurrence.0).await?
                    {
                        let stored: Vec<u8> = row.try_get("request_binding")?;
                        if stored != binding {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(WheelCommitOutcome::AlreadyCommitted(committed)));
                    }

                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let state = match lock_progression_state(&mut tx, fence.character_id).await? {
                        Some(state) => state,
                        None => {
                            return Ok(Err(CharacterProgressionError::MissingProgressionState));
                        }
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let level = u32::try_from(state.try_get::<i64, _>("level")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let stored = lock_wheel_state(&mut tx, fence.character_id).await?;
                    let refuse = |refusal| Ok(Ok(WheelCommitOutcome::Refused(refusal)));

                    let currency = match &stored {
                        Some(stored) => {
                            currency(&mut tx, &stored.wheel_ruleset_revision, &ruleset).await?
                        }
                        None => match currency(&mut tx, ruleset.revision(), &ruleset).await? {
                            Currency::Current => Currency::Current,
                            _ => Currency::Unresolved,
                        },
                    };
                    if currency != Currency::Current {
                        return refuse(WheelRefusal::RulesetNotCurrent);
                    }
                    let (before_revision, before) =
                        stored.as_ref().map_or((0, WheelSlots::ZERO), |stored| {
                            (stored.wheel_revision, stored.slots)
                        });
                    if request.expected_wheel_revision != before_revision {
                        return refuse(WheelRefusal::StaleRevision);
                    }
                    if request.slots == before {
                        return refuse(WheelRefusal::NoChange);
                    }
                    let vocation = read_vocation(&mut tx, fence.character_id).await?;
                    if let Err(reason) = eligibility(&ruleset, &vocation, level, facts.promoted) {
                        return refuse(WheelRefusal::NotEligible(reason));
                    }
                    if let Err(refusal) = ruleset.validate(
                        &before,
                        &request.slots,
                        available_points(level),
                        facts.at_temple,
                    ) {
                        return refuse(refusal);
                    }
                    // An allocation keeps its pinned revision; a first row takes the active one.
                    let pinned = stored.as_ref().map_or_else(
                        || ruleset.revision().to_owned(),
                        |stored| stored.wheel_ruleset_revision.clone(),
                    );
                    let change = PlannedChange {
                        occurrence: request.occurrence.0,
                        kind: WheelReceiptKind::Allocation,
                        binding: binding.clone(),
                        original: root.revision,
                        before_wheel_revision: before_revision,
                        before,
                        after: request.slots,
                        before_ruleset: pinned.clone(),
                        after_ruleset: pinned,
                        first: stored.is_none(),
                    };
                    let committed = write_change(&mut tx, fence.character_id, &change).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(WheelCommitOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// The admission Wheel reset (WHEEL0-RST-1): after the admission's own session fence and
    /// `character_root` FOR UPDATE, clear an allocation whose revision has a `RESET` revision
    /// after it up to the active one, with one `RULESET_RESET` receipt. Free; no eligibility,
    /// points or temple rule applies. Runtime callers reach it only through a
    /// [`RevisionSlot`](super::character_revision_sequencer::RevisionSlot).
    pub async fn reset_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        ruleset: std::sync::Arc<WheelRuleset>,
    ) -> Result<WheelResetOutcome> {
        if fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        Ok(root) => root,
                        Err(error) => return Ok(Err(error)),
                    };
                    let state = match lock_progression_state(&mut tx, fence.character_id).await? {
                        Some(state) => state,
                        None => {
                            return Ok(Err(CharacterProgressionError::MissingProgressionState));
                        }
                    };
                    if numeric_u64(&state, "character_revision")? != root.revision {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    if !state_matches_root(&state, &root) {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                    let Some(stored) = lock_wheel_state(&mut tx, fence.character_id).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(WheelResetOutcome::Current));
                    };
                    let outcome =
                        match currency(&mut tx, &stored.wheel_ruleset_revision, &ruleset).await? {
                            Currency::Current => WheelResetOutcome::Current,
                            Currency::Unresolved => WheelResetOutcome::Unresolved,
                            Currency::NeedsReset => {
                                let occurrence = reset_occurrence(
                                    fence.character_id,
                                    &stored.wheel_ruleset_revision,
                                    ruleset.revision(),
                                );
                                let change = PlannedChange {
                                    occurrence,
                                    kind: WheelReceiptKind::RulesetReset,
                                    binding: reset_binding(
                                        fence.character_id,
                                        &stored.wheel_ruleset_revision,
                                        ruleset.revision(),
                                    ),
                                    original: root.revision,
                                    before_wheel_revision: stored.wheel_revision,
                                    before: stored.slots,
                                    after: WheelSlots::ZERO,
                                    before_ruleset: stored.wheel_ruleset_revision.clone(),
                                    after_ruleset: ruleset.revision().to_owned(),
                                    first: false,
                                };
                                WheelResetOutcome::Reset(Box::new(
                                    write_change(&mut tx, fence.character_id, &change).await?,
                                ))
                            }
                        };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(outcome))
                })
            })
            .await?
    }

    /// Read a retained change after a lost response. This proves only what committed for the
    /// occurrence and never reacquires gameplay authority.
    pub async fn reconcile_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        occurrence: WheelOccurrence,
    ) -> Result<Option<CommittedWheelChange>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let record = load_receipt(&mut tx, character_id, &occurrence.0)
                        .await?
                        .as_ref()
                        .map(decode_receipt)
                        .transpose()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(record))
                })
            })
            .await?
    }

    /// The allocation a new runtime actor caches (§4 "Load"). No row reads every slot 0 under
    /// the active revision. A slot above its capacity under the stored revision, a total that is
    /// not the sum of the slots, or an unknown stored revision fails the load closed with
    /// `Unavailable(InvalidStoredState)`.
    pub async fn read_character_wheel(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        ruleset: std::sync::Arc<WheelRuleset>,
    ) -> Result<WheelAllocation> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let stored = read_wheel_state(&mut tx, character_id, false).await?;
                    let allocation = match stored {
                        None => WheelAllocation {
                            wheel_ruleset_revision: None,
                            wheel_revision: 0,
                            slots: WheelSlots::ZERO,
                            current: currency(&mut tx, ruleset.revision(), &ruleset).await?
                                == Currency::Current,
                        },
                        Some(stored) => {
                            let capacities =
                                stored_capacities(&mut tx, &stored.wheel_ruleset_revision).await?;
                            if stored
                                .slots
                                .0
                                .iter()
                                .zip(capacities)
                                .any(|(points, capacity)| *points > capacity)
                            {
                                return Err(DurabilityError::InvalidStoredState);
                            }
                            WheelAllocation {
                                current: currency(
                                    &mut tx,
                                    &stored.wheel_ruleset_revision,
                                    &ruleset,
                                )
                                .await?
                                    == Currency::Current,
                                wheel_ruleset_revision: Some(stored.wheel_ruleset_revision),
                                wheel_revision: stored.wheel_revision,
                                slots: stored.slots,
                            }
                        }
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(allocation))
                })
            })
            .await?
    }
}

/// The admission step of fresh admission and resume (§4 "Wheel reset" and "Load"): the Wheel
/// reset in the Character's revision slot, then the load. `None` when the load failed: Wheel
/// actions of the session then fail closed, and login does not. A reset that did not commit
/// leaves the allocation not current, so every stage derives as 0.
pub async fn admit_character_wheel(
    sequencer: &CharacterRevisionSequencer,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    ruleset: &std::sync::Arc<WheelRuleset>,
) -> Option<WheelAllocation> {
    let mut slot = sequencer.acquire(fence.character_id).await;
    let reset = slot
        .reset_wheel(root, authority, node, fence, std::sync::Arc::clone(ruleset))
        .await;
    drop(slot);
    if let Err(error) = &reset {
        eprintln!("oteryn-game-server: Wheel reset at admission did not commit: {error}");
    }
    root.read_character_wheel(
        authority,
        fence.character_id,
        std::sync::Arc::clone(ruleset),
    )
    .await
    .ok()
}

/// One stored Wheel state row with its slots.
struct StoredWheel {
    wheel_ruleset_revision: String,
    wheel_revision: u64,
    slots: WheelSlots,
}

/// One change to write: the receipt and the row successor.
struct PlannedChange {
    occurrence: [u8; 16],
    kind: WheelReceiptKind,
    binding: Vec<u8>,
    original: u64,
    before_wheel_revision: u64,
    before: WheelSlots,
    after: WheelSlots,
    before_ruleset: String,
    after_ruleset: String,
    first: bool,
}

async fn lock_occurrence(
    tx: &mut Transaction<'_>,
    occurrence: &[u8; 16],
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:character-wheel:' || encode($1, 'hex'), 0))",
    )
    .bind(occurrence.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn lock_progression_state(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT character_revision::text, level, profile_revision, ruleset_revision, \
                content_revision \
           FROM game_character_progression_state \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

async fn lock_wheel_state(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
) -> std::result::Result<Option<StoredWheel>, DurabilityError> {
    read_wheel_state(tx, character_id, true).await
}

async fn read_wheel_state(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
    for_update: bool,
) -> std::result::Result<Option<StoredWheel>, DurabilityError> {
    let query = if for_update {
        "SELECT wheel_ruleset_revision, wheel_revision::text, allocated_total \
           FROM game_character_wheel_state \
          WHERE character_id = encode($1,'hex')::uuid FOR UPDATE"
    } else {
        "SELECT wheel_ruleset_revision, wheel_revision::text, allocated_total \
           FROM game_character_wheel_state \
          WHERE character_id = encode($1,'hex')::uuid"
    };
    let Some(row) = sqlx::query(query)
        .bind(character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?
    else {
        return Ok(None);
    };
    let rows = sqlx::query(
        "SELECT slot, points FROM game_character_wheel_slots \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let mut points = [0_u16; WHEEL_SLOTS];
    for slot in rows {
        let index = usize::try_from(slot.try_get::<i16, _>("slot")?)
            .ok()
            .and_then(|slot| slot.checked_sub(1))
            .filter(|index| *index < WHEEL_SLOTS)
            .ok_or(DurabilityError::InvalidStoredState)?;
        points[index] = u16::try_from(slot.try_get::<i16, _>("points")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    let slots = WheelSlots::new(points).map_err(|_| DurabilityError::InvalidStoredState)?;
    let total = u32::try_from(row.try_get::<i32, _>("allocated_total")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if total != slots.total() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(StoredWheel {
        wheel_ruleset_revision: row.try_get("wheel_ruleset_revision")?,
        wheel_revision: numeric_u64(&row, "wheel_revision")?,
        slots,
    }))
}

/// The Character's vocation key; `none` without a build row.
async fn read_vocation(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
) -> std::result::Result<String, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT vocation FROM game_character_build_state \
          WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or_else(|| "none".to_owned()))
}

async fn stored_capacities(
    tx: &mut Transaction<'_>,
    revision: &str,
) -> std::result::Result<[u16; WHEEL_SLOTS], DurabilityError> {
    let rows = sqlx::query(
        "SELECT slot, capacity FROM game_wheel_ruleset_slot_capacities \
          WHERE wheel_ruleset_revision = $1",
    )
    .bind(revision)
    .fetch_all(&mut **tx)
    .await?;
    let mut capacities = [0_u16; WHEEL_SLOTS];
    let mut seen = BTreeSet::new();
    for row in rows {
        let slot = usize::try_from(row.try_get::<i16, _>("slot")?)
            .ok()
            .filter(|slot| (1..=WHEEL_SLOTS).contains(slot))
            .ok_or(DurabilityError::InvalidStoredState)?;
        capacities[slot - 1] = u16::try_from(row.try_get::<i16, _>("capacity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        seen.insert(slot);
    }
    if seen.len() != WHEEL_SLOTS {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(capacities)
}

/// The stored revision `stored` against the active `ruleset` (§4 WHEEL0-RST-1, §5.1). The active
/// revision must be registered with the ruleset's own capacities, and no later than nothing.
async fn currency(
    tx: &mut Transaction<'_>,
    stored: &str,
    ruleset: &WheelRuleset,
) -> std::result::Result<Currency, DurabilityError> {
    let rows = sqlx::query(
        "SELECT wheel_ruleset_revision, ordinal, kind FROM game_wheel_ruleset_revisions",
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut order = BTreeMap::new();
    for row in rows {
        order.insert(
            row.try_get::<String, _>("wheel_ruleset_revision")?,
            (
                row.try_get::<i32, _>("ordinal")?,
                row.try_get::<String, _>("kind")?,
            ),
        );
    }
    let (Some((stored_ordinal, _)), Some((active_ordinal, _))) =
        (order.get(stored), order.get(ruleset.revision()))
    else {
        return Ok(Currency::Unresolved);
    };
    if *stored_ordinal > *active_ordinal
        || stored_capacities(tx, ruleset.revision()).await? != ruleset.capacities()
    {
        return Ok(Currency::Unresolved);
    }
    let reset = order.values().any(|(ordinal, kind)| {
        *ordinal > *stored_ordinal && *ordinal <= *active_ordinal && kind == "RESET"
    });
    Ok(if reset {
        Currency::NeedsReset
    } else {
        Currency::Current
    })
}

/// Write one planned change: the root and typed-state successor, the receipt, the state row and
/// the changed slot rows, in that order.
async fn write_change(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
    change: &PlannedChange,
) -> std::result::Result<CommittedWheelChange, DurabilityError> {
    let character = character_id.as_bytes().as_slice();
    let committed_revision = change
        .original
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let after_wheel_revision = change
        .before_wheel_revision
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let root_update = sqlx::query(
        "UPDATE game_character_roots SET character_revision = $2::text::numeric(20,0) \
          WHERE character_id = encode($1,'hex')::uuid \
            AND character_revision = $3::text::numeric(20,0)",
    )
    .bind(character)
    .bind(committed_revision.to_string())
    .bind(change.original.to_string())
    .execute(&mut **tx)
    .await?;
    let state_update = sqlx::query(
        "UPDATE game_character_progression_state \
            SET character_revision = $2::text::numeric(20,0) \
          WHERE character_id = encode($1,'hex')::uuid \
            AND character_revision = $3::text::numeric(20,0)",
    )
    .bind(character)
    .bind(committed_revision.to_string())
    .bind(change.original.to_string())
    .execute(&mut **tx)
    .await?;
    if root_update.rows_affected() != 1 || state_update.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let receipt = sqlx::query(
        "INSERT INTO game_character_wheel_receipts(\
           character_id, wheel_occurrence_id, kind, request_binding, before_wheel_revision, \
           after_wheel_revision, slots_before, slots_after, before_wheel_ruleset_revision, \
           wheel_ruleset_revision, original_character_revision, committed_character_revision, \
           level_before, level_after, experience_before, experience_after, profile_revision, \
           ruleset_revision, content_revision, simulation_revision, evidence_revision, \
           declaration_revision, policy_revision, reward_revision, committed_at) \
         SELECT s.character_id, encode($2,'hex')::uuid, $3, $4, $5::text::numeric(20,0), \
           $6::text::numeric(20,0), $7, $8, $9, $10, $11::text::numeric(20,0), \
           $12::text::numeric(20,0), s.level, s.level, s.total_experience, \
           s.total_experience, s.profile_revision, s.ruleset_revision, s.content_revision, \
           s.simulation_revision, s.evidence_revision, s.declaration_revision, \
           s.policy_revision, s.reward_revision, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
           FROM game_character_progression_state s \
          WHERE s.character_id = encode($1,'hex')::uuid",
    )
    .bind(character)
    .bind(change.occurrence.as_slice())
    .bind(change.kind.as_str())
    .bind(&change.binding)
    .bind(change.before_wheel_revision.to_string())
    .bind(after_wheel_revision.to_string())
    .bind(change.before.columns())
    .bind(change.after.columns())
    .bind(&change.before_ruleset)
    .bind(&change.after_ruleset)
    .bind(change.original.to_string())
    .bind(committed_revision.to_string())
    .execute(&mut **tx)
    .await?;
    if receipt.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let total =
        i32::try_from(change.after.total()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row = if change.first {
        sqlx::query(
            "INSERT INTO game_character_wheel_state(character_id, wheel_ruleset_revision, \
               wheel_revision, allocated_total, committed_character_revision, \
               last_wheel_occurrence_id) \
             VALUES (encode($1,'hex')::uuid, $2, $3::text::numeric(20,0), $4, \
               $5::text::numeric(20,0), encode($6,'hex')::uuid)",
        )
    } else {
        sqlx::query(
            "UPDATE game_character_wheel_state SET wheel_ruleset_revision = $2, \
               wheel_revision = $3::text::numeric(20,0), allocated_total = $4, \
               committed_character_revision = $5::text::numeric(20,0), \
               last_wheel_occurrence_id = encode($6,'hex')::uuid \
             WHERE character_id = encode($1,'hex')::uuid",
        )
    }
    .bind(character)
    .bind(&change.after_ruleset)
    .bind(after_wheel_revision.to_string())
    .bind(total)
    .bind(committed_revision.to_string())
    .bind(change.occurrence.as_slice())
    .execute(&mut **tx)
    .await?;
    if row.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Only the slots that change: a slot at 0 has no row.
    let (mut kept_slots, mut kept_points) = (Vec::new(), Vec::new());
    for (index, points) in change.after.0.iter().enumerate() {
        if *points > 0 {
            kept_slots.push(i16::try_from(index + 1).unwrap_or(i16::MAX));
            kept_points.push(i16::try_from(*points).unwrap_or(i16::MAX));
        }
    }
    sqlx::query(
        "DELETE FROM game_character_wheel_slots \
          WHERE character_id = encode($1,'hex')::uuid AND NOT (slot = ANY($2))",
    )
    .bind(character)
    .bind(&kept_slots)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_wheel_slots(character_id, slot, points) \
         SELECT encode($1,'hex')::uuid, s.slot, s.points \
           FROM unnest($2::smallint[], $3::smallint[]) AS s(slot, points) \
         ON CONFLICT (character_id, slot) DO UPDATE SET points = EXCLUDED.points \
          WHERE game_character_wheel_slots.points <> EXCLUDED.points",
    )
    .bind(character)
    .bind(&kept_slots)
    .bind(&kept_points)
    .execute(&mut **tx)
    .await?;

    let revision =
        |value| CharacterRevision::new(value).map_err(|_| DurabilityError::InvalidStoredState);
    Ok(CommittedWheelChange {
        occurrence: change.occurrence,
        kind: change.kind,
        character_id,
        original_character_revision: revision(change.original)?,
        committed_character_revision: revision(committed_revision)?,
        before_wheel_revision: change.before_wheel_revision,
        after_wheel_revision,
        before: change.before,
        after: change.after,
        before_wheel_ruleset_revision: change.before_ruleset.clone(),
        wheel_ruleset_revision: change.after_ruleset.clone(),
    })
}

fn bound(semantic: &[u8]) -> Vec<u8> {
    let digest: [u8; 32] = Sha256::digest(semantic).into();
    let mut binding = Vec::with_capacity(33);
    binding.push(BINDING_VERSION);
    binding.extend_from_slice(&digest);
    binding
}

/// The request-only binding of a change (§4): occurrence, Character, expected Wheel revision and
/// slot vector. The CharacterRevision and every retry-local authority are excluded.
fn request_binding(character: CharacterId, request: &WheelChangeRequest) -> Vec<u8> {
    let mut semantic = vec![BINDING_VERSION, b'A'];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(&request.expected_wheel_revision.to_be_bytes());
    for points in request.slots.0 {
        semantic.extend_from_slice(&points.to_be_bytes());
    }
    bound(&semantic)
}

fn reset_semantic(character: CharacterId, source: &str, destination: &str) -> Vec<u8> {
    let mut semantic = vec![BINDING_VERSION, b'R'];
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(source.as_bytes());
    semantic.push(0);
    semantic.extend_from_slice(destination.as_bytes());
    semantic
}

fn reset_binding(character: CharacterId, source: &str, destination: &str) -> Vec<u8> {
    bound(&reset_semantic(character, source, destination))
}

/// The occurrence of a reset, derived from (Character, source, destination) (§13 item 3): a
/// version 8 UUID of the binding's digest.
fn reset_occurrence(character: CharacterId, source: &str, destination: &str) -> [u8; 16] {
    let mut semantic = b"oteryn:wheel-reset:".to_vec();
    semantic.extend_from_slice(&reset_semantic(character, source, destination));
    let digest: [u8; 32] = Sha256::digest(&semantic).into();
    let mut occurrence = [0_u8; 16];
    occurrence.copy_from_slice(&digest[..16]);
    occurrence[6] = (occurrence[6] & 0x0f) | 0x80;
    occurrence[8] = (occurrence[8] & 0x3f) | 0x80;
    occurrence
}

async fn load_receipt(
    tx: &mut Transaction<'_>,
    character_id: CharacterId,
    occurrence: &[u8; 16],
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT wheel_occurrence_id::text, kind, request_binding, character_id::text, \
                original_character_revision::text, committed_character_revision::text, \
                before_wheel_revision::text, after_wheel_revision::text, slots_before, \
                slots_after, before_wheel_ruleset_revision, wheel_ruleset_revision \
           FROM game_character_wheel_receipts \
          WHERE character_id = encode($1,'hex')::uuid \
            AND wheel_occurrence_id = encode($2,'hex')::uuid",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(occurrence.as_slice())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedWheelChange, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let kind = match row.try_get::<String, _>("kind")?.as_str() {
        "ALLOCATION" => WheelReceiptKind::Allocation,
        "RULESET_RESET" => WheelReceiptKind::RulesetReset,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(CommittedWheelChange {
        occurrence: uuid_text(row.try_get("wheel_occurrence_id")?)?,
        kind,
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(invalid)?,
        original_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "original_character_revision",
        )?)
        .map_err(invalid)?,
        committed_character_revision: CharacterRevision::new(numeric_u64(
            row,
            "committed_character_revision",
        )?)
        .map_err(invalid)?,
        before_wheel_revision: numeric_u64(row, "before_wheel_revision")?,
        after_wheel_revision: numeric_u64(row, "after_wheel_revision")?,
        before: WheelSlots::stored(&row.try_get::<Vec<i16>, _>("slots_before")?)?,
        after: WheelSlots::stored(&row.try_get::<Vec<i16>, _>("slots_after")?)?,
        before_wheel_ruleset_revision: row.try_get("before_wheel_ruleset_revision")?,
        wheel_ruleset_revision: row.try_get("wheel_ruleset_revision")?,
    })
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

    #[test]
    fn slots_and_occurrences_are_bounded() {
        let mut points = [0; WHEEL_SLOTS];
        points[0] = MAX_SLOT_POINTS;
        assert!(WheelSlots::new(points).is_ok());
        points[0] = MAX_SLOT_POINTS + 1;
        assert!(WheelSlots::new(points).is_err());
        assert!(WheelSlots::stored(&[0; 35]).is_err());
        assert!(WheelSlots::stored(&[-1; 36]).is_err());
        let mut v7 = [0x11; 16];
        v7[6] = 0x70;
        v7[8] = 0x80;
        assert!(WheelOccurrence::from_bytes(v7).is_ok());
        v7[6] = 0x40;
        assert!(WheelOccurrence::from_bytes(v7).is_err());
    }

    #[test]
    fn a_reset_occurrence_is_derived_from_the_character_and_both_revisions() {
        let id = |seed: u8| {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            CharacterId::from_bytes(bytes).expect("character")
        };
        let (character, other) = (id(3), id(4));
        let occurrence = reset_occurrence(character, "r1", "r2");
        assert_eq!(occurrence, reset_occurrence(character, "r1", "r2"));
        assert_ne!(occurrence, reset_occurrence(other, "r1", "r2"));
        assert_ne!(occurrence, reset_occurrence(character, "r1", "r3"));
        assert_ne!(occurrence, reset_occurrence(character, "r1r", "2"));
        assert_eq!(occurrence[6] >> 4, 8);
        assert_eq!(occurrence[8] & 0xc0, 0x80);
        assert_eq!(reset_binding(character, "r1", "r2").len(), 33);
    }
}
