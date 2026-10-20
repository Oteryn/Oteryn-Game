//! Temporary Character attributes. Base Character build values are never mutated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttributeModifier {
    PercentOfBase(i32),
    Add(i32),
}
impl AttributeModifier {
    fn valid(self) -> bool {
        match self {
            Self::PercentOfBase(n) => (0..=100).contains(&n),
            Self::Add(_) => true,
        }
    }
    fn effective(self, base: u32) -> Option<u32> {
        let delta = match self {
            // Canary interprets zero percent as an unset parameter, not zero skill.
            Self::PercentOfBase(0) => 0,
            Self::PercentOfBase(n) => i128::from(base) * (i128::from(n) - 100) / 100,
            Self::Add(n) => i128::from(n),
        };
        u32::try_from((i128::from(base) + delta).max(0)).ok()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AttributeModifiers {
    pub(crate) melee: Option<AttributeModifier>,
    pub(crate) distance: Option<AttributeModifier>,
    pub(crate) magic_points: Option<AttributeModifier>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatSkill {
    Fist,
    Club,
    Sword,
    Axe,
    Distance,
    Shield,
    Fishing,
}
impl AttributeModifiers {
    pub(crate) fn is_negative(self) -> bool {
        [self.melee, self.distance, self.magic_points]
            .into_iter()
            .flatten()
            .any(|m| match m {
                AttributeModifier::PercentOfBase(n) => n > 0 && n < 100,
                AttributeModifier::Add(n) => n < 0,
            })
    }
    pub(crate) fn valid(self) -> bool {
        let values = [self.melee, self.distance, self.magic_points];
        values.iter().any(Option::is_some)
            && values.into_iter().flatten().all(AttributeModifier::valid)
    }
    pub(crate) fn magic_level(self, base: u32) -> Option<u32> {
        self.magic_points.map_or(Some(base), |m| m.effective(base))
    }
    pub(crate) fn combat_skill(self, skill: CombatSkill, base: u32) -> Option<u32> {
        let modifier = match skill {
            CombatSkill::Club | CombatSkill::Sword | CombatSkill::Axe => self.melee,
            CombatSkill::Distance => self.distance,
            CombatSkill::Fist | CombatSkill::Shield | CombatSkill::Fishing => None,
        };
        modifier.map_or(Some(base), |m| m.effective(base))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduction_uses_delta_truncation_not_percent_floor() {
        let mods = AttributeModifiers {
            magic_points: Some(AttributeModifier::PercentOfBase(90)),
            ..Default::default()
        };
        assert_eq!(mods.magic_level(53), Some(48));
        assert_eq!(AttributeModifier::PercentOfBase(0).effective(53), Some(53));
        assert_eq!(AttributeModifier::Add(-15).effective(10), Some(0));
    }
    #[test]
    fn melee_is_only_sword_axe_and_club() {
        let mods = AttributeModifiers {
            melee: Some(AttributeModifier::PercentOfBase(40)),
            distance: Some(AttributeModifier::Add(-15)),
            ..Default::default()
        };
        for skill in [CombatSkill::Club, CombatSkill::Sword, CombatSkill::Axe] {
            assert_eq!(mods.combat_skill(skill, 100), Some(40));
        }
        assert_eq!(mods.combat_skill(CombatSkill::Distance, 100), Some(85));
        for skill in [CombatSkill::Fist, CombatSkill::Shield, CombatSkill::Fishing] {
            assert_eq!(mods.combat_skill(skill, 100), Some(100));
        }
    }
    #[test]
    fn invalid_percent_and_arithmetic_overflow_are_not_saturation() {
        assert!(
            !AttributeModifiers {
                melee: Some(AttributeModifier::PercentOfBase(-1)),
                ..Default::default()
            }
            .valid()
        );
        assert!(
            !AttributeModifiers {
                melee: Some(AttributeModifier::PercentOfBase(101)),
                ..Default::default()
            }
            .valid()
        );
        assert_eq!(AttributeModifier::Add(i32::MAX).effective(u32::MAX), None);
    }
}
