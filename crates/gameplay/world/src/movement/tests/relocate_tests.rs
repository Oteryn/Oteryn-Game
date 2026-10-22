use super::*;

#[test]
fn relocation_carrier_refuses_players_and_stale_snapshots_without_mutation()
-> Result<(), Box<dyn Error>> {
    let mut runtime = runtime()?;
    let (player, before) = admitted_at(&mut runtime, 30, 4)?;
    assert!(
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                player,
                before,
                MovementLocalPosition {
                    x: 100,
                    y: 5,
                    floor: 7
                }
            )
            .is_err()
    );
    assert_eq!(runtime.read_actor_position(player)?, before);
    let creature = runtime.admit_pinned_test_creature(MovementLocalPosition {
        x: 6,
        y: 5,
        floor: 7,
    })?;
    let before = runtime.read_actor_position(creature)?;
    let next = MovementLocalPosition {
        x: 57,
        y: 5,
        floor: 7,
    };
    let after = runtime
        .borrow_movement_position()
        .commit_creature_relocation(creature, before, next)?;
    assert_eq!(after.position(), next);
    assert_eq!(after.revision(), before.revision() + 1);
    assert!(
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(creature, before, next)
            .is_err()
    );
    assert_eq!(runtime.read_actor_position(creature)?, after);
    Ok(())
}
