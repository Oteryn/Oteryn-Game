use super::*;

fn at(x: u16, y: u16, floor: i8) -> TilePos {
    TilePos { x, y, floor }
}

#[test]
fn travel_plans_a_closed_door_as_use_then_step() -> TestResult {
    let world = crate::map::boot::tests::door_world()?;
    let door = crate::map::boot::tests::DOOR_AT;
    let west = at(door.x - 1, door.y, door.floor);
    // The single-floor walker refuses the closed door; the travel planner opens it first.
    assert_eq!(destination(&world, west, StepDirection::East), None);
    let planner = Planner::new(&world);
    assert_eq!(planner.step(west, StepDirection::East), Some(door));
    let route = planner
        .plan(west, door, &|here| here == door, false)
        .ok_or("door route missing")?;
    assert_eq!(route, [(Leg::Step(StepDirection::East), door)]);
    assert_eq!(
        planner.describe(west, &route),
        [format!("door@{},{},7(action=0)", door.x, door.y)]
    );
    Ok(())
}

#[test]
fn travel_climb_lands_one_floor_up_in_server_offset_order() {
    let spot = at(10, 10, -8);
    assert_eq!(first_landing(spot, |_| true), Some(at(10, 11, -7)));
    assert_eq!(
        first_landing(spot, |here| here.y != 11),
        Some(at(10, 9, -7))
    );
    assert_eq!(
        first_landing(spot, |here| here == at(9, 9, -7)),
        Some(at(9, 9, -7))
    );
    assert_eq!(first_landing(at(10, 10, 0), |_| true), None);
}

#[test]
fn travel_search_crosses_floors_by_ladder_and_reports_each_action() -> TestResult {
    // Floor 8 and floor 7 are corridors along x 0..=4; the ladder at (4, 0, 8) lands on (4, 1, 7).
    let ladder = at(4, 0, -8);
    let edges = |here: TilePos, out: &mut Vec<(Leg, TilePos)>| {
        for direction in [StepDirection::East, StepDirection::West] {
            if let Some(next) = neighbour(here, direction)
                && next.x <= 4
            {
                out.push((Leg::Step(direction), next));
            }
        }
        if here.floor == -8 && beside(ladder)(here) {
            out.push((
                Leg::Ladder {
                    at: ladder,
                    ordinal: 1,
                },
                at(4, 1, -7),
            ));
        }
    };
    let goal = |here: TilePos| here == at(0, 1, -7);
    let route = search(at(0, 0, -8), &goal, |_| 0, &edges, 1_000).ok_or("ladder route missing")?;
    assert_eq!(route.len(), 8);
    assert_eq!(
        route[3],
        (
            Leg::Ladder {
                at: ladder,
                ordinal: 1
            },
            at(4, 1, -7)
        )
    );
    assert_eq!(
        describe(at(0, 0, -8), &route, |_| None),
        ["walk(3)->3,0,8 ladder-USE@4,0,8->4,1,7 walk(4)->0,1,7"]
    );
    assert!(search(at(0, 0, -8), &goal, |_| 0, &edges, 1).is_none());
    Ok(())
}
