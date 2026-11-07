//! Crystal201189f soul_war_mechanics.lua579-666; source callback count, not a timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BurningForm {
    Ashes,
    Spark,
    Flame,
    Blaze,
}
impl BurningForm {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Ashes => "oteryn:creature.ashes_of_burning_hatred",
            Self::Spark => "oteryn:creature.spark_of_burning_hatred",
            Self::Flame => "oteryn:creature.flame_of_burning_hatred",
            Self::Blaze => "oteryn:creature.blaze_of_burning_hatred",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    SetForm {
        form: BurningForm,
        restore_health: bool,
    },
    AshesSpeech,
    EmpowerBoss(u32),
    RemoveNamed(BurningForm),
}
#[derive(Debug, Clone, Default)]
pub struct HatredRoom {
    remaining: u64,
    last_think: Option<u64>,
}
impl HatredRoom {
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }
    pub fn lever_pulled(&mut self) {
        self.remaining = 180;
    }
    pub fn sorrow_used(&mut self) -> Option<u64> {
        self.remaining = self.remaining.checked_add(10)?;
        Some(self.remaining)
    }
    pub fn burning_think(
        &mut self,
        sequence: u64,
        form: BurningForm,
        boss_present: bool,
    ) -> Vec<Effect> {
        if self.last_think.is_some_and(|last| sequence <= last) {
            return Vec::new();
        }
        self.last_think = Some(sequence);
        let old = self.remaining;
        if old == 0 {
            self.remaining = 180;
            return Vec::new();
        }
        self.remaining -= 1;
        let next = match old {
            180 => BurningForm::Ashes,
            135 => BurningForm::Spark,
            90 => BurningForm::Flame,
            45 => BurningForm::Blaze,
            _ => return Vec::new(),
        };
        if form == next {
            return Vec::new();
        }
        let mut effects = vec![Effect::SetForm {
            form: next,
            restore_health: true,
        }];
        if next == BurningForm::Ashes {
            effects.push(Effect::AshesSpeech);
            if boss_present {
                effects.push(Effect::EmpowerBoss(10));
            }
        }
        effects
    }
    pub fn boss_disappeared(&self) -> Vec<Effect> {
        [
            BurningForm::Ashes,
            BurningForm::Spark,
            BurningForm::Flame,
            BurningForm::Blaze,
        ]
        .into_iter()
        .map(Effect::RemoveNamed)
        .collect()
    }
    pub const fn last_think(&self) -> Option<u64> {
        self.last_think
    }
}
pub fn outgoing(
    primary: u64,
    secondary: u64,
    percent: u32,
    attacker_hatred: bool,
    target_player_named_megalomania: bool,
) -> Option<(u64, u64)> {
    let primary = if attacker_hatred || target_player_named_megalomania {
        u64::try_from(u128::from(primary).checked_mul(100 + u128::from(percent))? / 100).ok()?
    } else {
        primary
    };
    Some((primary, secondary))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_hatred_predecrement_thresholds_and_zero_reset_are_source_exact() {
        let mut room = HatredRoom::default();
        room.lever_pulled();
        assert_eq!(room.remaining(), 180);
        for sequence in 1..=45 {
            assert!(
                room.burning_think(sequence, BurningForm::Ashes, true)
                    .is_empty()
            );
        }
        assert_eq!(room.remaining(), 135);
        assert_eq!(
            room.burning_think(46, BurningForm::Ashes, true),
            vec![Effect::SetForm {
                form: BurningForm::Spark,
                restore_health: true
            }]
        );
        assert_eq!(room.remaining(), 134);
        assert!(
            room.burning_think(46, BurningForm::Ashes, true).is_empty(),
            "same native think is not a second source callback"
        );
        for sequence in 47..=90 {
            room.burning_think(sequence, BurningForm::Spark, true);
        }
        assert_eq!(
            room.burning_think(91, BurningForm::Spark, true),
            vec![Effect::SetForm {
                form: BurningForm::Flame,
                restore_health: true
            }]
        );
        for sequence in 92..=135 {
            room.burning_think(sequence, BurningForm::Flame, true);
        }
        assert_eq!(
            room.burning_think(136, BurningForm::Flame, true),
            vec![Effect::SetForm {
                form: BurningForm::Blaze,
                restore_health: true
            }]
        );
        for sequence in 137..=180 {
            room.burning_think(sequence, BurningForm::Blaze, true);
        }
        assert_eq!(room.remaining(), 0);
        assert!(room.burning_think(181, BurningForm::Blaze, true).is_empty());
        assert_eq!(room.remaining(), 180);
        assert_eq!(
            room.burning_think(182, BurningForm::Blaze, true),
            vec![
                Effect::SetForm {
                    form: BurningForm::Ashes,
                    restore_health: true
                },
                Effect::AshesSpeech,
                Effect::EmpowerBoss(10)
            ]
        );
    }
    #[test]
    fn srx3_hatred_sorrow_can_reverse_forms_without_inventing_forward_only_timers()
    -> Result<(), &'static str> {
        let mut room = HatredRoom::default();
        room.lever_pulled();
        for sequence in 1..=46 {
            room.burning_think(sequence, BurningForm::Ashes, true);
        }
        assert_eq!(room.remaining(), 134);
        for _ in 0..5 {
            room.sorrow_used().ok_or("source counter overflow")?;
        }
        assert_eq!(room.remaining(), 184);
        for sequence in 47..=50 {
            assert!(
                room.burning_think(sequence, BurningForm::Spark, true)
                    .is_empty()
            );
        }
        assert_eq!(
            room.burning_think(51, BurningForm::Spark, false),
            vec![
                Effect::SetForm {
                    form: BurningForm::Ashes,
                    restore_health: true
                },
                Effect::AshesSpeech
            ]
        );
        Ok(())
    }
    #[test]
    fn srx3_hatred_non_death_disappearance_removes_all_four_named_forms() {
        assert_eq!(
            HatredRoom::default().boss_disappeared(),
            vec![
                Effect::RemoveNamed(BurningForm::Ashes),
                Effect::RemoveNamed(BurningForm::Spark),
                Effect::RemoveNamed(BurningForm::Flame),
                Effect::RemoveNamed(BurningForm::Blaze)
            ]
        );
    }
    #[test]
    fn srx3_hatred_outgoing_primary_only_preserves_source_megalomania_typo() {
        assert_eq!(outgoing(101, 50, 10, true, false), Some((111, 50)));
        assert_eq!(
            outgoing(101, 50, 10, false, false),
            Some((101, 50)),
            "normal player hit by Megalomania does not satisfy the source typo"
        );
        assert_eq!(
            outgoing(101, 50, 10, false, true),
            Some((111, 50)),
            "literal alternative checks target-player name"
        );
    }
}
