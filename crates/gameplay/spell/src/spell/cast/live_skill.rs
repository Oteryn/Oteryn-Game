//! MELEE-SKILL-1: the live attack and shielding skills an auto-attack swing reads.

use std::sync::Arc;

use oteryn_gameplay_progression::loyalty;

use super::PlayerSpellState;
use crate::durability::character_build::CombatLoyaltyProjection;
use crate::spell::mana_training::CompiledTrainingFormula;

/// The shielding skill's index in the seven owned skills.
const SHIELDING: usize = 5;

/// The live skills of one swing: the swung weapon's skill and the shielding skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwingSkills {
    pub attack: u32,
    pub shielding: u32,
}

/// The account projection and training curve captured at fenced admission.
#[derive(Debug)]
pub(super) struct CombatLoyalty {
    projection: CombatLoyaltyProjection,
    formula: CompiledTrainingFormula,
}

impl PartialEq for CombatLoyalty {
    fn eq(&self, other: &Self) -> bool {
        self.projection == other.projection && self.formula.digest == other.formula.digest
    }
}
impl Eq for CombatLoyalty {}

impl PlayerSpellState {
    /// Install a current account projection. A newer zero value clears the combat bonus.
    pub fn set_combat_loyalty(
        &mut self,
        projection: CombatLoyaltyProjection,
        formula: &CompiledTrainingFormula,
    ) {
        if projection.platform_revision == 0
            || self.combat_loyalty.as_ref().is_some_and(|current| {
                current.projection.platform_revision > projection.platform_revision
                    || (current.projection.platform_revision == projection.platform_revision
                        && current.formula.digest == formula.digest)
            })
        {
            return;
        }
        self.combat_loyalty = Some(Arc::new(CombatLoyalty {
            projection,
            formula: formula.clone(),
        }));
    }

    fn loyalty_skill(&self, index: usize, base: u16) -> u32 {
        let (Some(loyalty), Some(training)) = (&self.combat_loyalty, &self.training) else {
            return u32::from(base);
        };
        // Receipt installation advances durable(), while banked tries stay pending.
        let build = training.durable();
        let (level, tries) = build.skills()[index];
        u32::from(loyalty::projected_level(
            &loyalty.formula,
            build.vocation(),
            index + 1,
            level,
            tries,
            loyalty.projection.points,
        ))
    }

    /// One committed skill with loyalty and current condition adjustments applied.
    pub fn live_skill(&self, index: usize, now_us: u64) -> Option<u32> {
        let owned = self.owned_skills?;
        let adjustments = self.owned_skill_adjustments(now_us).ok()?;
        let base = *owned.get(index)?;
        adjustments.adjust_projected_skill(self.loyalty_skill(index, base), u32::from(base), index)
    }

    /// The live swing skills at `now_us`: the committed skills with loyalty applied before
    /// condition adjustments. `Some(None)` when no durable build was loaded, which fresh
    /// admission refuses, so the caller keeps the ATTACK-1 starting skill. `None` (no swing) when
    /// `attack_index` is not a skill or a live adjustment does not resolve.
    #[must_use]
    pub fn swing_skills(&self, attack_index: usize, now_us: u64) -> Option<Option<SwingSkills>> {
        let Some(owned) = self.owned_skills else {
            return Some(None);
        };
        let adjustments = self.owned_skill_adjustments(now_us).ok()?;
        let skill = |index| {
            let base = *owned.get(index)?;
            adjustments.adjust_projected_skill(
                self.loyalty_skill(index, base),
                u32::from(base),
                index,
            )
        };
        let attack = skill(attack_index)?;
        let shielding = if attack_index == SHIELDING {
            attack
        } else {
            skill(SHIELDING)?
        };
        Some(Some(SwingSkills { attack, shielding }))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::spell::Vocation;
    use crate::spell::cast::CharacterCastFacts;
    use oteryn_gameplay_spell_rules::native_actor_states::StandardStance;

    fn state(vocation: Vocation) -> PlayerSpellState {
        PlayerSpellState::new(
            CharacterCastFacts {
                vocation,
                level: 8,
                magic_level: 0,
                max_health: 185,
                max_mana: 90,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap()
    }

    fn formula() -> CompiledTrainingFormula {
        CompiledTrainingFormula::from_profile(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/native-gameplay/build-training.json"
            )),
            "test",
        )
        .unwrap()
    }

    #[test]
    fn an_actor_without_a_loaded_build_keeps_the_starting_skill() {
        assert_eq!(state(Vocation::Knight).swing_skills(2, 0), Some(None));
    }

    #[test]
    fn the_swing_reads_the_owned_weapon_and_shielding_skills() {
        let mut state = state(Vocation::Knight);
        state.owned_skills = Some([11, 12, 42, 14, 15, 37, 10]);
        assert_eq!(
            state.swing_skills(2, 0),
            Some(Some(SwingSkills {
                attack: 42,
                shielding: 37
            }))
        );
        assert_eq!(state.swing_skills(0, 0).unwrap().unwrap().attack, 11);
        assert_eq!(state.swing_skills(7, 0), None, "no eighth skill");
    }

    #[test]
    fn loyalty_uses_trained_skill_for_condition_percentages() {
        use crate::durability::character_build::{BuildFormula, DurableBuildState};
        use oteryn_gameplay_domain::domain::condition::ConditionValues;
        use oteryn_gameplay_domain::domain::condition::attributes::{
            AttributeModifier, AttributeModifiers,
        };
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind,
        };
        use oteryn_gameplay_progression::hit_training::SkillFamily;
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

        let formula = formula();
        let mut state = state(Vocation::Paladin);
        let build = DurableBuildState::new("paladin", (0, 0), [(100, 0); 7]).unwrap();
        state.load_owned_build(&build).unwrap();
        state.enable_owned_training(&build, &formula).unwrap();
        state.set_combat_loyalty(
            CombatLoyaltyProjection {
                points: 3_600,
                platform_revision: 1,
            },
            &formula,
        );

        let pending = formula.required("paladin", 3, 101).unwrap() - 1;
        assert!(
            !state
                .train_skill(SkillFamily::Sword, 0, &formula, |_| pending)
                .unwrap()
                .checkpoint_due
        );
        assert_eq!(state.live_skill(2, 0), Some(102));
        // Crystal fishing.lua reads the effective skill, which includes loyalty.
        assert_eq!(state.live_skill(6, 0), Some(104));

        let definition = ConditionDefinition::new(
            "source.native.attributes",
            1,
            ConditionValues::SourceAttributes {
                modifiers: AttributeModifiers {
                    melee: Some(AttributeModifier::PercentOfBase(25)),
                    ..Default::default()
                },
                duration_ms: 1_000,
            },
        )
        .unwrap();
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &GameplayDecisionRoot::from_bytes([0; 32]),
            occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
        };
        state
            .conditions
            .apply(&definition, None, ConditionSourceKind::SelfUse, &[], &facts)
            .unwrap();
        assert_eq!(state.swing_skills(2, 0).unwrap().unwrap().attack, 27);
    }

    #[test]
    fn justice_uses_trained_fist_skill_with_loyalty() {
        use crate::durability::character_build::DurableBuildState;

        let formula = formula();
        let mut state = state(Vocation::Monk);
        let mut skills = [(10, 0); 7];
        skills[0] = (110, 0);
        let build = DurableBuildState::new("monk", (0, 0), skills).unwrap();
        state.load_owned_build(&build).unwrap();
        state.enable_owned_training(&build, &formula).unwrap();
        state.set_combat_loyalty(
            CombatLoyaltyProjection {
                points: 3_600,
                platform_revision: 1,
            },
            &formula,
        );
        assert_eq!(state.live_skill(0, 0), Some(114));

        state.stance = Some(StandardStance::Justice);
        assert_eq!(state.swing_skills(0, 0).unwrap().unwrap().attack, 122);
    }

    #[test]
    fn a_newer_zero_revision_clears_loyalty_and_cannot_be_overwritten_by_a_stale_read() {
        use crate::durability::character_build::DurableBuildState;

        let formula = formula();
        let mut state = state(Vocation::Paladin);
        let build = DurableBuildState::new("paladin", (0, 0), [(100, 0); 7]).unwrap();
        state.load_owned_build(&build).unwrap();
        state.enable_owned_training(&build, &formula).unwrap();
        let original = CombatLoyaltyProjection {
            points: 3_600,
            platform_revision: 1,
        };
        state.set_combat_loyalty(original, &formula);
        assert_eq!(state.live_skill(2, 0), Some(102));

        state.set_combat_loyalty(
            CombatLoyaltyProjection {
                points: 0,
                platform_revision: 2,
            },
            &formula,
        );
        state.set_combat_loyalty(original, &formula);
        assert_eq!(state.live_skill(2, 0), Some(100));
    }
}
