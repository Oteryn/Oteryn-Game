//! Source-required caster inputs. Missing producer evidence is distinct from a known negative.
//! This data snapshot grants no session authority; the gameplay owner qualifies its binding anew.
use super::cast::PlayerSpellState;
use super::harmony::HarmonyMultiplier;
use super::{CasterState, SpellDefinition};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceWeaponType};
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::foundation::{ExactActorRef, GameSessionId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastFactsBinding {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub character: [u8; 16],
    pub character_revision: u64,
    pub lease_generation: u64,
    pub connection_generation: u64,
    pub player_revision: u64,
    pub content_digest: [u8; 32],
    pub equipment_revision: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentProjection<T> {
    pub binding: CastFactsBinding,
    pub authority_revision: u64,
    pub valid_until_micros: u64,
    pub value: T,
}
impl<T> CurrentProjection<T> {
    pub fn current(&self, binding: &CastFactsBinding, now: u64) -> bool {
        &self.binding == binding && self.authority_revision > 0 && now < self.valid_until_micros
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessProjections {
    /// Independently registered combat-state owner. Missing skull evidence is
    /// unavailable; ordinary group membership cannot manufacture a clear skull.
    pub black_skull: Option<CurrentProjection<bool>>,
    pub premium: Option<CurrentProjection<bool>>,
    pub learned: Option<CurrentProjection<BTreeSet<String>>>,
    pub wheel: Option<CurrentProjection<BTreeMap<String, u8>>>,
    pub elemental_stance: Option<CurrentProjection<super::native_combat::ElementalStance>>,
    pub magnitude: Option<super::magnitude_owner::MagnitudeOwnedAttributes>,
}
#[derive(Debug, Clone)]
pub struct OwnedCastFacts {
    pub binding: CastFactsBinding,
    build: DurableBuildState,
    level: u32,
    equipment: EquipmentSnapshot,
    pub access: AccessProjections,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedFactsError {
    StaleBinding,
    BuildMismatch,
    UnavailablePremium,
    UnavailableLearning,
    UnavailableWheel,
    UnavailableEquipment,
    UnknownItemPolicy,
    UnknownCombatMode,
}
impl OwnedCastFacts {
    pub fn current_black_skull(&self, now: u64) -> Option<&CurrentProjection<bool>> {
        self.access
            .black_skull
            .as_ref()
            .filter(|p| p.current(&self.binding, now))
    }
    /// The producer is the independently fenced gameplay loader, never a decoded client packet.
    pub fn from_owner_reads(
        binding: CastFactsBinding,
        build: DurableBuildState,
        level: u32,
        equipment: EquipmentSnapshot,
        access: AccessProjections,
    ) -> Result<Self, OwnedFactsError> {
        if binding.character != equipment.character
            || binding.character_revision != equipment.character_revision
            || binding.content_digest != equipment.content_digest
            || binding.equipment_revision != equipment.revision
            || binding.content_digest == [0; 32]
            || binding.lease_generation == 0
            || binding.connection_generation == 0
            || level == 0
        {
            return Err(OwnedFactsError::StaleBinding);
        }
        Ok(Self {
            binding,
            build,
            level,
            equipment,
            access,
        })
    }
    pub fn current_premium(&self, now: u64) -> Option<&CurrentProjection<bool>> {
        self.access
            .premium
            .as_ref()
            .filter(|value| value.current(&self.binding, now))
    }
    pub fn binding(&self) -> &CastFactsBinding {
        &self.binding
    }
    pub fn wheel_revision(&self) -> Option<u64> {
        self.access.wheel.as_ref().map(|p| p.authority_revision)
    }
    pub fn durable_build(&self) -> &DurableBuildState {
        &self.build
    }
    /// An original pending roll cannot grant a now-expired account/learning benefit.
    pub fn check_current_access(
        &self,
        spell: &SpellDefinition,
        now: u64,
    ) -> Result<(), OwnedFactsError> {
        if spell.premium
            && !self
                .access
                .premium
                .as_ref()
                .filter(|p| p.current(&self.binding, now))
                .is_some_and(|p| p.value)
        {
            return Err(OwnedFactsError::UnavailablePremium);
        }
        if spell.learning_required
            && !self
                .access
                .learned
                .as_ref()
                .filter(|p| p.current(&self.binding, now))
                .is_some_and(|p| p.value.contains(&spell.key))
        {
            return Err(OwnedFactsError::UnavailableLearning);
        }
        Ok(())
    }
    pub fn equipment(&self) -> &EquipmentSnapshot {
        &self.equipment
    }
    pub fn magnitude(&self) -> Option<&super::magnitude_owner::MagnitudeOwnedAttributes> {
        self.access.magnitude.as_ref()
    }
    pub fn elemental_stance(
        &self,
        now: u64,
    ) -> Option<super::native_combat::ElementalStance> {
        self.access
            .elemental_stance
            .as_ref()
            .filter(|p| p.current(&self.binding, now))
            .map(|p| p.value)
    }
    pub fn equipment_speed_bonus(
        &self,
        content: &NativeGameplayState,
    ) -> Result<i32, OwnedFactsError> {
        if content.source_digest() != self.binding.content_digest {
            return Err(OwnedFactsError::StaleBinding);
        }
        self.equipment.items.iter().try_fold(0_i32, |sum, item| {
            let policy = content
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .ok_or(OwnedFactsError::UnknownItemPolicy)?;
            let value = policy
                .record()
                .attributes
                .speed_bonus
                .ok_or(OwnedFactsError::UnavailableEquipment)?;
            sum.checked_add(value)
                .ok_or(OwnedFactsError::UnavailableEquipment)
        })
    }
    pub fn has_current_wheel(&self, now: u64) -> bool {
        self.access
            .wheel
            .as_ref()
            .is_some_and(|p| p.current(&self.binding, now))
    }
    pub fn wheel_stage(&self, perk: &str, now: u64) -> Result<u8, OwnedFactsError> {
        let current = self
            .access
            .wheel
            .as_ref()
            .filter(|p| p.current(&self.binding, now))
            .ok_or(OwnedFactsError::UnavailableWheel)?;
        // Explicitly missing perk in a complete owner projection means no allocation for it.
        let stage = current.value.get(perk).copied().unwrap_or(0);
        if stage > 3 {
            return Err(OwnedFactsError::UnavailableWheel);
        }
        Ok(stage)
    }
    pub fn caster(
        &self,
        state: &PlayerSpellState,
        spell: &SpellDefinition,
        content: &NativeGameplayState,
        harmony: HarmonyMultiplier,
        now: u64,
    ) -> Result<CasterState, OwnedFactsError> {
        self.caster_projection(state, spell, content, harmony, now, true)
    }
    /// An already accepted chain callback reads current numerical owners without
    /// re-running the initial premium/learning admission. It cannot pay a cast.
    pub fn numerical_caster(
        &self,
        state: &PlayerSpellState,
        spell: &SpellDefinition,
        content: &NativeGameplayState,
        harmony: HarmonyMultiplier,
        now: u64,
    ) -> Result<CasterState, OwnedFactsError> {
        self.caster_projection(state, spell, content, harmony, now, false)
    }
    fn caster_projection(
        &self,
        state: &PlayerSpellState,
        spell: &SpellDefinition,
        content: &NativeGameplayState,
        harmony: HarmonyMultiplier,
        now: u64,
        require_cast_access: bool,
    ) -> Result<CasterState, OwnedFactsError> {
        let facts = state.character_facts();
        let vitals = state.vitals();
        let skill_adjustments = state
            .owned_skill_adjustments(now)
            .map_err(|_| OwnedFactsError::BuildMismatch)?;
        if state.revision() != self.binding.player_revision
            || content.source_digest() != self.binding.content_digest
        {
            return Err(OwnedFactsError::StaleBinding);
        }
        if facts.level != self.level
            || super::Vocation::from_key(self.build.vocation()) != Some(facts.vocation)
            || facts.magic_level != u32::from(self.build.magic().0)
        {
            return Err(OwnedFactsError::BuildMismatch);
        }
        let premium = self
            .access
            .premium
            .as_ref()
            .filter(|p| p.current(&self.binding, now));
        if require_cast_access && spell.premium && premium.is_none() {
            return Err(OwnedFactsError::UnavailablePremium);
        }
        let learned = self
            .access
            .learned
            .as_ref()
            .filter(|p| p.current(&self.binding, now));
        if require_cast_access && spell.learning_required && learned.is_none() {
            return Err(OwnedFactsError::UnavailableLearning);
        }
        let needs_attack = spell.needs_weapon
            || matches!(&spell.execution, super::Execution::NativeProfile(profile)
                if attack_input(&profile.spell()["execution"]))
            || spell.authored.as_ref().is_some_and(|p| {
                p.dependencies.formulas.iter().any(|f| {
                    f.minimum.as_ref().is_some_and(attack_input)
                        || f.maximum.as_ref().is_some_and(attack_input)
                })
            });
        let mut attack = None;
        let mut shield = None;
        // Source-defined first shield is left hand then right. The snapshot is the actual owner.
        for slot in [6_u8, 5] {
            if !needs_attack && !spell.needs_shield {
                break;
            }
            if let Some(item) = self.equipment.items.iter().find(|i| i.slot == slot) {
                let policy = content
                    .item_policy(
                        &item.definition.production_key,
                        &item.definition.revision_ref,
                    )
                    .ok_or(OwnedFactsError::UnknownItemPolicy)?;
                let ReferenceItemField::Known(weapon) = &policy.record().semantics.weapon else {
                    return Err(OwnedFactsError::UnavailableEquipment);
                };
                let ReferenceItemField::Known(kind) = weapon.weapon_type else {
                    return Err(OwnedFactsError::UnavailableEquipment);
                };
                if kind == ReferenceWeaponType::Shield {
                    if shield.is_none() {
                        let ReferenceItemField::Known(defense) = weapon.defense else {
                            return Err(OwnedFactsError::UnavailableEquipment);
                        };
                        shield = Some(
                            u32::try_from(defense.0)
                                .map_err(|_| OwnedFactsError::UnavailableEquipment)?,
                        );
                    }
                } else if slot == 5 {
                    let skill = match kind {
                        ReferenceWeaponType::Fist => Some(0),
                        ReferenceWeaponType::Club => Some(1),
                        ReferenceWeaponType::Sword => Some(2),
                        ReferenceWeaponType::Axe => Some(3),
                        ReferenceWeaponType::Distance => Some(4),
                        _ => None,
                    };
                    if let Some(skill) = skill {
                        let ReferenceItemField::Known(value) = weapon.attack else {
                            return Err(OwnedFactsError::UnavailableEquipment);
                        };
                        attack = Some((
                            skill_adjustments
                                .adjust_skill(u32::from(self.build.skills()[skill].0), skill)
                                .ok_or(OwnedFactsError::BuildMismatch)?,
                            u32::try_from(value.0)
                                .map_err(|_| OwnedFactsError::UnavailableEquipment)?,
                            skill < 4,
                        ));
                    }
                }
            }
        }
        if needs_attack && attack.is_none() {
            return Err(OwnedFactsError::UnavailableEquipment);
        }
        let attack_factor = if needs_attack {
            self.equipment
                .combat_mode
                .ok_or(OwnedFactsError::UnknownCombatMode)?
                .attack_factor()
        } else {
            0.0
        };
        if spell.needs_shield && shield.is_none() {
            return Err(OwnedFactsError::UnavailableEquipment);
        }
        let (attack_skill, attack_value, melee_weapon) = attack.unwrap_or((0, 0, false));
        Ok(CasterState {
            vocation: facts.vocation,
            level: facts.level,
            magic_level: skill_adjustments
                .adjust_magic_level(facts.magic_level)
                .ok_or(OwnedFactsError::BuildMismatch)?,
            premium: super::CasterPremium::from_current(premium.is_some_and(|p| p.value)),
            mana: vitals.mana,
            max_mana: vitals.max_mana,
            soul: vitals.soul,
            learned: learned.map(|p| p.value.clone()).unwrap_or_default(),
            attack_skill,
            attack_value,
            attack_factor,
            shielding_skill: skill_adjustments
                .adjust_shielding(u32::from(self.build.skills()[5].0))
                .ok_or(OwnedFactsError::BuildMismatch)?,
            melee_weapon,
            shield_defense: shield,
            harmony_multiplier: harmony,
        })
    }
}
pub fn attack_input(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(s) => matches!(
            s.as_str(),
            "attack_skill" | "attack_value" | "attack_factor"
        ),
        serde_json::Value::Object(o) => o.values().any(attack_input),
        serde_json::Value::Array(a) => a.iter().any(attack_input),
        _ => false,
    }
}
