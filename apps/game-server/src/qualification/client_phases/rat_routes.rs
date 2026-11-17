// Careful rat routes share the production STEP/USE movement oracle.
fn careful_rat_route(
    world: &BundleWorld,
    from: TilePos,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> Option<Vec<WalkAction>> {
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().map(move |point| {
                (
                    TilePos {
                        x: point.x,
                        y: point.y,
                        floor: source.floor,
                    },
                    spawns
                        .creatures
                        .get(point.creature as usize)
                        .is_some_and(|entry| entry.key == "oteryn:creature.rat"),
                )
            })
        })
        .collect();
    let forbidden: std::collections::BTreeSet<_> = points
        .iter()
        .flat_map(|(point, _)| {
            (-2..=2).flat_map(move |dy| {
                (-2..=2).filter_map(move |dx| {
                    point
                        .x
                        .checked_add_signed(dx)
                        .zip(point.y.checked_add_signed(dy))
                        .map(|(x, y)| TilePos {
                            x,
                            y,
                            floor: point.floor,
                        })
                })
            })
        })
        .collect();
    points
        .iter()
        .enumerate()
        .filter(|(index, (at, rat))| {
            *rat && at.x.abs_diff(from.x) <= 160
                && at.y.abs_diff(from.y) <= 160
                && at.floor.abs_diff(from.floor) <= 2
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
                && !points.iter().enumerate().any(|(other, (point, _))| {
                    other != *index
                        && point.floor == at.floor
                        && point.x.abs_diff(at.x).max(point.y.abs_diff(at.y)) <= 4
                })
        })
        .filter_map(|(_, (target, _))| {
            let path = plan_route_filtered(
                world,
                from,
                ("isolated rat", target),
                true,
                |at| {
                    rat_public(world, at)
                        && (!forbidden.contains(&at)
                            || (at.floor == target.floor
                                && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 2))
                },
                |at| {
                    at.floor == target.floor
                        && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
                        && world
                            .base()
                            .tile(at.x, at.y, at.floor)
                            .is_some_and(|tile| !tile.protection_zone())
                },
            )?;
            let end = path
                .iter()
                .try_fold(from, |at, action| destination(world, at, *action))?;
            let refuge = rat_refuge_route(world, end)?;
            (refuge.len() <= 32).then_some(path)
        })
        .min_by_key(Vec::len)
}

fn rat_public(world: &BundleWorld, at: TilePos) -> bool {
    use crate::map::view::MapFacts;
    !world.facts().pinned().house_tile(at)
}
fn rat_refuge_route(world: &BundleWorld, from: TilePos) -> Option<Vec<WalkAction>> {
    plan_route_filtered(
        world,
        from,
        "public rat hunter PZ refuge",
        true,
        |at| rat_public(world, at),
        |at| {
            rat_public(world, at)
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| tile.protection_zone())
        },
    )
}
pub(super) fn rat_escape_available(
    client: &oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult<bool> {
    Ok(
        rat_refuge_route(world, tile(client.world_spatial().actor_position)?)
            .is_some_and(|path| path.len() <= 32),
    )
}
pub(super) async fn rat_retreat(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    client.session_mut().attack_target(None).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = rat_refuge_route(world, from).ok_or_else(|| {
        route_gap(
            world,
            client.world_spatial().actor_position,
            "PZ refuge",
            "rat hunter has no ordinary retreat",
        )
    })?;
    evidence(&format!(
        "rat_care phase=retreat start={from:?} steps={} vitals={:?}",
        path.len(),
        client.actor_vitals()
    ));
    walk(client, world, &path).await
}
fn rat_clear(client: &oteryn_dev_client::DevClientSession, at: TilePos, radius: u16) -> bool {
    !client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .any(|entry| {
            entry.kind == oteryn_dev_client::EntityKind::Creature
                && tile(entry.position).is_ok_and(|creature| {
                    creature.floor == at.floor
                        && creature.x.abs_diff(at.x).max(creature.y.abs_diff(at.y)) <= radius
                })
        })
}
pub(super) async fn rat_recovery_edge(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = plan_route_filtered(
        world,
        from,
        "clear regeneration tile beside public PZ",
        true,
        |at| {
            rat_public(world, at)
                && (world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| tile.protection_zone())
                    || rat_clear(client, at, 8))
        },
        |at| {
            rat_public(world, at)
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
                && rat_clear(client, at, 8)
                && [
                    StepDirection::North,
                    StepDirection::East,
                    StepDirection::South,
                    StepDirection::West,
                ]
                .into_iter()
                .filter_map(|direction| destination(world, at, direction))
                .any(|next| {
                    rat_public(world, next)
                        && world
                            .base()
                            .tile(next.x, next.y, next.floor)
                            .is_some_and(|tile| tile.protection_zone())
                })
        },
    )
    .ok_or("rat recovery has no clear non-PZ tile with a real STEP into public refuge")?;
    walk(client, world, &path).await
}
pub(super) fn rat_recovery_clear(client: &oteryn_dev_client::DevClientSession) -> TestResult<bool> {
    Ok(rat_clear(
        client,
        tile(client.world_spatial().actor_position)?,
        8,
    ))
}
pub(super) async fn rat_return_to_arena(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    arena: ActorPosition,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let goal = tile(arena)?;
    let path = route_toward(world, from, ("rat arena", goal), |at| at == goal)
        .ok_or("rat arena has no ordinary return after recovery")?;
    rat_approach_walk(client, world, &path).await
}
async fn rat_approach_walk(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[WalkAction],
) -> TestResult {
    let rat_name = native_creature_name("oteryn:creature.rat")?;
    for action in path {
        let position = client.world_spatial().actor_position;
        let rat_is_adjacent = client
            .world_entities()
            .into_iter()
            .flat_map(|entities| entities.iter())
            .any(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry
                        .display_name
                        .is_some_and(|name| name.as_str() == Ok(rat_name))
                    && entry.position.floor == position.floor
                    && entry
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(entry.position.y.abs_diff(position.y))
                        <= 1
            });
        if rat_is_adjacent {
            return Ok(());
        }
        if let Err(error) = walk(client, world, std::slice::from_ref(action)).await {
            // A completed refused STEP can be followed by a retreat. Transport
            // errors retain their existing session-poison/drop handling.
            if error.to_string().starts_with("town route step ") {
                rat_retreat(client, world).await?;
            }
            return Err(error);
        }
        let vitals = client.actor_vitals().ok_or("rat approach vitals absent")?;
        let here = tile(client.world_spatial().actor_position)?;
        let nearby = client
            .world_entities()
            .into_iter()
            .flat_map(|entities| entities.iter())
            .filter(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && tile(entry.position).is_ok_and(|at| {
                        at.floor == here.floor
                            && at.x.abs_diff(here.x).max(at.y.abs_diff(here.y)) <= 3
                    })
            })
            .count();
        if u64::from(vitals.health) * 4 <= u64::from(vitals.max_health) * 3 || nearby > 1 {
            rat_retreat(client, world).await?;
            return Err("rat approach aborted at health reserve; hunter returned to PZ".into());
        }
    }
    Ok(())
}
pub(super) async fn travel_to_careful_rat(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    client_hunt::recover_rat_health(
        client,
        world,
        tokio::time::Instant::now() + Duration::from_secs(180),
    )
    .await?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = careful_rat_route(world, from, spawns).ok_or_else(|| {
        route_gap(
            world,
            client.world_spatial().actor_position,
            "isolated rat with PZ escape",
            "no safe rat approach within the town budget",
        )
    })?;
    rat_approach_walk(client, world, &path).await
}
