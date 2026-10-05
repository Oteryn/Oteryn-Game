//! CREATURE-AI-1 §1.2: `CreatureBehaviourProfile`, the one runtime value a creature think reads.
//!
//! It is the projection of the validated v2 behaviour authoring (`ProjectV2BehaviorAuthoring`)
//! onto what the think consumes: `targeting` (all fields), `movement.wander`, `attacks[]` and
//! `defenses[]`, with the Ability profiles those entries name. `summons`, `voices`, periodic
//! audio and event bindings are not projected (MONSTER-SUMMON-1 and later decisions). The carrier
//! holds one per creature, given at admission; a creature without one is refused
//! (`PROFILE_MISSING`) and never defaulted. This module holds no profile literal: every value
//! arrives from content (or, until SPAWN-1a, from test data).
//!
//! Declared from `ai_think.rs` with `#[path]`, not from `ai/mod.rs`: `tests/ai_bootstrap.rs`
//! path-includes `ai/mod.rs` as a standalone crate root without `content`.

use std::collections::BTreeMap;

use crate::ai_think::profile_schedule::{MAX_ATTACK_ENTRIES, MAX_DEFENCE_ENTRIES, validate_entry};
use crate::content::{
    ProjectV2AbilityAuthoring, ProjectV2AbilityKind, ProjectV2BehaviorAuthoring,
    ProjectV2DefinitionRef, ProjectV2Targeting, ProjectV2Wander,
};

/// `CREATUREAI0-RL-07`: Ability proposals one think may emit (16 attacks + 8 defences). A
/// profile above it is refused at admission.
pub const CREATUREAI0_PROPOSALS_PER_THINK: usize = MAX_ATTACK_ENTRIES + MAX_DEFENCE_ENTRIES;

/// Why admission refused a creature's profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileRefusal {
    /// `PROFILE_MISSING`: the creature arrived without a behaviour profile.
    Missing,
    /// `PROFILE_INVALID`: above `RL-07`, a malformed or unresolved entry, a melee entry in
    /// `defenses[]`, a melee entry without a magnitude, or more than one melee entry.
    Invalid,
}

/// The melee entry ATTACK-1's swing swings (§1.5); the think never casts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeleeEntry {
    /// Index in `attacks[]`: the swing's `AI_ATTACK` draw index.
    pub index: u16,
    pub interval_ms: u64,
    pub chance_ppm: u32,
    pub minimum: u64,
    pub maximum: u64,
}

/// One creature's projected behaviour profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureBehaviourProfile {
    behavior: ProjectV2BehaviorAuthoring,
    abilities: BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    melee: Option<MeleeEntry>,
}

impl CreatureBehaviourProfile {
    /// Projects and validates `behavior`. `None` is `PROFILE_MISSING`; any refusal leaves no
    /// state behind.
    pub fn project(
        behavior: Option<&ProjectV2BehaviorAuthoring>,
        abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    ) -> Result<Self, ProfileRefusal> {
        let behavior = behavior.ok_or(ProfileRefusal::Missing)?;
        if behavior.attacks.len() > MAX_ATTACK_ENTRIES
            || behavior.defenses.len() > MAX_DEFENCE_ENTRIES
        {
            return Err(ProfileRefusal::Invalid);
        }
        let mut melee = None;
        let mut projected_abilities = BTreeMap::new();
        for (index, entry) in behavior.attacks.iter().enumerate() {
            let details = validate_entry(entry, abilities).map_err(|_| ProfileRefusal::Invalid)?;
            if details.kind == ProjectV2AbilityKind::Melee {
                let magnitude = entry.magnitude.ok_or(ProfileRefusal::Invalid)?;
                if melee.is_some() || magnitude.maximum == 0 {
                    return Err(ProfileRefusal::Invalid);
                }
                melee = Some(MeleeEntry {
                    index: u16::try_from(index).map_err(|_| ProfileRefusal::Invalid)?,
                    interval_ms: entry.interval_ms,
                    chance_ppm: entry.chance_ppm,
                    minimum: magnitude.minimum,
                    maximum: magnitude.maximum,
                });
            }
            if let Some(ability) = abilities.get(&entry.ability) {
                projected_abilities.insert(entry.ability.clone(), ability.clone());
            }
        }
        for entry in &behavior.defenses {
            let details = validate_entry(entry, abilities).map_err(|_| ProfileRefusal::Invalid)?;
            if details.kind == ProjectV2AbilityKind::Melee {
                return Err(ProfileRefusal::Invalid);
            }
            if let Some(ability) = abilities.get(&entry.ability) {
                projected_abilities.insert(entry.ability.clone(), ability.clone());
            }
        }
        let mut projected = behavior.clone();
        projected.voices = None;
        projected.summons = None;
        projected.periodic_audio = None;
        projected.faction = None;
        projected.event_bindings = Vec::new();
        Ok(Self {
            behavior: projected,
            abilities: projected_abilities,
            melee,
        })
    }

    #[must_use]
    pub const fn targeting(&self) -> &ProjectV2Targeting {
        &self.behavior.targeting
    }

    #[must_use]
    pub const fn wander(&self) -> Option<ProjectV2Wander> {
        self.behavior.movement.wander
    }

    #[must_use]
    pub const fn melee(&self) -> Option<MeleeEntry> {
        self.melee
    }

    /// The projected authoring, for `profile_schedule::ProfileScheduleState::prepare`.
    #[must_use]
    pub const fn behavior(&self) -> &ProjectV2BehaviorAuthoring {
        &self.behavior
    }

    #[must_use]
    pub const fn abilities(&self) -> &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring> {
        &self.abilities
    }

    /// Attack plus defence entries: the think's proposal evaluation units.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.behavior.attacks.len() + self.behavior.defenses.len()
    }
}
