//! Qualified creature policy values shared by Content and the actor carrier.

use super::{CarrierError, condition::ConditionType};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureExactRatio {
    pub numerator: i64,
    pub denominator: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureResistance {
    pub damage_type: String,
    pub percent: CreatureExactRatio,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureFlags {
    pub attackable: bool,
    pub illusionable: bool,
    pub health_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCreaturePolicy {
    pub definition_key: String,
    pub definition_revision: String,
    pub display_name: String,
    pub maximum_health: i64,
    pub base_speed: i32,
    pub outfit_look_type: u32,
    /// Source lookTypeEx is an Object appearance, never an Outfit or an invisible actor.
    pub object_look_type: Option<u32>,
    pub summonable: bool,
    pub convinceable: bool,
    pub mana_cost: Option<u32>,
    pub is_familiar: bool,
    pub condition_immunities: Vec<ConditionType>,
    /// The source defence a blocked melee hit draws from (`creature.cpp:966-967`).
    pub defense: Option<u32>,
    pub armor: Option<u32>,
    pub mitigation: Option<CreatureExactRatio>,
    pub resistances: Vec<CreatureResistance>,
    pub damage_immunities: Vec<String>,
    /// Separate source healingMap response, not an elemental resistance.
    pub healing_from_damage: Vec<CreatureResistance>,
    pub flags: CreatureFlags,
    pub preferred_distance: Option<u32>,
    pub reward_boss: Option<bool>,
}
impl CompiledCreaturePolicy {
    /// Source game.cpp healingMap is evaluated before blockHit/immunity, from
    /// the incoming pre-buff magnitude. No attacker means no damage-derived heal.
    pub fn damage_healing(&self, damage_type: &str, magnitude: i64) -> Result<i64, CarrierError> {
        if magnitude < 0 {
            return Err(CarrierError::InvalidDamage);
        }
        let Some(response) = self
            .healing_from_damage
            .iter()
            .find(|v| v.damage_type == damage_type)
        else {
            return Ok(0);
        };
        if response.percent.denominator == 0 || response.percent.numerator < 0 {
            return Err(CarrierError::DamageOverflow);
        }
        let numerator = i128::from(magnitude)
            .checked_mul(i128::from(response.percent.numerator))
            .ok_or(CarrierError::DamageOverflow)?;
        let denominator = i128::from(response.percent.denominator)
            .checked_mul(100)
            .ok_or(CarrierError::DamageOverflow)?;
        let rounded = numerator
            .checked_add(denominator - 1)
            .ok_or(CarrierError::DamageOverflow)?
            / denominator;
        i64::try_from(rounded).map_err(|_| CarrierError::DamageOverflow)
    }
    pub fn outfit_appearance(&self) -> Option<u32> {
        (self.object_look_type.is_none() && self.outfit_look_type != 0)
            .then_some(self.outfit_look_type)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledCreaturePolicies {
    source_digest: [u8; 32],
    policies: Vec<Arc<CompiledCreaturePolicy>>,
}
impl CompiledCreaturePolicies {
    pub const fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub fn policies(&self) -> &[Arc<CompiledCreaturePolicy>] {
        &self.policies
    }

    /// Content's already-qualified outer artifact contains these exact decoded records.
    /// Rebinding data does not issue runtime scope or current mutation authority.
    pub fn bind_qualified_outer_artifact(&mut self, digest: [u8; 32]) -> Result<(), CarrierError> {
        if digest == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        self.source_digest = digest;
        Ok(())
    }
    /// The activation loader supplies only decoded, artifact-qualified CreatureAuthoring data.
    /// Missing health/speed/outfit/details must be rejected there, never filled with prototypes.
    pub fn from_active_artifact(
        source_digest: [u8; 32],
        records: Vec<CompiledCreaturePolicy>,
    ) -> Result<Self, CarrierError> {
        if source_digest == [0; 32] {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let mut policies = Vec::new();
        for record in records {
            if record.definition_key.is_empty()
                || record.definition_revision.is_empty()
                || record.display_name.trim().is_empty()
                || record.maximum_health <= 0
                || record.base_speed < 0
                || record.object_look_type.is_some_and(|look| {
                    look == 0 || record.outfit_look_type != 0 || record.is_familiar
                })
                || record.mitigation.is_some_and(|v| v.denominator == 0)
                || record
                    .resistances
                    .iter()
                    .any(|v| v.damage_type.is_empty() || v.percent.denominator == 0)
                || record.resistances.iter().enumerate().any(|(i, v)| {
                    record.resistances[..i]
                        .iter()
                        .any(|old| old.damage_type == v.damage_type)
                })
                || record.healing_from_damage.iter().enumerate().any(|(i, v)| {
                    v.damage_type.is_empty()
                        || v.percent.denominator == 0
                        || v.percent.numerator < 0
                        || record.healing_from_damage[..i]
                            .iter()
                            .any(|old| old.damage_type == v.damage_type)
                })
                || record.damage_immunities.iter().any(String::is_empty)
                || (record.summonable || record.convinceable) != record.mana_cost.is_some()
                || policies.iter().any(|p: &Arc<CompiledCreaturePolicy>| {
                    p.definition_key == record.definition_key
                })
            {
                return Err(CarrierError::InvalidCreatureTarget);
            }
            policies.push(Arc::new(record));
        }
        Ok(Self {
            source_digest,
            policies,
        })
    }
}

/// Qualified active-Content data for a real familiar self-defense. The consumer
/// independently proves actual actor/master/incarnation and the matching Content pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamiliarSelfHealDefense {
    pub source_digest: [u8; 32],
    pub creature_key: String,
    pub creature_revision: String,
    pub profile_digest: [u8; 32],
    pub interval_ms: u32,
    pub chance_percent: u8,
    pub heal_min: i64,
    pub heal_max: i64,
    pub effect: String,
}
