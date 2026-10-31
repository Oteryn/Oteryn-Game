//! Way of the Monk: percentages compose once after the draw, before blocking.

use std::sync::OnceLock;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonkShrineBonuses {
    schema: String,
    source: String,
    offensive_per_shrine_percent: u8,
    serene_offensive_per_shrine_percent: u8,
    defensive_per_shrine_percent: u8,
    defensive_cap_percent: u8,
}

impl MonkShrineBonuses {
    pub fn checked_in() -> Option<&'static Self> {
        static TUNING: OnceLock<Option<MonkShrineBonuses>> = OnceLock::new();
        TUNING
            .get_or_init(|| {
                Self::parse(include_str!(
                    "../../../../content/combat/monk_shrine_bonuses_v1.json"
                ))
            })
            .as_ref()
    }

    pub fn parse(text: &str) -> Option<Self> {
        let tuning: Self = serde_json::from_str(text).ok()?;
        (tuning.schema == "OTERYN_MONK_SHRINE_BONUSES/v1"
            && !tuning.source.is_empty()
            && tuning.offensive_per_shrine_percent <= tuning.serene_offensive_per_shrine_percent
            && tuning.defensive_cap_percent <= 100)
            .then_some(tuning)
    }

    /// Percentages add across the five offensive shrines, then multiply the drawn hit once.
    /// An unrepresentable positive hit refuses the swing instead of wrapping or saturating.
    pub fn attack(&self, damage: i64, offensive: u8, monk: bool, serene: bool) -> Option<i64> {
        let per_shrine = if serene {
            self.serene_offensive_per_shrine_percent
        } else {
            self.offensive_per_shrine_percent
        };
        let bonus = if monk {
            u16::from(offensive.min(5)) * u16::from(per_shrine)
        } else {
            0
        };
        scale(damage, 100 + bonus)
    }

    /// Melee auto-attacks only: the caller proves the origin before entering this function.
    /// Damage taken retains the fractional remainder until the final integer division.
    pub fn taken(&self, damage: i64, completed: u8, monk: bool) -> Option<i64> {
        let reduction = if monk {
            (u16::from(completed.min(10)) * u16::from(self.defensive_per_shrine_percent))
                .min(u16::from(self.defensive_cap_percent))
        } else {
            0
        };
        scale(damage, 100 - reduction)
    }
}

fn scale(damage: i64, percent: u16) -> Option<i64> {
    let scaled = u128::try_from(damage)
        .ok()?
        .checked_mul(u128::from(percent))?
        / 100;
    scaled.try_into().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn monk_shrine_bonus_full_damage_breakdown() -> Result<(), String> {
        let tuning = super::MonkShrineBonuses::checked_in().ok_or("tuning")?;
        // Five offensive shrines; Serene changes each shrine's additive percentage.
        let draw = 200;
        for (serene, boosted) in [(false, 300), (true, 400)] {
            let attack = tuning.attack(draw, 5, true, serene).ok_or("attack")?;
            assert_eq!(attack, boosted);
            let after_defence = crate::resolution::player_defence::player_defence_mitigated(
                attack.try_into().map_err(|_| "attack range")?,
                (30, 30),
                0,
            );
            let constants =
                crate::attack::AttackConstants::checked_in().ok_or("attack constants")?;
            let after_armor =
                crate::resolution::player_armor_mitigated(constants, after_defence, 40, 0);
            let taken = tuning
                .taken(i64::from(after_armor), 10, true)
                .ok_or("taken")?;
            assert_eq!(
                (
                    draw,
                    attack,
                    i64::from(after_defence),
                    i64::from(after_armor),
                    taken
                ),
                if serene {
                    (200, 400, 370, 350, 280)
                } else {
                    (200, 300, 270, 250, 200)
                }
            );
        }
        assert_eq!(tuning.attack(draw, 0, true, true), Some(draw));
        assert_eq!(tuning.attack(draw, 5, false, true), Some(draw));
        assert_eq!(tuning.taken(draw, 10, false), Some(draw));
        assert_eq!(tuning.taken(draw, 1, true), Some(196));
        assert_eq!(tuning.taken(draw, u8::MAX, true), Some(160));
        assert_eq!(tuning.attack(i64::MAX, 5, true, true), None);
        assert_eq!(
            tuning.attack(i64::MAX / 2, 5, true, true),
            Some(i64::MAX - 1)
        );
        assert_eq!(
            tuning.taken(i64::MAX, 10, true),
            Some(7_378_697_629_483_820_645)
        );
        assert_eq!(tuning.attack(-1, 5, true, true), None);
        assert_eq!(tuning.taken(-1, 10, true), None);
        Ok(())
    }
}
