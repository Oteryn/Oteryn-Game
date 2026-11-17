//! Qualified creature policy values shared by Content and the actor carrier.

use oteryn_gameplay_domain::domain::{carrier_values::CarrierError, condition::ConditionType};
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
    /// Explicitly admitted normal, hostile, lootable Forge creature; Bestiary difficulty.
    pub forge_difficulty: Option<u8>,
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
    /// CREATURE-AI-0 §6.3: the profile's `blocked_by_nearby_players`; a player's interest
    /// blocks this creature's due respawn (source `spawn_monster.cpp:288` `isBlockable`).
    pub blocked_by_nearby_players: bool,
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
    soul_war_health: Vec<Arc<super::native_soul_war_callbacks::QualifiedSoulWarHealthPolicy>>,
    native_death_events: Vec<Arc<super::native_creature_death_events::QualifiedNativeDeathEvents>>,
    native_health_events: Vec<Arc<super::native_monster_callbacks::QualifiedNativeHealthEvent>>,
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
        for policy in &mut self.soul_war_health {
            Arc::make_mut(policy).bind_outer(digest);
        }
        for policy in &mut self.native_death_events {
            Arc::make_mut(policy).bind_outer(digest);
        }
        for policy in &mut self.native_health_events {
            Arc::make_mut(policy).bind_outer(digest);
        }
        Ok(())
    }
    pub fn native_health_event(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<&super::native_monster_callbacks::QualifiedNativeHealthEvent> {
        self.native_health_events
            .iter()
            .map(Arc::as_ref)
            .find(|policy| policy.matches(self.source_digest, key, revision))
    }
    pub fn soul_war_health_policy(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<&super::native_soul_war_callbacks::QualifiedSoulWarHealthPolicy> {
        self.soul_war_health
            .iter()
            .map(Arc::as_ref)
            .find(|policy| policy.matches(self.source_digest, key, revision))
    }
    pub fn native_death_events(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<Arc<super::native_creature_death_events::QualifiedNativeDeathEvents>> {
        self.native_death_events
            .iter()
            .find(|policy| policy.matches(self.source_digest, key, revision))
            .cloned()
    }
    pub(crate) fn bind_soul_war_health_callbacks(
        &mut self,
        document: &super::native_gameplay::CreatureProfilesDocument,
    ) {
        self.soul_war_health =
            super::native_soul_war_callbacks::health_policies(document, self.source_digest)
                .into_iter()
                .map(Arc::new)
                .collect();
        self.native_death_events =
            super::native_creature_death_events::death_event_policies(document, self.source_digest);
        self.native_health_events =
            super::native_monster_callbacks::health_event_policies(document, self.source_digest)
                .into_iter()
                .map(Arc::new)
                .collect();
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
            // PUBPROJ owner2026-11-07: ADMINAN reserves these aggregate
            // killer classes. No actual creature may collide with either class.
            if matches!(record.definition_key.as_str(), "player" | "unknown")
                || record.definition_key.is_empty()
                || record.definition_revision.is_empty()
                || record.display_name.trim().is_empty()
                || record.maximum_health <= 0
                || record.base_speed < 0
                || record.forge_difficulty.is_some_and(|difficulty| {
                    !matches!(difficulty, 1 | 2 | 4 | 6 | 8)
                        || record.is_familiar
                        || !record.flags.attackable
                        || record.reward_boss == Some(true)
                })
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
            soul_war_health: Vec::new(),
            native_death_events: Vec::new(),
            native_health_events: Vec::new(),
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

#[cfg(test)]
mod statistics_identity_tests {
    use super::*;
    fn policy(key: &str) -> CompiledCreaturePolicy {
        CompiledCreaturePolicy {
            definition_key: key.into(),
            definition_revision: "r1".into(),
            display_name: "Rat".into(),
            maximum_health: 20,
            forge_difficulty: None,
            base_speed: 100,
            outfit_look_type: 21,
            object_look_type: None,
            summonable: false,
            convinceable: false,
            mana_cost: None,
            is_familiar: false,
            condition_immunities: Vec::new(),
            defense: None,
            armor: None,
            mitigation: None,
            resistances: Vec::new(),
            damage_immunities: Vec::new(),
            healing_from_damage: Vec::new(),
            flags: CreatureFlags {
                attackable: true,
                illusionable: true,
                health_hidden: false,
            },
            preferred_distance: None,
            reward_boss: Some(false),
            blocked_by_nearby_players: true,
        }
    }
    #[test]
    fn native_creature_loader_rejects_admin_analytics_reserved_killer_keys() {
        for key in ["player", "unknown"] {
            assert!(
                matches!(
                    CompiledCreaturePolicies::from_active_artifact([1; 32], vec![policy(key)]),
                    Err(CarrierError::InvalidCreatureTarget)
                ),
                "reserved creature key {key}"
            );
        }
        assert!(
            CompiledCreaturePolicies::from_active_artifact(
                [1; 32],
                vec![policy("oteryn:creature.rat")]
            )
            .is_ok()
        );
    }
}
