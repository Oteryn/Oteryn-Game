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
    let mut passages = Passages::default();
    assert_eq!(
        planner.step(west, StepDirection::East, &passages),
        Some(door)
    );
    let route = planner.plan(west, door, &|here| here == door, false, &passages)?;
    assert_eq!(route, [(Leg::Step(StepDirection::East), door)]);
    assert_eq!(
        planner.describe(west, &route),
        [format!("door@{},{},7(action=0)", door.x, door.y)]
    );
    // A refused door is banned: with no way round, the plan fails and names its cause.
    passages.refused.insert(door);
    assert_eq!(planner.step(west, StepDirection::East, &passages), None);
    let refused = planner
        .plan(west, door, &|here| here == door, false, &passages)
        .err()
        .ok_or("a banned door still routed")?;
    assert!(refused.contains("exhausted"), "{refused}");
    assert!(
        refused.contains(&format!("closest tile reached {},{},7", west.x, west.y)),
        "{refused}"
    );
    // A tile a pulled lever opened is passable even when the door was refused.
    passages.open.insert(door);
    assert_eq!(
        planner.plan(west, door, &|here| here == door, false, &passages)?,
        route
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
    let edges = |here: TilePos, out: &mut Vec<(Leg, TilePos, u32)>| {
        for direction in [StepDirection::East, StepDirection::West] {
            if let Some(next) = neighbour(here, direction)
                && next.x <= 4
            {
                out.push((Leg::Step(direction), next, STEP_COST));
            }
        }
        if here.floor == -8 && beside(ladder)(here) {
            out.push((
                Leg::Ladder {
                    at: ladder,
                    ordinal: 1,
                },
                at(4, 1, -7),
                CLIMB_COST,
            ));
        }
    };
    let goal = |here: TilePos| here == at(0, 1, -7);
    let route = search(at(0, 0, -8), &goal, |_| 0, &edges, 1_000)?;
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
    let limited = search(at(0, 0, -8), &goal, |_| 0, &edges, 1)
        .err()
        .ok_or("a one-tile limit still routed")?;
    assert!(limited.contains("search limit of 1"), "{limited}");
    Ok(())
}

#[test]
fn travel_search_prefers_an_open_way_round_to_a_closed_door() -> TestResult {
    // From (0,0) to (2,0): straight through the door at (1,0) costs DOOR_COST + STEP_COST,
    // round through (0,1) (1,1) (2,1) costs four steps, which is cheaper.
    let door = at(1, 0, -7);
    let edges = |here: TilePos, out: &mut Vec<(Leg, TilePos, u32)>| {
        for direction in DIRECTIONS {
            if let Some(next) = neighbour(here, direction)
                && next.x <= 2
                && next.y <= 1
            {
                let cost = if next == door { DOOR_COST } else { STEP_COST };
                out.push((Leg::Step(direction), next, cost));
            }
        }
    };
    let route = search(
        at(0, 0, -7),
        &|here| here == at(2, 0, -7),
        |_| 0,
        edges,
        100,
    )?;
    assert_eq!(route.len(), 4);
    assert!(route.iter().all(|&(_, tile)| tile != door));
    Ok(())
}

/// Not run in this change (no cargo): a passage is one costed edge and one evidence line.
#[test]
fn travel_describes_an_npc_passage_with_its_fare() {
    let offer: &'static Offer = Box::leak(Box::new(Offer {
        npc: "oteryn:npc.charles".into(),
        at: at(5, 5, -6),
        keyword: "edron".into(),
        price: 150,
        premium: true,
        dest: at(50, 50, -6),
    }));
    let lines = describe(at(5, 6, -6), &[(Leg::Travel(offer), offer.dest)], |_| None);
    assert_eq!(
        lines,
        [
            "passage[cost=450](oteryn:npc.charles@5,5,6 says 'edron' for 150 gold (premium) -> 50,50,6)"
        ]
    );
}
