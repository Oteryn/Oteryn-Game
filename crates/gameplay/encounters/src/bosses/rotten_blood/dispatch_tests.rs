#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::gameplay_transport::rotten_blood_dispatch::{
        AgonyDamage, RoomPosition, RoomTarget, RottenBloodEvent, RottenBloodRefusal, dispatch,
        owner,
    };

    fn target(actor: u64, x: i32) -> RoomTarget {
        RoomTarget {
            actor,
            position: RoomPosition { x, y: 40, floor: 7 },
        }
    }

    #[test]
    fn native_dispatch_applies_gloom_chain_geometry_and_escalation() {
        let mut owner = owner();
        let targets = [target(10, 43), target(11, 49), target(12, 55)];
        let hits = dispatch(
            &mut owner,
            RottenBloodEvent::GloomChain {
                origin: RoomPosition {
                    x: 40,
                    y: 40,
                    floor: 7,
                },
                targets: &targets,
            },
        )
        .expect("source-qualified chain must dispatch");

        assert_eq!(
            hits,
            vec![
                AgonyDamage {
                    actor: 10,
                    amount: 2_250
                },
                AgonyDamage {
                    actor: 11,
                    amount: 3_375
                },
                AgonyDamage {
                    actor: 12,
                    amount: 4_500
                },
            ]
        );
        let outside = [target(10, 44)];
        assert_eq!(
            dispatch(
                &mut owner,
                RottenBloodEvent::GloomChain {
                    origin: RoomPosition {
                        x: 40,
                        y: 40,
                        floor: 7
                    },
                    targets: &outside,
                }
            ),
            Err(RottenBloodRefusal::OutsideChainRange)
        );
    }

    #[test]
    fn native_dispatch_applies_dropped_charge_to_every_room_player() {
        let mut owner = owner();
        let players = [target(21, 1), target(22, 99), target(23, 150)];
        let hits = dispatch(
            &mut owner,
            RottenBloodEvent::RottenChargeDropped {
                room_players: &players,
            },
        )
        .expect("room consequence must dispatch");

        assert_eq!(hits.len(), players.len());
        assert!(hits.iter().all(|hit| hit.amount == 1_250));
        assert_eq!(
            hits.iter().map(|hit| hit.actor).collect::<Vec<_>>(),
            [21, 22, 23]
        );
    }

    /// PAR-W11: Crystal `rotten_blood_boss_mechanics.lua:246` deals `ceil(maxHealth * 0.4)`
    /// agony to each room player on the rotten-charge explosion; Oteryn keeps the flat 1250
    /// from news 7852 regardless of the player's max health (DIFF, owner decision).
    #[test]
    fn w11_crystal_rotten_charge_damage_diverges_from_flat_news_value() {
        let mut owner = owner();
        let players = [target(31, 1)];
        let hits = dispatch(
            &mut owner,
            RottenBloodEvent::RottenChargeDropped {
                room_players: &players,
            },
        )
        .expect("room consequence must dispatch");
        let crystal = |max_health: u32| (max_health * 2).div_ceil(5);
        assert_eq!(crystal(3_125), 1_250);
        for max_health in [1_000, 3_122, 3_126, 20_000] {
            assert_ne!(hits[0].amount, crystal(max_health));
        }
        assert_eq!(hits[0].amount, 1_250);
    }
}
