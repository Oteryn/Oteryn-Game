//! Real floor resolver and actual paced steps inside the captured producer frame.
use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type Cell = (i32, i32, i32);
fn tuple(p: crate::foundation::MovementLocalPosition) -> Cell {
    (p.x, p.y, i32::from(p.floor))
}
fn logical(p: Cell) -> crate::content::LogicalCell {
    crate::content::LogicalCell {
        x: p.0,
        y: p.1,
        z: p.2,
    }
}
fn plan(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    from: Cell,
    to: Cell,
    blocked: &BTreeSet<Cell>,
) -> TestResult<Vec<(crate::movement::CardinalStep, Cell)>> {
    use crate::map::floor::{Ground, resolve_landing, table_of};
    use crate::movement::CardinalStep;
    let cells = owner.movement_cells;
    let table = table_of(cells).ok_or("actual source floor table")?;
    let ground = |p: Cell| {
        if blocked.contains(&p) && p != from {
            Ground::Blocked
        } else {
            match cells.index().lookup(cells.scope(), logical(p)) {
                Ok(crate::content::CollisionClass::Walkable) => Ground::Walkable,
                Ok(_) => Ground::Blocked,
                Err(_) => Ground::Absent,
            }
        }
    };
    let mut before = BTreeMap::<Cell, (Cell, CardinalStep)>::new();
    let mut queue = VecDeque::from([from]);
    before.insert(from, (from, CardinalStep::North));
    while let Some(p) = queue.pop_front() {
        if p == to {
            break;
        }
        if before.len() > 200_000 {
            return Err("bounded actual source route search exceeded".into());
        }
        for d in [
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::South,
            CardinalStep::West,
        ] {
            let Ok(next) = resolve_landing(table, ground, |_| true, p, d) else {
                continue;
            };
            if next == p || before.contains_key(&next) {
                continue;
            }
            before.insert(next, (p, d));
            queue.push_back(next);
        }
    }
    if !before.contains_key(&to) {
        return Err(format!("no actual bundle route {from:?}→{to:?}").into());
    }
    let mut steps = Vec::new();
    let mut p = to;
    while p != from {
        let (prior, d) = before[&p];
        steps.push((d, p));
        p = prior;
    }
    steps.reverse();
    Ok(steps)
}
pub(super) fn reachable(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    from: crate::foundation::MovementLocalPosition,
    to: crate::foundation::MovementLocalPosition,
) -> bool {
    plan(owner, tuple(from), tuple(to), &BTreeSet::new()).is_ok()
}
pub(super) async fn walk(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &crate::gameplay_transport::connection::AdmittedSession,
    to: crate::foundation::MovementLocalPosition,
) -> TestResult {
    use crate::gameplay_transport::world_spatial::{StepDirection, StepDisposition};
    use crate::movement::CardinalStep;
    let actor = admitted.runtime_actor.ok_or("actual source actor")?;
    let start = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let blocked = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        visible
            .npcs
            .iter()
            .map(|n| n.actor)
            .chain(visible.creatures.iter().map(|c| c.actor))
            .filter_map(|a| {
                runtime
                    .read_actor_position(a)
                    .ok()
                    .map(|p| tuple(p.position()))
            })
            .collect::<BTreeSet<_>>()
    };
    for (direction, expected) in plan(owner, tuple(start), tuple(to), &blocked)? {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("route clock")?;
        let d = match direction {
            CardinalStep::North => StepDirection::North,
            CardinalStep::East => StepDirection::East,
            CardinalStep::South => StepDirection::South,
            CardinalStep::West => StepDirection::West,
        };
        let (outcome, _) = owner
            .ordinary_paced_step(actor, admitted.game_session_id, d, None, None)
            .await;
        assert_eq!(
            outcome.disposition,
            StepDisposition::Moved,
            "real source route to {expected:?}"
        );
        let actual = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            tuple(actual),
            expected,
            "same real floor/teleport resolver as actual move owner"
        );
    }
    Ok(())
}
