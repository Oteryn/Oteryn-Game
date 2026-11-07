//! Test-only navigation follows the real bundle floor/teleport resolver and commits real steps.
use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
type Cell = (i32, i32, i32);
/// Read only the same real Base action selectors used by production committed-step callbacks.
fn spike_callback_landing(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    at: Cell,
    callbacks: &[oteryn_gameplay_quests::quest::world_callbacks::Callback],
) -> TestResult<Cell> {
    use crate::map::view::MapFacts;
    let map = owner.bundle_map().ok_or("actual Spike Bundle map")?;
    let pos = crate::map::overlay::TilePos {
        x: at.0.try_into()?,
        y: at.1.try_into()?,
        floor: i8::try_from(at.2)?.checked_neg().ok_or("nativefloor")?,
    };
    let Some(base) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return Ok(at);
    };
    let mut selected = Vec::new();
    for ordinal in 0..base.depths().iter().filter(|d| **d == 0).count() {
        let key = crate::map::view::placement_key(pos, u8::try_from(ordinal)?)
            .ok_or("source portal placement")?;
        if map.facts.base_hidden(key) {
            continue;
        }
        for c in callbacks {
            if c.key.starts_with("oteryn:quest-callback/spike-portal-")
                && c.trigger == oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn
                && c.selector.position.is_none()
                && c.selector.unique.is_none()
                && c.selector.appearance.is_none()
                && c.selector.action.is_some()
                && c.storage.is_empty()
                && c.offerings.is_empty()
                && map
                    .facts
                    .quest_callback_matches(key, None, c.selector.action, None)
                && let Some(to) = c.teleport
            {
                selected.push((i32::from(to.x), i32::from(to.y), -i32::from(to.floor)));
            }
        }
    }
    match selected.as_slice() {
        [] => Ok(at),
        [to] => Ok(*to),
        _ => Err("ambiguous actual Spike source callback".into()),
    }
}

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
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load().map_err(debug)?;
    let mut before = BTreeMap::<Cell, (Cell, CardinalStep)>::new();
    let mut queue = VecDeque::from([from]);
    before.insert(from, (from, CardinalStep::North));
    while let Some(p) = queue.pop_front() {
        if p == to {
            break;
        }
        if before.len() > 1_000_000 {
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
            let next = spike_callback_landing(owner, next, &callbacks)?;
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
