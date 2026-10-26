//! Exaltation Forge creature arithmetic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeCreature {
    Influenced(u8),
    Fiendish,
}

pub const FIENDISH_RESPAWN_SECONDS: u16 = 270;
pub const FIENDISH_SIMULTANEOUS_MAX: u8 = 4;
pub const SHARED_XP_RANGE_XY: u16 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeRecipientFacts {
    pub premium: bool,
    pub logout_blocked: bool,
    pub shared_experience_enabled: bool,
    pub delta_x: u16,
    pub delta_y: u16,
    pub floor_delta: u8,
}

/// Global dust recipient gate. Premium is deliberately only a dust gate;
/// corpse slivers are ordinary loot and never pass through this function.
#[must_use]
pub const fn eligible_dust_recipient(facts: ForgeRecipientFacts) -> bool {
    facts.premium
        && facts.logout_blocked
        && facts.shared_experience_enabled
        && facts.delta_x <= SHARED_XP_RANGE_XY
        && facts.delta_y <= SHARED_XP_RANGE_XY
        && facts.floor_delta <= 1
}

/// Difficulty-qualified sliver quantity. The caller supplies the creature's
/// admitted difficulty and deterministic roll; no global min/max economy roll
/// is used.
#[must_use]
pub const fn slivers(difficulty: u8, roll: u8) -> u16 {
    match difficulty {
        0..=1 => 0,
        2..=3 => 1 + (roll % 2) as u16,
        4..=5 => 2 + (roll % 3) as u16,
        _ => 3 + (roll % 4) as u16,
    }
}

impl ForgeCreature {
    pub fn new_influenced(stacks: u8) -> Option<Self> {
        (1..=5)
            .contains(&stacks)
            .then_some(Self::Influenced(stacks))
    }

    #[must_use]
    pub const fn reward_stacks(self) -> u8 {
        match self {
            Self::Influenced(stacks) => stacks,
            Self::Fiendish => 15,
        }
    }

    /// Global health scaling: base 35%, plus 15% for every stack.
    #[must_use]
    pub const fn health_percent(self) -> u16 {
        135 + self.reward_stacks() as u16 * 15
    }

    /// One through three dust per reward stack; randomness belongs to the simulation owner.
    #[must_use]
    pub const fn dust(self, roll: u8) -> u32 {
        self.reward_stacks() as u32 * (1 + (roll % 3) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::{ForgeCreature, ForgeRecipientFacts};

    #[test]
    fn global_stack_health_and_dust_bounds() {
        assert!(ForgeCreature::new_influenced(0).is_none());
        assert!(ForgeCreature::new_influenced(6).is_none());
        for stacks in 1..=5 {
            if let Some(creature) = ForgeCreature::new_influenced(stacks) {
                assert_eq!(creature.health_percent(), 135 + u16::from(stacks) * 15);
                assert_eq!(creature.dust(0), u32::from(stacks));
                assert_eq!(creature.dust(2), u32::from(stacks) * 3);
            } else {
                assert!(ForgeCreature::new_influenced(stacks).is_some());
            }
        }
        assert_eq!(ForgeCreature::Fiendish.health_percent(), 360);
        assert_eq!(ForgeCreature::Fiendish.dust(0), 15);
        assert_eq!(ForgeCreature::Fiendish.dust(2), 45);
    }

    #[test]
    fn recipient_requires_every_global_gate() {
        let eligible = ForgeRecipientFacts {
            premium: true,
            logout_blocked: true,
            shared_experience_enabled: true,
            delta_x: 30,
            delta_y: 30,
            floor_delta: 1,
        };
        assert!(super::eligible_dust_recipient(eligible));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            premium: false,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            logout_blocked: false,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            shared_experience_enabled: false,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            delta_x: 31,
            ..eligible
        }));
        assert!(!super::eligible_dust_recipient(ForgeRecipientFacts {
            floor_delta: 2,
            ..eligible
        }));
    }

    #[test]
    fn slivers_are_difficulty_qualified() {
        assert_eq!(super::slivers(1, 255), 0);
        assert_eq!(super::slivers(2, 0), 1);
        assert_eq!(super::slivers(4, 2), 4);
        assert_eq!(super::slivers(6, 3), 6);
    }
}
