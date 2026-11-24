// Route over the loaded qualification crop. A route may need to leave the
// normal local town window to get around a wall or use a return staircase.
fn qualification_hunt_plan(
    client: &DevClientSession,
    world: &BundleWorld,
    target: Option<TilePos>,
    return_to: Option<TilePos>,
    deadline: tokio::time::Instant,
) -> TestResult<Vec<StepDirection>> {
    if tokio::time::Instant::now() >= deadline {
        return Err("qualification_hunt_route expired before travel".into());
    }
    let start = tile(client.world_spatial().actor_position)?;
    let rat_approaches = if target.is_none() {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let creatures = &metadata.spawns.creatures;
        metadata
            .spawns
            .sources
            .iter()
            .flat_map(|source| {
                source.points.iter().filter_map(move |point| {
                    creatures
                        .get(point.creature as usize)
                        .filter(|creature| creature.key == "oteryn:creature.rat")
                        .map(|_| TilePos {
                            x: point.x,
                            y: point.y,
                            floor: source.floor,
                        })
                })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let adjacent = |at: TilePos, other: TilePos| {
        at.floor == other.floor && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
    };
    let mut live_rats = Vec::new();
    let mut corpses = Vec::new();
    for entity in client
        .world_entities()
        .into_iter()
        .flat_map(|view| view.iter())
    {
        let at = tile(entity.position)?;
        match entity.kind {
            oteryn_dev_client::EntityKind::Creature
                if entity
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Rat"))
                    && rat_approaches.iter().any(|&rat| adjacent(at, rat)) =>
            {
                live_rats.push(at);
            }
            oteryn_dev_client::EntityKind::Corpse => corpses.push(at),
            _ => {}
        }
    }
    let landing = |at, direction| {
        (tokio::time::Instant::now() < deadline)
            .then(|| destination(world, at, direction))
            .flatten()
    };
    let returnable = |endpoint| {
        return_to.is_none_or(|origin| find_route(endpoint, |at| at == origin, landing).is_ok())
    };
    let rat_tile = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
            && !rat_approaches.contains(&at)
    };
    if let Some(target) = target {
        find_route(start, |at| at == target && returnable(at), landing)
    } else {
        let approached_live = (!live_rats.is_empty()).then(|| find_route(start, |at| {
            rat_tile(at)
                && live_rats.iter().any(|&rat| at != rat && adjacent(at, rat))
                && returnable(at)
        }, landing));
        if let Some(Ok(path)) = approached_live {
            Ok(path)
        } else {
            find_route(start, |at| {
                rat_tile(at)
                    && rat_approaches.iter().any(|&rat| adjacent(at, rat))
                    && !corpses.iter().any(|&corpse| adjacent(at, corpse))
                    && returnable(at)
            }, landing)
        }
    }
    .map_err(|visited| {
        format!("qualification_hunt_route from {start:?} has no approach and return after {visited} tiles").into()
    })
}
