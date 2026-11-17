//! FLOORCHG: a ladder USE whose preferred landing is occupied falls back to the next donor-order
//! landing through the production router (`serve_admitted`) on the compiled Elvenbane source
//! region. Donor: Crystal 201189f8 data/scripts/actions/items/ladder_up.lua:5-18 (ladder USE
//! teleports to `Position:moveUpstairs()`) and data/libs/functions/position.lua:22-50 (one floor
//! up; south, then N, E, W, SW, SE, NW, NE; the first walkable tile). The single-user ladder,
//! hole, stairs and replay path is already proved by `elvenbane_route.rs`.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::MovementLocalPosition;
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_object::UseDisposition;

/// Position:moveUpstairs order (position.lua:22-50), as `(dx, dy)` from the ladder.
const DONOR_ORDER: [(i32, i32); 8] = [
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (-1, 1),
    (1, 1),
    (-1, -1),
    (1, -1),
];
const LADDER: TilePos = TilePos {
    x: 32598,
    y: 31651,
    floor: -7,
};

#[test]
fn floorchg_ladder_occupied_landing_falls_back_in_donor_order() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::floor_change()
}

pub(super) async fn scenario(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
) -> TestResult {
    let [first, second, ..] = admitted else {
        return Err("two admitted players".into());
    };
    let landings = walkable_landings(owner)?;
    let [preferred, fallback, ..] = landings.as_slice() else {
        return Err("two walkable ladder landings".into());
    };
    assert_eq!(
        *preferred,
        MovementLocalPosition {
            x: 32598,
            y: 31652,
            floor: 6
        },
        "the source ladder's south landing is walkable"
    );
    let ladder = super::questpc8_authored_programmes::object_target(owner, LADDER, 1)?;
    assert_eq!(
        super::annihilator_route::use_target(owner, first, &ladder).await?,
        UseDisposition::Committed
    );
    assert_eq!(position(owner, first).await?, *preferred);
    let ladder = super::questpc8_authored_programmes::object_target(owner, LADDER, 1)?;
    assert_eq!(
        super::annihilator_route::use_target(owner, second, &ladder).await?,
        UseDisposition::Committed,
        "an occupied south landing is not a refusal"
    );
    assert_eq!(
        position(owner, second).await?,
        *fallback,
        "the second climber lands on the next walkable donor-order tile"
    );
    assert_eq!(position(owner, first).await?, *preferred);
    Ok(())
}

/// The walkable tiles one floor above the ladder, in donor order.
fn walkable_landings(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<MovementLocalPosition>> {
    let cells = owner.movement_cells();
    let floor = i32::from(-LADDER.floor) - 1;
    let mut landings = Vec::new();
    for (dx, dy) in DONOR_ORDER {
        let x = i32::from(LADDER.x) + dx;
        let y = i32::from(LADDER.y) + dy;
        if cells
            .index()
            .lookup(cells.scope(), LogicalCell { x, y, z: floor })
            .is_ok_and(|class| class == CollisionClass::Walkable)
        {
            landings.push(MovementLocalPosition {
                x,
                y,
                floor: i16::try_from(floor)?,
            });
        }
    }
    Ok(landings)
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}
