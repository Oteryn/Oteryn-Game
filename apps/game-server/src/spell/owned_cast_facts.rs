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
pub(crate) struct CastFactsBinding {
    pub(crate) actor: ExactActorRef,
    pub(crate) session: GameSessionId,
    pub(crate) character: [u8; 16],
    pub(crate) character_revision: u64,
    pub(crate) lease_generation: u64,
    pub(crate) connection_generation: u64,
    pub(crate) player_revision: u64,
    pub(crate) content_digest: [u8; 32],
    pub(crate) equipment_revision: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CurrentProjection<T> {
    pub(crate) binding: CastFactsBinding,
    pub(crate) authority_revision: u64,
    pub(crate) valid_until_micros: u64,
    pub(crate) value: T,
}
impl<T> CurrentProjection<T> {
    pub(crate) fn current(&self, binding: &CastFactsBinding, now: u64) -> bool {
        &self.binding == binding && self.authority_revision > 0 && now < self.valid_until_micros
    }
}
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct AccessProjections {
    /// Independently registered combat-state owner. Missing skull evidence is
    /// unavailable; ordinary group membership cannot manufacture a clear skull.
    pub(crate) black_skull: Option<CurrentProjection<bool>>,
    pub(crate) premium: Option<CurrentProjection<bool>>,
    pub(crate) learned: Option<CurrentProjection<BTreeSet<String>>>,
    pub(crate) wheel: Option<CurrentProjection<BTreeMap<String, u8>>>,
    pub(crate) elemental_stance: Option<CurrentProjection<super::native_combat::ElementalStance>>,
    pub(crate) magnitude: Option<super::magnitude_owner::MagnitudeOwnedAttributes>,
}
#[derive(Debug, Clone)]
pub(crate) struct OwnedCastFacts {
    binding: CastFactsBinding,
    build: DurableBuildState,
    level: u32,
    equipment: EquipmentSnapshot,
    access: AccessProjections,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnedFactsError {
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
    pub(crate) fn current_black_skull(&self, now: u64) -> Option<&CurrentProjection<bool>> {
        self.access
            .black_skull
            .as_ref()
            .filter(|p| p.current(&self.binding, now))
    }
    /// The producer is the independently fenced gameplay loader, never a decoded client packet.
    pub(crate) fn from_owner_reads(
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
    pub(crate) fn current_premium(&self, now: u64) -> Option<&CurrentProjection<bool>> {
        self.access
            .premium
            .as_ref()
            .filter(|value| value.current(&self.binding, now))
    }
    pub(crate) fn binding(&self) -> &CastFactsBinding {
        &self.binding
    }
    pub(crate) fn wheel_revision(&self) -> Option<u64> {
        self.access.wheel.as_ref().map(|p| p.authority_revision)
    }
    pub(crate) fn durable_build(&self) -> &DurableBuildState {
        &self.build
    }
    /// An original pending roll cannot grant a now-expired account/learning benefit.
    pub(crate) fn check_current_access(
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
    pub(crate) fn equipment(&self) -> &EquipmentSnapshot {
        &self.equipment
    }
    pub(crate) fn magnitude(&self) -> Option<&super::magnitude_owner::MagnitudeOwnedAttributes> {
        self.access.magnitude.as_ref()
    }
    pub(crate) fn elemental_stance(
        &self,
        now: u64,
    ) -> Option<super::native_combat::ElementalStance> {
        self.access
            .elemental_stance
            .as_ref()
            .filter(|p| p.current(&self.binding, now))
            .map(|p| p.value)
    }
    pub(crate) fn equipment_speed_bonus(
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
    pub(crate) fn wheel_stage(&self, perk: &str, now: u64) -> Result<u8, OwnedFactsError> {
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
    pub(crate) fn caster(
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
    pub(crate) fn numerical_caster(
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
            premium: premium.is_some_and(|p| p.value),
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
fn attack_input(value: &serde_json::Value) -> bool {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    fn binding() -> CastFactsBinding {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 3,
            content_digest: [9; 32],
            equipment_revision: 2,
        }
    }
    #[test]
    fn current_projection_requires_every_owner_revision_and_unexpired_authority() {
        let expected = binding();
        let mut projection = CurrentProjection {
            binding: expected.clone(),
            authority_revision: 7,
            valid_until_micros: 100,
            value: true,
        };
        assert!(projection.current(&expected, 99));
        assert!(!projection.current(&expected, 100));
        projection.authority_revision = 0;
        assert!(!projection.current(&expected, 0));
        projection.authority_revision = 7;
        for field in 0..5 {
            let mut replaced = expected.clone();
            match field {
                0 => replaced.character_revision += 1,
                1 => replaced.equipment_revision += 1,
                2 => replaced.lease_generation += 1,
                3 => replaced.player_revision += 1,
                _ => replaced.content_digest = [8; 32],
            }
            assert!(!projection.current(&replaced, 0));
        }
    }
    #[test]
    fn owner_snapshot_rejects_mixed_equipment_and_keeps_missing_grants_unknown() {
        let binding = binding();
        let build = DurableBuildState::new("knight", (0, 0), [(10, 0); 7]).expect("explicit build");
        let mut equipment = EquipmentSnapshot {
            character: binding.character,
            content_digest: binding.content_digest,
            character_revision: binding.character_revision,
            revision: binding.equipment_revision,
            combat_mode: None,
            items: vec![],
        };
        equipment.revision += 1;
        assert!(matches!(
            OwnedCastFacts::from_owner_reads(
                binding.clone(),
                build.clone(),
                8,
                equipment.clone(),
                AccessProjections::default()
            ),
            Err(OwnedFactsError::StaleBinding)
        ));
        equipment.revision -= 1;
        let owned = OwnedCastFacts::from_owner_reads(
            binding,
            build,
            8,
            equipment,
            AccessProjections::default(),
        )
        .expect("matching owner reads");
        assert_eq!(
            owned.wheel_stage("unknown", 0),
            Err(OwnedFactsError::UnavailableWheel)
        );
        assert!(owned.magnitude().is_none());
        assert!(owned.access.premium.is_none());
        assert!(owned.access.learned.is_none());
    }
    #[test]
    fn accepted_source_chain_due_keeps_numeric_owners_after_premium_expires() {
        use crate::content::native_gameplay::{
            NativeGameplayInput, NativeGameplayMapProfile, PinnedGameplayBytes,
            compile_native_gameplay, decode,
        };
        use sha2::{Digest, Sha256};
        let pinned = |bytes: &[u8]| PinnedGameplayBytes {
            bytes: bytes.to_vec(),
            sha256: Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        };
        let input = NativeGameplayInput {
            native_map_profile: NativeGameplayMapProfile::AcceptedEntryR1,
            catalog: pinned(include_bytes!(
                "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
            )),
            source_selection: pinned(include_bytes!(
                "../../../../tools/content-schema/spell-authoring/samples/executable-spell-source-selection.json"
            )),
            creature_profiles: pinned(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/creature_profiles.json"
            )),
            presentation_profiles: pinned(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/presentation_profiles.json"
            )),
            item_profiles: None,
            spell_appearances: None,
            build_training: None,
            familiar_config: None,
            familiar_defenses: None,
            wheel_profile: None,
            source_world: None,
        };
        let (runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        let world = runtime.binding().world_id();
        let baseline =
            crate::content::qualify_native_entry_room(world).expect("actual accepted map");
        let wrapped = compile_native_gameplay(baseline.compiled(), &input)
            .expect("actual compiled owner source");
        let content = decode(&wrapped.server_artifact)
            .expect("qualified content")
            .state;
        let catalog: serde_json::Value =
            serde_json::from_slice(&input.catalog.bytes).expect("catalog");
        let row = catalog["bundles"]
            .as_array()
            .expect("bundles")
            .iter()
            .find(|row| row["bundle"]["spell"]["identity"]["key"] == "candidate:spell/lightning")
            .expect("source chain");
        let spell =
            crate::spell::authoring::spell_from_bundle(&row["bundle"], &row["dependencies"])
                .expect("closed genuine source");
        let state = PlayerSpellState::new(
            crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::Sorcerer,
                level: 100,
                magic_level: 50,
                max_health: 500,
                max_mana: 5000,
                max_soul: 100,
            },
            0,
            0,
        )
        .expect("actual player");
        let mut b = binding();
        b.player_revision = state.revision();
        b.content_digest = content.source_digest();
        let premium = CurrentProjection {
            binding: b.clone(),
            authority_revision: 1,
            valid_until_micros: 10,
            value: true,
        };
        let build =
            DurableBuildState::new("sorcerer", (50, 0), [(10, 0); 7]).expect("actual build");
        let equipment = EquipmentSnapshot {
            character: b.character,
            content_digest: b.content_digest,
            character_revision: b.character_revision,
            revision: b.equipment_revision,
            combat_mode: None,
            items: vec![],
        };
        let owned = OwnedCastFacts::from_owner_reads(
            b,
            build,
            100,
            equipment,
            AccessProjections {
                premium: Some(premium),
                ..AccessProjections::default()
            },
        )
        .expect("matched owners");
        let accepted = owned
            .caster(&state, &spell, &content, HarmonyMultiplier::ONE, 9)
            .expect("initial admitted source cast");
        assert_eq!(accepted.magic_level, 50);
        assert!(matches!(
            owned.caster(&state, &spell, &content, HarmonyMultiplier::ONE, 10),
            Err(OwnedFactsError::UnavailablePremium)
        ));
        let due = owned
            .numerical_caster(&state, &spell, &content, HarmonyMultiplier::ONE, 10)
            .expect("already accepted numerical phase");
        assert_eq!(due.magic_level, accepted.magic_level);
        let effects =
            crate::spell::cast::resolve_ordinary_due_effects(&spell, &due, 1, &mut |minimum, _| {
                minimum
            })
            .expect("real source due formula");
        assert!(
            matches!(&effects[0],crate::spell::ResolvedEffect::Damage{magnitude,..} if *magnitude > 0)
        );
        let mut changed = state.clone();
        changed
            .advance_batch_revision()
            .expect("independent owner revision");
        assert!(matches!(
            owned.numerical_caster(&changed, &spell, &content, HarmonyMultiplier::ONE, 10),
            Err(OwnedFactsError::StaleBinding)
        ));
    }
    #[test]
    fn attack_dependencies_are_found_inside_native_parameter_asts() {
        assert!(attack_input(
            &serde_json::json!({"native_behavior":{"parameters":{"minimum":{"multiply":[2,{"input":"attack_value"}]}}}})
        ));
        assert!(!attack_input(&serde_json::json!({"input":"magic_level"})));
    }
}
