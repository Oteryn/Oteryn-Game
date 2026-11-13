// The source tests only the destination cell, retaining the one-candidate budget.
#[test]
fn m40_diagonals_commit_both_axes_with_one_collision_lookup() -> Result<(), Box<dyn Error>> {
    for (direction, dx, dy, facing) in [
        (
            CardinalStep::NorthEast,
            1,
            -1,
            crate::foundation::MovementFacing::East,
        ),
        (
            CardinalStep::SouthEast,
            1,
            1,
            crate::foundation::MovementFacing::East,
        ),
        (
            CardinalStep::SouthWest,
            -1,
            1,
            crate::foundation::MovementFacing::West,
        ),
        (
            CardinalStep::NorthWest,
            -1,
            -1,
            crate::foundation::MovementFacing::West,
        ),
    ] {
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let cells = index(&scope, &[(4 + dx, 5 + dy, 7, walkable())])?;
        let before = fixture.current_position()?;
        let actor = fixture.actor();
        let mut owner = fixture.borrow_position();
        let mut decision =
            MovementDecision::begin(&owner, actor, before, &selection(before, &scope), direction)?;
        decision.attempt_candidate(&cells)?;
        assert_eq!(decision.real_content_lookups(), 1);
        assert_eq!(
            owner.read(actor)?,
            before,
            "planning cannot mutate either axis"
        );
        let after = decision.commit(&mut owner)?;
        assert_eq!((after.position().x, after.position().y), (4 + dx, 5 + dy));
        assert_eq!(direction.facing(), facing);
        assert_eq!(direction.duration_multiplier(), 3);
    }
    Ok(())
}

#[test]
fn m40_blocked_diagonal_keeps_the_entire_snapshot() -> Result<(), Box<dyn Error>> {
    let mut fixture = self::fixture(4, 5, false)?;
    let scope = self::scope(&fixture)?;
    let cells = index(
        &scope,
        &[(
            5,
            4,
            7,
            EngineeringCollisionClaim::Qualified(CollisionClass::Blocked),
        )],
    )?;
    let before = fixture.current_position()?;
    let actor = fixture.actor();
    let mut owner = fixture.borrow_position();
    let mut decision = MovementDecision::begin(
        &owner,
        actor,
        before,
        &selection(before, &scope),
        CardinalStep::NorthEast,
    )?;
    assert_eq!(
        decision.attempt_candidate(&cells),
        Err(MovementError::Blocked)
    );
    assert_eq!(decision.commit(&mut owner), Err(MovementError::Blocked));
    assert_eq!(owner.read(actor)?, before);
    Ok(())
}

#[test]
fn m40_neighbor_commit_still_refuses_jumps_floor_changes_and_no_move() -> Result<(), Box<dyn Error>>
{
    let mut fixture = self::fixture(4, 5, false)?;
    let before = fixture.current_position()?;
    let actor = fixture.actor();
    for next in [
        MovementLocalPosition {
            x: 6,
            y: 6,
            floor: 7,
        },
        MovementLocalPosition {
            x: 5,
            y: 6,
            floor: 8,
        },
        before.position(),
    ] {
        let mut owner = fixture.borrow_position();
        assert_eq!(
            owner.commit_step(before, next),
            Err(CarrierError::MovementNonCardinal)
        );
        assert_eq!(owner.read(actor)?, before);
    }
    Ok(())
}

#[test]
fn m40_shared_duration_keeps_house_and_channel_steps_on_the_source_beat() {
    use crate::movement::speed::{EngineeringGroundSpeed, player_directional_step_duration};
    let at = LogicalCell { x: 4, y: 5, z: 7 };
    assert_eq!(
        player_directional_step_duration(&EngineeringGroundSpeed, at, 220, CardinalStep::East),
        Some(std::time::Duration::from_millis(300))
    );
    assert_eq!(
        player_directional_step_duration(&EngineeringGroundSpeed, at, 220, CardinalStep::NorthEast),
        Some(std::time::Duration::from_millis(900))
    );
}
