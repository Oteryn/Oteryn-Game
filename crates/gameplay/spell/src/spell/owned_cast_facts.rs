//! Source-required caster inputs. Missing producer evidence is distinct from a known negative.
//! This data snapshot grants no session authority; the gameplay owner qualifies its binding anew.
use super::cast::PlayerSpellState;
use super::harmony::HarmonyMultiplier;
use super::magnitude_owner::IntrinsicWeaponDamage;
use super::{CasterState, SpellDefinition};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ProjectV2RangedWeapon, ReferenceItemField, ReferenceWeaponType};
use crate::domain::equipment_policy_abi::{EquipmentBaseVocation, EquipmentPolicyLookup};
use crate::durability::character_build::DurableBuildState;
use crate::durability::character_equipment::{CombatMode, EquipmentSnapshot, EquippedItem};
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
    /// Selected rewards of the currently worn weapon, read by native cast dispatch.
    /// Fist, club, sword, axe, distance, shielding, fishing, magic level.
    pub proficiency_skills: [u32; 8],
    /// Current durable Platform projection. Zero is the defined no-row value.
    pub loyalty_points: u32,
    /// Independently registered combat-state owner. Missing skull evidence is
    /// unavailable; ordinary group membership cannot manufacture a clear skull.
    pub black_skull: Option<CurrentProjection<bool>>,
    pub premium: Option<CurrentProjection<bool>>,
    pub learned: Option<CurrentProjection<BTreeSet<String>>>,
    pub wheel: Option<CurrentProjection<BTreeMap<String, u8>>>,
    pub wheel_effects:
        Option<CurrentProjection<oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects>>,
    /// Durable promotion read in the same Wheel cast transaction as `wheel`. The build keeps
    /// the base vocation, so a promoted vocation name never proves promotion.
    pub wheel_promoted: bool,
    pub elemental_stance: Option<CurrentProjection<super::native_combat::ElementalStance>>,
    pub magnitude: Option<super::magnitude_owner::MagnitudeOwnedAttributes>,
}
#[derive(Debug, Clone)]
pub struct OwnedCastFacts {
    pub binding: CastFactsBinding,
    build: DurableBuildState,
    level: u32,
    equipment: EquipmentSnapshot,
    quiver_ammunition: Vec<EquippedItem>,
    pub access: AccessProjections,
    native_health_observations: Vec<oteryn_gameplay_entities::NativeHealthChangeObservation>,
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
            quiver_ammunition: Vec::new(),
            access,
            native_health_observations: Vec::new(),
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
    pub fn current_wheel_effects(
        &self,
        now: u64,
    ) -> Option<&oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects> {
        self.access
            .wheel_effects
            .as_ref()
            .filter(|value| value.current(&self.binding, now))
            .filter(|_| {
                self.access.wheel_promoted
                    && self
                        .current_premium(now)
                        .is_some_and(|premium| premium.value)
            })
            .map(|value| &value.value)
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
    /// Opaque, current source observations supplied by the fenced composition
    /// loader. The magnitude owner independently validates them before use.
    pub fn set_native_health_observations(
        &mut self,
        observations: Vec<oteryn_gameplay_entities::NativeHealthChangeObservation>,
    ) {
        self.native_health_observations = observations;
    }
    pub fn native_health_observations(
        &self,
    ) -> &[oteryn_gameplay_entities::NativeHealthChangeObservation] {
        &self.native_health_observations
    }
    pub fn magnitude(&self) -> Option<&super::magnitude_owner::MagnitudeOwnedAttributes> {
        self.access.magnitude.as_ref()
    }
    pub fn elemental_stance(&self, now: u64) -> Option<super::native_combat::ElementalStance> {
        self.access
            .elemental_stance
            .as_ref()
            .filter(|p| p.current(&self.binding, now))
            .map(|p| p.value)
    }
    pub fn elemental_stance_for_cast(
        &self,
        now: u64,
    ) -> Result<super::native_combat::ElementalStance, OwnedFactsError> {
        match &self.access.elemental_stance {
            Some(projection) if projection.current(&self.binding, now) => Ok(projection.value),
            Some(_) => Err(OwnedFactsError::StaleBinding),
            // Crystal 201189f8 energy_beam.lua:46–53 uses the base element until a stance is set.
            None => Ok(super::native_combat::ElementalStance::None),
        }
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
    fn main_hand_melee(
        &self,
        content: &NativeGameplayState,
    ) -> Option<oteryn_gameplay_combat_rules::resolution::SwingWeapon> {
        oteryn_gameplay_combat_rules::resolution::swing_weapon(content, |slot| {
            (slot == 5)
                .then(|| self.equipment.items.iter().find(|item| item.slot == slot))
                .flatten()
                .map(|item| {
                    (
                        item.definition.production_key.as_str(),
                        item.definition.revision_ref.as_str(),
                    )
                })
        })
    }

    /// Ammunition read under the same equipment transaction as the launcher.
    pub fn with_quiver_ammunition(mut self, ammunition: Vec<EquippedItem>) -> Self {
        self.quiver_ammunition = ammunition;
        self
    }

    fn ranged_skill_attack(
        &self,
        content: &NativeGameplayState,
        profile: &ProjectV2RangedWeapon,
    ) -> Option<IntrinsicWeaponDamage> {
        match profile {
            ProjectV2RangedWeapon::Throwing {
                attack,
                elemental_attack,
                element,
                ..
            } => Some(IntrinsicWeaponDamage {
                physical_attack: u32::from(*attack),
                elemental_attack: u32::from(*elemental_attack),
                element: *element,
            }),
            ProjectV2RangedWeapon::Distance {
                attack: launcher,
                ammunition,
                ..
            } => self
                .quiver_ammunition
                .iter()
                .chain(self.equipment.items.iter().filter(|item| item.slot == 10))
                .filter(|item| item.quantity > 0)
                .find_map(|item| {
                    let key = &item.definition.production_key;
                    let revision = &item.definition.revision_ref;
                    let equipment = content.equipment_policy(key, revision)?;
                    let claims = equipment
                        .claims()
                        .iter()
                        .find(|(slot, _)| *slot == 10)?
                        .1
                        .as_ref()
                        .ok()?;
                    if self.level < u32::from(claims.level) {
                        return None;
                    }
                    if !claims.vocations.is_empty() {
                        use super::Vocation;
                        use EquipmentBaseVocation as Base;
                        let vocation = match Vocation::from_key(self.build.vocation())? {
                            Vocation::None => Base::None,
                            Vocation::Druid | Vocation::ElderDruid => Base::Druid,
                            Vocation::Knight | Vocation::EliteKnight => Base::Knight,
                            Vocation::Monk | Vocation::ExaltedMonk => Base::Monk,
                            Vocation::Paladin | Vocation::RoyalPaladin => Base::Paladin,
                            Vocation::Sorcerer | Vocation::MasterSorcerer => Base::Sorcerer,
                        };
                        if !claims.vocations.contains(&vocation) {
                            return None;
                        }
                    }
                    let projectile = content.item_policy(key, revision)?;
                    let ProjectV2RangedWeapon::Ammunition {
                        attack,
                        ammunition: kind,
                        elemental_attack,
                        element,
                        ..
                    } = projectile.record().authoring.ranged.as_ref()?
                    else {
                        return None;
                    };
                    (kind == ammunition).then_some(IntrinsicWeaponDamage {
                        physical_attack: u32::from(*launcher) + u32::from(*attack),
                        elemental_attack: u32::from(*elemental_attack),
                        element: *element,
                    })
                }),
            _ => None,
        }
    }

    /// Weapon-skill callbacks divide damage by the qualified attack points of the
    /// held weapon, or by the launcher plus the compatible projectile.
    pub fn intrinsic_weapon_for_spell(
        &self,
        spell: &SpellDefinition,
        content: &NativeGameplayState,
    ) -> Option<IntrinsicWeaponDamage> {
        if !uses_attack_formula(spell) {
            return None;
        }
        let item = self.equipment.items.iter().find(|item| item.slot == 5)?;
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        if let Some(profile) = &policy.record().authoring.ranged {
            return self.ranged_skill_attack(content, profile);
        }
        IntrinsicWeaponDamage::from_melee(self.main_hand_melee(content)?)
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
            .map_err(|_| OwnedFactsError::BuildMismatch)?
            .with_wheel(self.current_wheel_effects(now));
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
        let formula = if self.access.loyalty_points == 0 {
            None
        } else {
            Some(
                content
                    .training_formula()
                    .ok_or(OwnedFactsError::BuildMismatch)?,
            )
        };
        let loyalty_level = |family, (level, progress)| {
            formula.map_or(level, |formula| {
                oteryn_gameplay_progression::loyalty::projected_level(
                    formula,
                    self.build.vocation(),
                    family,
                    level,
                    progress,
                    self.access.loyalty_points,
                )
            })
        };
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
        let skill_formula = uses_attack_formula(spell);
        let needs_attack = spell.needs_weapon || skill_formula;
        let mut attack = None;
        let mut shield = None;
        // Source-defined first shield is left hand then right. The snapshot is the actual owner.
        for slot in [6_u8, 5] {
            let reads_weapon = needs_attack && slot == 5;
            let reads_shield = spell.needs_shield && shield.is_none();
            if !reads_weapon && !reads_shield {
                continue;
            }
            if let Some(item) = self.equipment.items.iter().find(|i| i.slot == slot) {
                let policy = content
                    .item_policy(
                        &item.definition.production_key,
                        &item.definition.revision_ref,
                    )
                    .ok_or(OwnedFactsError::UnknownItemPolicy)?;
                let weapon = match &policy.record().semantics.weapon {
                    ReferenceItemField::Known(weapon) => weapon,
                    ReferenceItemField::NotApplicable => continue,
                    _ => return Err(OwnedFactsError::UnavailableEquipment),
                };
                let ReferenceItemField::Known(kind) = weapon.weapon_type else {
                    return Err(OwnedFactsError::UnavailableEquipment);
                };
                if kind == ReferenceWeaponType::Shield {
                    if reads_shield {
                        let ReferenceItemField::Known(defense) = weapon.defense else {
                            return Err(OwnedFactsError::UnavailableEquipment);
                        };
                        shield = Some(
                            u32::try_from(defense.0)
                                .map_err(|_| OwnedFactsError::UnavailableEquipment)?,
                        );
                    }
                } else if reads_weapon {
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
                        let ranged = policy.record().authoring.ranged.as_ref();
                        let attack_value = if skill < 4 {
                            self.main_hand_melee(content)
                                .ok_or(OwnedFactsError::UnavailableEquipment)?
                                .attack(0)
                        } else if let Some(damage) =
                            ranged.and_then(|profile| self.ranged_skill_attack(content, profile))
                        {
                            damage.total_attack()
                        } else if matches!(ranged, Some(ProjectV2RangedWeapon::Distance { .. })) {
                            // A launcher without a usable projectile supplies no attack.
                            continue;
                        } else {
                            u32::try_from(value.0)
                                .map_err(|_| OwnedFactsError::UnavailableEquipment)?
                        };
                        attack = Some((
                            skill_adjustments
                                .adjust_skill(
                                    u32::from(loyalty_level(skill + 1, self.build.skills()[skill])),
                                    skill,
                                )
                                .and_then(|value| {
                                    value.checked_add(self.access.proficiency_skills[skill])
                                })
                                .ok_or(OwnedFactsError::BuildMismatch)?,
                            attack_value,
                            if needs_attack {
                                self.equipment
                                    .combat_mode
                                    .unwrap_or(CombatMode::Attack)
                                    .attack_factor()
                            } else {
                                0.0
                            },
                            skill < 4,
                        ));
                    }
                }
            }
        }
        // Initial casts admit equipment in source order with the other cast rules.
        // An already accepted callback has no admission phase to check required inputs.
        if !require_cast_access
            && ((spell.needs_weapon && attack.is_none())
                || (spell.needs_shield && shield.is_none()))
        {
            return Err(OwnedFactsError::UnavailableEquipment);
        }
        // Crystal COMBAT_FORMULA_SKILL initializes these inputs before looking up the weapon.
        let unarmed_attack = if skill_formula { 7 } else { 0 };
        let (attack_skill, attack_value, attack_factor, melee_weapon) =
            attack.unwrap_or((0, unarmed_attack, 0.0, false));
        let harmony = if spell.harmony_role == Some(super::HarmonyRole::Spender) {
            if let Some(monk) = &state.monk {
                let stage = if self.has_current_wheel(now) {
                    self.wheel_stage("ascetic", now)?
                } else {
                    0
                };
                HarmonyMultiplier::with_ascetic(
                    facts.level, monk.harmony(),
                    state.standard_stance() == Some(oteryn_gameplay_spell_rules::native_actor_states::StandardStance::Harmony),
                    monk.serene(), stage,
                ).ok_or(OwnedFactsError::UnavailableWheel)?
            } else {
                harmony
            }
        } else {
            harmony
        };
        Ok(CasterState {
            vocation: facts.vocation,
            level: facts.level,
            magic_level: skill_adjustments
                .adjust_magic_level(u32::from(loyalty_level(0, self.build.magic())))
                .and_then(|value| value.checked_add(self.access.proficiency_skills[7]))
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
                .adjust_shielding(u32::from(loyalty_level(6, self.build.skills()[5])))
                .and_then(|value| value.checked_add(self.access.proficiency_skills[5]))
                .ok_or(OwnedFactsError::BuildMismatch)?,
            melee_weapon,
            shield_defense: shield,
            harmony_multiplier: harmony,
        })
    }
}
fn uses_attack_formula(spell: &SpellDefinition) -> bool {
    matches!(&spell.execution, super::Execution::NativeProfile(profile)
                if attack_input(&profile.spell()["execution"]))
        || spell.authored.as_ref().is_some_and(|p| {
            p.dependencies.formulas.iter().any(|f| {
                f.minimum.as_ref().is_some_and(attack_input)
                    || f.maximum.as_ref().is_some_and(attack_input)
            })
        })
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
