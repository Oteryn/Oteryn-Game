//! Select actual source points and actual reachable talk cells, never a relocated/fake producer.
use super::*;
use crate::foundation::MovementLocalPosition as Pos;
pub(super) struct Scene {
    pub table: oteryn_world_bundle::spawn::Table,
    pub homes: Vec<(Pos, Pos)>,
}
fn walkable(owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>, p: Pos) -> bool {
    owner
        .movement_cells
        .index()
        .lookup(
            owner.movement_cells.scope(),
            crate::content::LogicalCell {
                x: p.x,
                y: p.y,
                z: i32::from(p.floor),
            },
        )
        .ok()
        == Some(crate::content::CollisionClass::Walkable)
}
pub(super) fn select(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Scene> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_SUPPORT_BUNDLE").ok_or("actual source support Bundle")?,
    )?;
    let mut b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let mut points = Vec::new();
    let mut demon_floors = std::collections::BTreeMap::<i8, usize>::new();
    for source in &b.spawns.sources {
        for point in &source.points {
            if b.spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|c| c.key == "oteryn:creature.demon_skeleton")
            {
                *demon_floors.entry(source.floor).or_default() += 1;
            }
        }
    }
    let mut matching = 0;
    let mut walkable_homes = 0;
    let mut walkable_stands = 0;
    let start = Pos {
        x: 32243,
        y: 32597,
        floor: 9,
    };
    for (source_index, source) in b.spawns.sources.iter().enumerate() {
        if source.floor != -9 {
            continue;
        }
        for (point_index, point) in source.points.iter().enumerate() {
            if b.spawns
                .creatures
                .get(point.creature as usize)
                .is_none_or(|c| c.key != "oteryn:creature.demon_skeleton")
            {
                continue;
            }
            let home = Pos {
                x: i32::from(point.x),
                y: i32::from(point.y),
                floor: 9,
            };
            matching += 1;
            walkable_homes += usize::from(walkable(owner, home));
            let distance = home.x.abs_diff(start.x) + home.y.abs_diff(start.y);
            if distance < 25 {
                continue;
            }
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let stand = Pos {
                    x: home.x + dx,
                    y: home.y + dy,
                    floor: 9,
                };
                walkable_stands += usize::from(walkable(owner, stand));
                if walkable(owner, stand) && walk::reachable(owner, start, stand) {
                    points.push((distance, source_index, point_index, home, stand));
                    break;
                }
            }
        }
    }
    let portal_attrs = b
        .sectors
        .iter()
        .flat_map(|sector| sector.tiles.iter())
        .flat_map(|tile| tile.items.iter())
        .filter(|item| {
            item.depth == 0
                && item
                    .attrs
                    .action
                    .is_some_and(|a| (4227..=4243).contains(&a))
        })
        .count();
    eprintln!(
        "SRV7_SPIKE_REAL_BOUNDARY start={start:?} start_walkable={} source_demon_floors={demon_floors:?} floor9_source_points={matching} walkable_homes={walkable_homes} walkable_neighbor_stands={walkable_stands} reachable_original_points={} actual_spike_action_bindings={portal_attrs}",
        walkable(owner, start),
        points.len()
    );
    for (x, y, z) in [
        (32234, 32604, 9),
        (32243, 32619, 9),
        (32242, 32596, 9),
        (32241, 32597, 9),
        (32243, 32598, 9),
        (32244, 32597, 9),
    ] {
        let cell = crate::content::LogicalCell { x, y, z };
        let state = owner
            .movement_cells
            .index()
            .lookup(owner.movement_cells.scope(), cell);
        eprintln!("SRV7_SPIKE_ACTUAL_COLLISION ({x},{y},{z})={state:?}");
    }
    if let Some(native) = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
    {
        for id in [
            15466, 15847, 15849, 15852, 15855, 15856, 15859, 15860, 18368, 19387,
        ] {
            eprintln!(
                "SRV7_SPIKE_ACTUAL_NATIVE_POLICY id={id} attributes={:?}",
                native
                    .item_policy_for_source_id(id)
                    .map(|p| p.record().attributes.clone())
            );
        }
    }
    points.sort_by_key(|r| (r.0, r.1, r.2));
    points.truncate(7);
    assert_eq!(
        points.len(),
        7,
        "seven original source Demon Skeletons reachable from actual Gnomilly"
    );
    let selected: std::collections::BTreeSet<_> = points.iter().map(|r| (r.1, r.2)).collect();
    assert!(
        b.spawns
            .weighted
            .iter()
            .all(|w| !selected.contains(&(w.source as usize, usize::from(w.point)))),
        "selected original Demon Skeleton points must be ordinary, never weighted alternatives"
    );
    for (si, s) in b.spawns.sources.iter_mut().enumerate() {
        let mut i = 0;
        s.points.retain(|_| {
            let retain = selected.contains(&(si, i));
            i += 1;
            retain
        });
    }
    b.spawns.sources.retain(|s| !s.points.is_empty());
    b.spawns.weighted.clear();
    for row in &points {
        assert!((32008..=32365).contains(&row.3.x) && (32522..=32759).contains(&row.3.y));
    }
    eprintln!(
        "SRV7_SPIKE_ACTUAL_SEVEN_SOURCE_POINTS {:?}",
        points.iter().map(|r| (r.3, r.4)).collect::<Vec<_>>()
    );
    Ok(Scene {
        table: b.spawns,
        homes: points.into_iter().map(|r| (r.3, r.4)).collect(),
    })
}
pub(super) async fn npc(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    key: &str,
) -> TestResult<(
    oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    Pos,
)> {
    let runtime = owner.runtime.lock().await;
    let visible = runtime.visible_entities();
    let n = visible
        .npcs
        .iter()
        .find(|n| runtime.npc_key(n.actor) == Some(key))
        .ok_or_else(|| format!("actual placed source NPC {key}"))?;
    let p = runtime
        .read_actor_position(n.actor)
        .map_err(debug)?
        .position();
    let entity = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: n.actor.placement_identity(),
        generation: n.generation,
    };
    drop(runtime);
    let start = Pos {
        x: 32243,
        y: 32597,
        floor: 9,
    };
    for radius in 1..=3 {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let stand = Pos {
                    x: p.x + dx,
                    y: p.y + dy,
                    floor: p.floor,
                };
                if walkable(owner, stand) && walk::reachable(owner, start, stand) {
                    return Ok((entity, stand));
                }
            }
        }
    }
    Err(format!("no real public reachable talk cell for{key}").into())
}
