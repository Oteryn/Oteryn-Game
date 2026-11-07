//! Crystal soul_war_mechanics.lua71–88 and game.cpp8278–8302.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Killer {
    Null,
    NonPlayer,
    Player { taints: u8, in_realm: bool },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    /// HP at the canonical DAMAGE phase, after an optional source script heal.
    pub damage_before: i64,
    /// Source already clamped this to the victim's ORIGINAL HP before callback.
    pub damage: i64,
    pub health_after: i64,
    pub healed: bool,
    pub cancelled: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    InvalidHealth,
    InvalidTaints,
    InvalidRoll,
}
pub fn resolve(
    health: i64,
    maximum: i64,
    requested: i64,
    killer: Killer,
    mut draw: impl FnMut() -> u32,
) -> Result<Resolution, Refusal> {
    if health <= 0 || maximum < health || requested <= 0 {
        return Err(Refusal::InvalidHealth);
    }
    let damage = requested.min(health);
    let mut result = Resolution {
        damage_before: health,
        damage,
        health_after: health - damage,
        healed: false,
        cancelled: false,
    };
    if requested < health {
        return Ok(result);
    }
    match killer {
        // Literal nil killer dereference fails the PrepareDeath callback. Keep
        // its cancellation without a runtime Lua error or an endless retry.
        Killer::Null => {
            result.damage = 0;
            result.health_after = health;
            result.cancelled = true;
        }
        Killer::NonPlayer => {}
        Killer::Player { taints, in_realm } => {
            if taints > 5 {
                return Err(Refusal::InvalidTaints);
            }
            if taints >= 4 && in_realm {
                let roll = draw();
                if !(1..=10).contains(&roll) {
                    return Err(Refusal::InvalidRoll);
                }
                if roll == 1 {
                    result.damage_before = maximum;
                    result.health_after = maximum - damage;
                    result.healed = true;
                }
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn srx3_fourth_taint_keeps_old_hp_clamp_after_atomic_heal() {
        let hit = resolve(
            1,
            1000,
            5000,
            Killer::Player {
                taints: 4,
                in_realm: true,
            },
            || 1,
        )
        .expect("source player");
        assert_eq!(
            (hit.damage_before, hit.damage, hit.health_after),
            (1000, 1, 999)
        );
        assert!(hit.healed);
        let full = resolve(
            1000,
            1000,
            5000,
            Killer::Player {
                taints: 4,
                in_realm: true,
            },
            || 1,
        )
        .expect("source full HP");
        assert_eq!(
            full.health_after, 0,
            "source callback does not cancel the retained lethal amount"
        );
    }
    #[test]
    fn srx3_fourth_taint_distinguishes_null_killer_from_non_player_without_rng() {
        let cancelled = resolve(1, 1000, 10, Killer::Null, || {
            panic!("NULL produces source Lua error before RNG")
        })
        .expect("cancelled source");
        assert_eq!(
            (
                cancelled.damage_before,
                cancelled.damage,
                cancelled.health_after
            ),
            (1, 0, 1)
        );
        assert!(cancelled.cancelled);
        let normal = resolve(1, 1000, 10, Killer::NonPlayer, || {
            panic!("nonPlayer exits before RNG")
        })
        .expect("source actual creature");
        assert_eq!((normal.damage, normal.health_after), (1, 0));
    }
    #[test]
    fn srx3_fourth_taint_requires_player_taint_zone_and_lethal_damage() {
        for (hp, damage, killer) in [
            (
                10,
                1,
                Killer::Player {
                    taints: 5,
                    in_realm: true,
                },
            ),
            (
                1,
                10,
                Killer::Player {
                    taints: 3,
                    in_realm: true,
                },
            ),
            (
                1,
                10,
                Killer::Player {
                    taints: 4,
                    in_realm: false,
                },
            ),
        ] {
            assert!(
                !resolve(hp, 1000, damage, killer, || panic!(
                    "ineligible source cannot draw"
                ))
                .expect("neutral")
                .healed
            );
        }
        for roll in 2..=10 {
            assert_eq!(
                resolve(
                    1,
                    1000,
                    10,
                    Killer::Player {
                        taints: 4,
                        in_realm: true
                    },
                    || roll
                )
                .expect("valid source roll")
                .health_after,
                0
            );
        }
    }
}
