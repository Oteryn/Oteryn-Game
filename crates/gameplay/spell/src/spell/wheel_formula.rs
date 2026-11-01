//! Per-cast Wheel formula inputs from the existing current build and physical owners.
//! These numerical observations never modify the durable base skill or spell admission facts.
use super::owned_cast_facts::{OwnedCastFacts, OwnedFactsError};
use super::{
    Carrier, CastRejection, CasterState, FormulaError, FormulaInputs, SpellBook, SpellDefinition,
    SpellEffect,
};
use crate::foundation::ChannelRuntimeV1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WheelFormulaMagic {
    specialized_magic: u32,
    distance: u32,
    melee: u32,
    shielding: u32,
    runic_percent: u32,
}

impl WheelFormulaMagic {
    /// The same current census supplies Battle Instinct and both Positional Tactics branches.
    pub fn from_position(
        owned: &OwnedCastFacts,
        runtime: &ChannelRuntimeV1,
        state: &super::cast::PlayerSpellState,
        now_us: u64,
    ) -> Result<Self, OwnedFactsError> {
        let mut result = Self::default();
        let Some(effects) = owned.current_wheel_effects(now_us) else {
            return Ok(result);
        };
        let battle = effects.active_unique_perks.contains("battle_instinct");
        let positional = effects.active_unique_perks.contains("positional_tactics");
        if battle || positional {
            runtime
                .player_control_facts(owned.binding.actor, owned.binding.session)
                .map_err(|_| OwnedFactsError::StaleBinding)?;
            let count = super::cast::wheel_adjacent_monsters(runtime, owned.binding.actor)
                .ok_or(OwnedFactsError::StaleBinding)?;
            if battle {
                let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
                    .map_err(|_| OwnedFactsError::UnavailableWheel)?;
                let bonus = oteryn_gameplay_progression::wheel_effects::battle_instinct_bonuses(
                    data.wheel(),
                    count,
                )
                .ok_or(OwnedFactsError::UnavailableWheel)?;
                result.melee = if state
                    .wheel_equipment()
                    .is_some_and(super::cast::WheelEquipment::battle_melee)
                {
                    bonus.melee
                } else {
                    0
                };
                result.shielding = bonus.shielding;
            }
            if positional {
                if count == 0 {
                    result.distance = 3;
                } else {
                    result.specialized_magic = 3;
                }
            }
        }
        Ok(result)
    }

    pub fn from_owners(
        owned: &OwnedCastFacts,
        runtime: &ChannelRuntimeV1,
        spell: &SpellDefinition,
        book: &SpellBook,
        caster: &CasterState,
        state: &super::cast::PlayerSpellState,
        now_us: u64,
    ) -> Result<Self, OwnedFactsError> {
        let mut result = Self::from_position(owned, runtime, state, now_us)?;
        let Some(effects) = owned.current_wheel_effects(now_us) else {
            return Ok(result);
        };
        if matches!(spell.carrier, Carrier::Rune { .. })
            && effects.active_unique_perks.contains("runic_mastery")
        {
            // Crystal combat.cpp:43-59,1992-2009: an actual distinct same-name instant
            // definition is required (spells.cpp:244-251 compares names case-insensitively).
            // spells.cpp:1408-1431 canCast does not test mana,
            // level, soul, cooldown or Premium. S16's admitted ordinary player flags are
            // empty and its global no-learning policy is already in learning_required.
            if let Some(conjurer) = book.spells.iter().find(|candidate| {
                candidate.key != spell.key
                    && candidate.name.eq_ignore_ascii_case(&spell.name)
                    && matches!(candidate.carrier, Carrier::Instant { .. })
            }) {
                let can_cast = if conjurer.learning_required {
                    owned
                        .access
                        .learned
                        .as_ref()
                        .filter(|read| read.current(&owned.binding, now_us))
                        .ok_or(OwnedFactsError::UnavailableLearning)?
                        .value
                        .contains(&conjurer.key)
                } else {
                    conjurer.vocations.is_empty() || conjurer.vocations.contains(&caster.vocation)
                };
                result.runic_percent = if can_cast { 20 } else { 10 };
            }
        }
        Ok(result)
    }

    pub(super) fn roll_magic_level(
        self,
        effective_magic: u32,
        draw: &mut dyn FnMut(i64, i64) -> i64,
    ) -> Result<u32, CastRejection> {
        if self.runic_percent == 0 {
            return Ok(effective_magic);
        }
        let roll = oteryn_gameplay_spell_rules::native_actor_states::normal_draw(0, 100, draw)
            .map_err(|_| CastRejection::Formula(FormulaError::NonFinite))?;
        if roll > 25 {
            return Ok(effective_magic);
        }
        // Primary donor uses effective getMagicLevel(), then adds specialized magic.
        u32::try_from(
            u64::from(effective_magic)
                + u64::from(effective_magic) * u64::from(self.runic_percent) / 100,
        )
        .map_err(|_| CastRejection::Formula(FormulaError::NonFinite))
    }

    pub(super) fn skill_inputs(
        self,
        inputs: &mut FormulaInputs,
        melee: bool,
    ) -> Result<(), FormulaError> {
        let attack = if melee {
            self.melee
        } else if inputs.attack_value > 0 {
            self.distance
        } else {
            0
        };
        inputs.attack_skill = inputs
            .attack_skill
            .checked_add(attack)
            .ok_or(FormulaError::NonFinite)?;
        inputs.shielding_skill = inputs
            .shielding_skill
            .checked_add(self.shielding)
            .ok_or(FormulaError::NonFinite)?;
        Ok(())
    }

    pub(super) fn effect_inputs(
        self,
        effect: &SpellEffect,
        inputs: &FormulaInputs,
    ) -> Result<FormulaInputs, FormulaError> {
        let mut inputs = *inputs;
        if matches!(effect, SpellEffect::Heal { .. })
            || matches!(effect, SpellEffect::Damage { damage_type, .. } if damage_type == "holy")
        {
            inputs.magic_level = inputs
                .magic_level
                .checked_add(self.specialized_magic)
                .ok_or(FormulaError::NonFinite)?;
        }
        Ok(inputs)
    }

    /// Native spatial facts use the same nonrandom positional/skill projection; rune
    /// formula draws are owned by the ordinary rune path, not by spatial capture.
    pub fn native_inputs(
        self,
        inputs: &mut FormulaInputs,
        melee: bool,
        element: Option<&str>,
    ) -> Result<(), FormulaError> {
        self.skill_inputs(inputs, melee)?;
        if matches!(element, Some("holy" | "healing")) {
            inputs.magic_level = inputs
                .magic_level
                .checked_add(self.specialized_magic)
                .ok_or(FormulaError::NonFinite)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::super::formula::{Expression, Formula, Input};
    use super::super::harmony::HarmonyMultiplier;
    use super::super::{CasterPremium, Cooldowns, Execution, ManaCost, ResolvedEffect, Vocation};
    use super::*;
    use oteryn_simulation_determinism::SemanticTimeMicros;

    fn caster() -> CasterState {
        CasterState {
            vocation: Vocation::Druid,
            level: 500,
            magic_level: 101,
            premium: CasterPremium::from_current(true),
            mana: 1000,
            max_mana: 1000,
            soul: 200,
            learned: Default::default(),
            attack_skill: 80,
            attack_value: 30,
            attack_factor: 1.0,
            shielding_skill: 60,
            melee_weapon: false,
            shield_defense: None,
            harmony_multiplier: HarmonyMultiplier::ONE,
        }
    }
    fn spell() -> Result<SpellDefinition, String> {
        let mut spell = super::super::cast::v1_spell_book()?
            .spells
            .into_iter()
            .next()
            .ok_or_else(|| "spell".to_owned())?;
        spell.authored = None;
        spell.carrier = Carrier::Rune {
            item: 3155,
            charges: 1,
            magic_level: 0,
        };
        spell.vocations = [Vocation::Druid].into_iter().collect();
        spell.level = 1;
        spell.mana = ManaCost::Fixed(0);
        spell.soul = 0;
        spell.learning_required = false;
        spell.needs_target = false;
        spell.allowed_targets = super::super::target::AllowedTargets::Any;
        spell.chain = None;
        spell.execution = Execution::Effects(vec![SpellEffect::Heal {
            formula: Formula {
                minimum: Expression::Var(Input::MagicLevel),
                maximum: Expression::Var(Input::MagicLevel),
            },
        }]);
        Ok(spell)
    }

    #[test]
    fn runic_uses_existing_normal_distribution_effective_magic_and_integer_bonus()
    -> Result<(), String> {
        for (percent, expected) in [(20, 121), (10, 111)] {
            let wheel = WheelFormulaMagic {
                runic_percent: percent,
                ..Default::default()
            };
            let mut draws = 0;
            assert_eq!(
                wheel
                    .roll_magic_level(101, &mut |low, high| {
                        assert_eq!((low, high), (0, i64::from(u32::MAX)));
                        draws += 1;
                        low
                    })
                    .map_err(|e| format!("{e:?}"))?,
                expected
            );
            assert_eq!(draws, 1);
            assert_eq!(
                wheel
                    .roll_magic_level(101, &mut |_, high| high / 4)
                    .map_err(|e| format!("{e:?}"))?,
                101,
                "the 25th uniform percentile is around normal34, not a25% proc"
            );
            assert_eq!(
                wheel
                    .roll_magic_level(101, &mut |_, high| high)
                    .map_err(|e| format!("{e:?}"))?,
                101
            );
            assert!(
                wheel
                    .roll_magic_level(101, &mut |_, high| high + 1)
                    .is_err()
            );
        }
        assert_eq!(
            WheelFormulaMagic::default()
                .roll_magic_level(101, &mut |_, _| panic!("inactive rune does not draw")),
            Ok(101)
        );
        Ok(())
    }

    #[test]
    fn runic_formula_is_after_unmodified_magic_and_resource_gates_and_draws_once()
    -> Result<(), String> {
        let caster = caster();
        let mut spell = spell()?;
        let wheel = WheelFormulaMagic {
            runic_percent: 20,
            specialized_magic: 3,
            ..Default::default()
        };
        spell.carrier = Carrier::Rune {
            item: 3155,
            charges: 1,
            magic_level: 102,
        };
        let now = SemanticTimeMicros::from_micros(1);
        let resolve = |spell: &SpellDefinition, draw: &mut dyn FnMut(i64, i64) -> i64| {
            super::super::resolve_with_wheel(
                spell,
                &caster,
                &Cooldowns::default(),
                now,
                false,
                super::super::Facts::None,
                wheel,
                draw,
            )
        };
        assert!(matches!(
            resolve(&spell, &mut |_, _| panic!("failed gate cannot roll Runic")),
            Err(CastRejection::MagicLevelTooLow { .. })
        ));
        spell.carrier = Carrier::Rune {
            item: 3155,
            charges: 1,
            magic_level: 0,
        };
        spell.mana = ManaCost::Fixed(1001);
        assert!(matches!(
            resolve(&spell, &mut |_, _| panic!(
                "failed payment cannot roll Runic"
            )),
            Err(CastRejection::NotEnoughMana { .. })
        ));
        spell.mana = ManaCost::Fixed(0);
        if let Execution::Effects(effects) = &mut spell.execution {
            effects.push(effects[0].clone());
        }
        let mut normal_draws = 0;
        let result = resolve(&spell, &mut |low, high| {
            if high == i64::from(u32::MAX) {
                normal_draws += 1;
            }
            low
        })
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(normal_draws, 1, "one rune Combat shares its Runic result");
        assert_eq!(
            result.effects,
            vec![
                ResolvedEffect::Heal { magnitude: 124 },
                ResolvedEffect::Heal { magnitude: 124 }
            ]
        );
        assert_eq!(
            caster.magic_level, 101,
            "numerical bonus never changes admission or durable base"
        );
        Ok(())
    }

    #[test]
    fn positional_specialization_and_battle_instinct_affect_only_their_formula_inputs()
    -> Result<(), String> {
        let mut inputs = FormulaInputs {
            level: 500,
            magic_level: 101,
            base_power: None,
            attack_skill: 80,
            attack_value: 30,
            attack_factor: 1.0,
            shielding_skill: 60,
            shield_defense: Some(35),
        };
        let wheel = WheelFormulaMagic {
            specialized_magic: 3,
            distance: 0,
            melee: 4,
            shielding: 24,
            runic_percent: 0,
        };
        wheel
            .skill_inputs(&mut inputs, true)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!((inputs.attack_skill, inputs.shielding_skill), (84, 84));
        let formula = Formula {
            minimum: Expression::Var(Input::MagicLevel),
            maximum: Expression::Var(Input::MagicLevel),
        };
        for (kind, expected) in [("holy", 104), ("fire", 101), ("physical", 101)] {
            let effect = SpellEffect::Damage {
                damage_type: kind.into(),
                formula: formula.clone(),
            };
            assert_eq!(
                wheel
                    .effect_inputs(&effect, &inputs)
                    .map_err(|e| format!("{e:?}"))?
                    .magic_level,
                expected
            );
        }
        assert_eq!(
            wheel
                .effect_inputs(&SpellEffect::Heal { formula }, &inputs)
                .map_err(|e| format!("{e:?}"))?
                .magic_level,
            104
        );
        let no_monsters = WheelFormulaMagic {
            distance: 3,
            ..Default::default()
        };
        let mut ranged = inputs;
        no_monsters
            .skill_inputs(&mut ranged, false)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(ranged.attack_skill, 87);
        Ok(())
    }
}
