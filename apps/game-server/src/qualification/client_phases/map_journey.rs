//! MAPBOT-1: bounded map checks through a real admitted client session.
include!("map_model.rs");

pub(super) async fn run(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    model: &MapModel<'_>,
    (samples, returnable): (Vec<Sample>, BTreeSet<TilePos>),
) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    if start != TOWN {
        return Err(format!("map journey expected to begin at {TOWN:?}, got {start:?}").into());
    }
    let mut totals: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for sample in samples {
        let result = check(client, model, &sample, start, &returnable).await;
        let (index, reason) = match result {
            Ok(None) => (0, "position_and_protocol_result_match_bundle".to_owned()),
            Ok(Some(reason)) => (2, reason.to_owned()),
            Err(error) => (1, error.to_string().replace(['\n', '\r'], " ")),
        };
        totals.entry(sample.kind).or_default()[index] += 1;
        let at = sample.position;
        evidence(&format!(
            "MAP_CHECK {} {},{},{} {} {}{}",
            sample.kind,
            at.x,
            at.y,
            -i16::from(at.floor),
            ["PASS", "FAIL", "SKIPPED"][index],
            reason,
            sample
                .content
                .as_ref()
                .map_or(String::new(), |key| format!(" content={key}"))
        ));
    }
    // Restore the caller's start so subsequent phases retain their own preconditions.
    let restore: TestResult = async {
        let here = tile(client.world_spatial().actor_position)?;
        let back = model
            .path(here, start, Some(start))
            .ok_or("map journey has no return route")?;
        walk_map_route(client, world, &back, start, None).await
    }
    .await;
    let (index, reason) = match restore {
        Ok(()) => (0, "returned_to_phase_start".to_owned()),
        Err(error) => (1, error.to_string().replace(['\n', '\r'], " ")),
    };
    totals.entry("return").or_default()[index] += 1;
    evidence(&format!(
        "MAP_CHECK return {},{},{} {} {reason}",
        start.x,
        start.y,
        -i16::from(start.floor),
        ["PASS", "FAIL"][index]
    ));
    let mut failures = 0;
    for (kind, [pass, fail, skipped]) in totals {
        failures += fail;
        evidence(&format!(
            "MAP_SUMMARY kind={kind} pass={pass} fail={fail} skipped={skipped}"
        ));
    }
    if failures != 0 {
        return Err(format!("map journey: {failures} failed checks").into());
    }
    Ok(())
}

async fn walk_map_route(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    bounds: TilePos,
    actor_approaches: Option<&BTreeSet<TilePos>>,
) -> TestResult {
    walk_with_landing_budget(
        client,
        path,
        |at, direction| destination(world, at, direction),
        None,
        Some(bounds),
        actor_approaches,
    )
    .await
}

async fn check(
    client: &mut oteryn_dev_client::DevClientSession,
    model: &MapModel<'_>,
    sample: &Sample,
    bounds: TilePos,
    returnable: &BTreeSet<TilePos>,
) -> TestResult<Option<&'static str>> {
    let from = tile(client.world_spatial().actor_position)?;
    let target = sample.position;
    let rope = sample.kind == "rope_spot";
    let tool_probe = rope || sample.kind == "shovel_hole";
    if tool_probe
        && !client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err("map tool probe has no negotiated world map view".into());
    }
    let blocked = matches!(sample.kind, "blocking_item" | "closed_door")
        || (sample.kind == "field"
            && model
                .tiles
                .get(&target)
                .is_some_and(|facts| !facts.walkable));
    let mut approaches = Vec::new();
    for (at, direction) in approach_steps(target) {
        let landing = model.destination(at, direction);
        if !model.approach_allowed(sample, at, direction, landing, Some((returnable, bounds))) {
            continue;
        }
        // An automatic transition can make an apparently adjacent tile unstandable.
        if let Some(path) = model.path(from, at, Some(bounds)) {
            approaches.push((path, at, direction, landing));
        }
    }
    approaches.sort_by_key(|(path, at, _, _)| (path.len(), *at));
    let Some((path, at, direction, landing)) = approaches.first() else {
        return Ok(Some("no_reachable_approach_within_160_tile_2_floor_budget"));
    };
    if rope && !model.rope_lands_on_return_route(target, returnable) {
        return Err(
            format!("rope spot {target:?} lacks a walking return from every landing").into(),
        );
    }
    // An actor observation can use any of the already reachable adjacent cells.
    // The actor may have wandered onto the first approach before we get there.
    let actor_approaches = matches!(sample.kind, "npc" | "monster_spawn")
        .then(|| approaches.iter().map(|(_, at, _, _)| *at).collect());
    walk_map_route(client, model.world, path, bounds, actor_approaches.as_ref()).await?;
    let reached = tile(client.world_spatial().actor_position)?;
    if !actor_approaches
        .as_ref()
        .map_or(reached == *at, |approaches| approaches.contains(&reached))
    {
        return Err("map approach did not end at predicted position".into());
    }
    if sample.kind == "pz_border" {
        client_spell_journey::protection_zone_probe(client, model.tiles[at].protection_zone)
            .await?;
    }
    if blocked {
        let before = client.world_spatial().actor_position;
        let result = client.step_retrying(*direction).await?;
        if result.disposition != StepDisposition::Blocked
            || client.world_spatial().actor_position != before
            || result.world_spatial_delta.is_some()
        {
            return Err(
                format!("expected unchanged position and Blocked refusal, got {result:?}").into(),
            );
        }
        client.take_events();
        return Ok(None);
    }
    if rope {
        rope_home(client, model, target, bounds, None).await?;
        return Ok(None);
    }
    if sample.kind == "shovel_hole" {
        let expected = model
            .shovel_landing(*at, *direction, target)
            .ok_or("shovel probe has no compiled lower-floor landing")?;
        let exit = model
            .shovel_exit(expected, target, bounds, returnable)
            .ok_or("shovel landing has no ordinary route to town")?;
        use_map_tool(
            client,
            model,
            target,
            model.tiles[&target].shovel_ordinal,
            3457,
        )
        .await?;
        if tile(client.world_spatial().actor_position)? != *at {
            return Err("shovel use moved the character before stepping".into());
        }
        let result = client.step_retrying(*direction).await?;
        let landed = tile(client.world_spatial().actor_position)?;
        if result.disposition != StepDisposition::Moved
            || result.world_spatial_delta.is_none()
            || landed != expected
        {
            return Err(format!("opened shovel hole expected lower landing {expected:?}, arrived at {landed:?}: {result:?}").into());
        }
        client.take_events();
        match exit {
            ShovelExit::Walk(path) => {
                walk_map_route(client, model.world, &path, bounds, None).await?
            }
            ShovelExit::Rope(path) => {
                walk_map_route(client, model.world, &path, bounds, None).await?;
                rope_home(client, model, landed, bounds, Some(target)).await?;
            }
        }
        return Ok(None);
    }
    if matches!(sample.kind, "npc" | "monster_spawn") {
        let kind = if sample.kind == "npc" {
            oteryn_dev_client::EntityKind::Npc
        } else {
            oteryn_dev_client::EntityKind::Creature
        };
        if !client.world_entities().is_some_and(|entities| {
            entities.iter().any(|entity| {
                entity.kind == kind
                    && tile(entity.position).is_ok_and(|at| {
                        at.floor == target.floor
                            && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 3
                    })
            })
        }) {
            return Err("expected spawn has no visible actor within three tiles".into());
        }
        // VIS-2 actors have appearance_ref, not a content definition id. Do not claim
        // that a different creature near the spawn proves the requested identity.
        return Ok(Some("VIS-2_actor_has_no_content_id_identity_unverifiable"));
    }
    if sample.kind == "ladder" && landing.is_none_or(|landed| landed.floor == at.floor) {
        return Ok(Some(
            "catalogue_ladder_has_no_server_floor_change_or_USE_handler",
        ));
    }
    let Some(expected) = landing else {
        return Err("walkable sample has no predicted enterable landing".into());
    };
    let result = client.step_retrying(*direction).await?;
    if result.disposition != StepDisposition::Moved
        || result.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != *expected
    {
        return Err(format!("expected landing {expected:?}, got {result:?}").into());
    }
    client.take_events();
    if sample.kind == "pz_border" {
        client_spell_journey::protection_zone_probe(client, model.tiles[expected].protection_zone)
            .await?;
    }
    Ok(None)
}

async fn use_map_tool(
    client: &mut oteryn_dev_client::DevClientSession,
    model: &MapModel<'_>,
    target: TilePos,
    ordinal: Option<u8>,
    tool: u32,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::*;
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, tool)?;
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .ok_or("provisioned map tool missing from inventory")?
        .handle;
    let key =
        crate::map::view::placement_key(target, ordinal.ok_or("map tool target ordinal missing")?)
            .ok_or("map tool target placement missing")?;
    let mut placement = model.world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    };
    let payload =
        encode_item_use_intent(&intent).map_err(|error| format!("map tool encode: {error:?}"))?;
    let response = client.session_mut().world_context_use(&payload).await?;
    if response.status != CommandStatus::Accepted
        || decode_use_result(&response.payload)
            .map_err(|error| format!("map tool result: {error:?}"))?
            != UseDisposition::Committed
    {
        return Err(format!("map tool use did not commit: {response:?}").into());
    }
    Ok(())
}

async fn rope_home(
    client: &mut oteryn_dev_client::DevClientSession,
    model: &MapModel<'_>,
    target: TilePos,
    bounds: TilePos,
    opened_hole: Option<TilePos>,
) -> TestResult {
    use_map_tool(
        client,
        model,
        target,
        model.tiles[&target].rope_ordinal,
        3003,
    )
    .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let landed = loop {
        let landed = tile(client.world_spatial().actor_position)?;
        if landed.floor == target.floor + 1
            && landed.x.abs_diff(target.x).max(landed.y.abs_diff(target.y)) == 1
        {
            break landed;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("rope did not publish an adjacent upstairs landing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let back = model
        .path_avoiding(landed, bounds, Some(bounds), opened_hole)
        .ok_or_else(|| format!("rope landing {landed:?} has no walking return to {bounds:?}"))?;
    walk_map_route(client, model.world, &back, bounds, None).await?;
    client.take_events();
    Ok(())
}

#[cfg(test)]
#[path = "map_model_tests.rs"]
mod tests;
