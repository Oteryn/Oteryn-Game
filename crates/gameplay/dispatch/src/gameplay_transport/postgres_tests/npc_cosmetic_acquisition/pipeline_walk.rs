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
                    .quest_callback_matches(key, None, c.selector.action, Some(19387))
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

/// Only this fixture's17 immutable source pads are cached; every edge is proven
/// against the active Base action/appearance and the real admitted callback.
fn source_portals(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<&'static BTreeMap<Cell, Cell>> {
    static CACHE: std::sync::OnceLock<([u8; 32], BTreeMap<Cell, Cell>)> =
        std::sync::OnceLock::new();
    let digest = owner
        .bundle_map()
        .ok_or("actual Spike map")?
        .overlay
        .base()
        .digest();
    if CACHE.get().is_none() {
        let source: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/world-authoring/samples/spike-portal-bindings.json"
        )))?;
        let rows = source["bindings"]
            .as_array()
            .ok_or("17 source portal coordinates")?;
        if rows.len() != 17 {
            return Err("exact17 source portal bindings required".into());
        }
        let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load().map_err(debug)?;
        let mut edges = BTreeMap::new();
        for row in rows {
            let position = &row["position"];
            let origin = (
                i32::try_from(position[0].as_i64().ok_or("source x")?)?,
                i32::try_from(position[1].as_i64().ok_or("source y")?)?,
                i32::try_from(position[2].as_i64().ok_or("source z")?)?,
            );
            if row["item"] != 19387
                || !row["action"]
                    .as_u64()
                    .is_some_and(|a| (4227..=4243).contains(&a))
            {
                return Err("source portal identity mismatch".into());
            }
            let destination = spike_callback_landing(owner, origin, &callbacks)?;
            if origin == destination || edges.insert(origin, destination).is_some() {
                return Err("missing/duplicate actual source portal edge".into());
            }
        }
        // A raced initializer still has to pass the exact immutable map digest below.
        let _ = CACHE.set((digest, edges));
    }
    let (cached, edges) = CACHE.get().ok_or("source portal cache initialization")?;
    if *cached != digest {
        return Err("source portal fixture changed its immutable Base".into());
    }
    Ok(edges)
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
    let portals = source_portals(owner)?;
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
            let next = portals.get(&next).copied().unwrap_or(next);
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
