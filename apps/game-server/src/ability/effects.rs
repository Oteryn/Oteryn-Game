use super::AbilityError;
use super::intent::TargetId;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Damage { target: TargetId, magnitude: i64 },
    Heal { target: TargetId, magnitude: i64 },
}

impl Effect {
    pub fn damage(target: &str, magnitude: i64) -> Result<Self, AbilityError> {
        Self::new_damage_or_heal(target, magnitude, true)
    }

    pub fn heal(target: &str, magnitude: i64) -> Result<Self, AbilityError> {
        Self::new_damage_or_heal(target, magnitude, false)
    }

    fn new_damage_or_heal(
        target: &str,
        magnitude: i64,
        damage: bool,
    ) -> Result<Self, AbilityError> {
        if magnitude <= 0 {
            return Err(AbilityError::InvalidMagnitude);
        }
        let target = TargetId::new(target)?;
        if damage {
            Ok(Self::Damage { target, magnitude })
        } else {
            Ok(Self::Heal { target, magnitude })
        }
    }

    #[must_use]
    pub fn target(&self) -> &TargetId {
        match self {
            Self::Damage { target, .. } | Self::Heal { target, .. } => target,
        }
    }

    #[must_use]
    pub const fn magnitude(&self) -> i64 {
        match self {
            Self::Damage { magnitude, .. } | Self::Heal { magnitude, .. } => *magnitude,
        }
    }

    pub(crate) fn canonical_cmp(&self, other: &Self) -> Ordering {
        self.target()
            .cmp(other.target())
            .then_with(|| self.kind_rank().cmp(&other.kind_rank()))
            .then_with(|| other.magnitude().cmp(&self.magnitude()))
    }

    const fn kind_rank(&self) -> u8 {
        match self {
            Self::Damage { .. } => 0,
            Self::Heal { .. } => 1,
        }
    }
}
