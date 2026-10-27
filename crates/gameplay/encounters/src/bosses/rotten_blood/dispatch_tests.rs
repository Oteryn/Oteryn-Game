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
}
