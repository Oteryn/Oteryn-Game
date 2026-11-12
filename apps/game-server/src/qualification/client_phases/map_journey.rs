//! MAPBOT-1: bounded map checks through a real admitted client session.
include!("map_model.rs");

pub(super) async fn run(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    model: &MapModel<'_>,
) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    let mut totals: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    for sample in model.samples(start, 1) {
        let result = check(client, model, &sample, start).await;
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
        walk(client, world, &back).await
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

async fn check(
    client: &mut oteryn_dev_client::DevClientSession,
    model: &MapModel<'_>,
    sample: &Sample,
    bounds: TilePos,
) -> TestResult<Option<&'static str>> {
    let from = tile(client.world_spatial().actor_position)?;
    let target = sample.position;
    let blocked = matches!(sample.kind, "blocking_item" | "closed_door")
        || (sample.kind == "field"
            && model
                .tiles
                .get(&target)
                .is_some_and(|facts| !facts.walkable));
    let transition = matches!(
        sample.kind,
        "stairs_up" | "stairs_down" | "hole_down" | "teleport"
    ) || (sample.kind == "ladder" && model.world.floors().contains(target));
    let mut approaches = Vec::new();
    for at in DIRECTIONS
        .into_iter()
        .filter_map(|direction| adjacent(target, direction))
    {
        for direction in DIRECTIONS {
            if adjacent(at, direction) != Some(target) {
                continue;
            }
            let landing = model.destination(at, direction);
            if transition
                && !landing.is_some_and(|landed| match sample.kind {
                    "stairs_up" => landed.floor > at.floor,
                    "stairs_down" | "hole_down" => landed.floor < at.floor,
                    "ladder" => landed.floor != at.floor,
                    _ => Some(landed) == model.tiles.get(&target).and_then(|facts| facts.teleport),
                })
            {
                continue;
            }
            if model.tiles.get(&at).is_none_or(|facts| !facts.walkable) {
                continue;
            }
            if sample.kind == "pz_border"
                && model
                    .tiles
                    .get(&target)
                    .zip(model.tiles.get(&at))
                    .is_none_or(|(target, origin)| target.protection_zone == origin.protection_zone)
            {
                continue;
            }
            // An automatic transition can make an apparently adjacent tile unstandable.
            if let Some(path) = model.path(from, at, Some(bounds)) {
                approaches.push((path, at, direction, landing));
            }
        }
    }
    approaches.sort_by_key(|(path, at, _, _)| (path.len(), *at));
    let Some((path, at, direction, landing)) = approaches.first() else {
        return Ok(Some("no_reachable_approach_within_160_tile_2_floor_budget"));
    };
    walk(client, model.world, path).await?;
    if tile(client.world_spatial().actor_position)? != *at {
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
    if matches!(sample.kind, "rope_spot" | "shovel_hole") {
        use oteryn_protocol_oteryn::world_object::*;
        let rope = sample.kind == "rope_spot";
        let index = provisioning::item_index()?;
        let reference = provisioning::item_reference(&index, if rope { 3003 } else { 3457 })?;
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
        let facts = &model.tiles[&target];
        let ordinal = if rope {
            facts.rope_ordinal
        } else {
            facts.shovel_ordinal
        }
        .ok_or("map tool target ordinal missing")?;
        let key = crate::map::view::placement_key(target, ordinal)
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
        let payload = encode_item_use_intent(&intent)
            .map_err(|error| format!("map tool encode: {error:?}"))?;
        let before = tile(client.world_spatial().actor_position)?;
        let response = client
            .session_mut()
            .gated_command(CAPABILITY_ITEM_USE_V1, COMMAND_TYPE_USE_INTENT, &payload)
            .await?;
        if response.status != CommandStatus::Accepted
            || decode_use_result(&response.payload)
                .map_err(|error| format!("map tool result: {error:?}"))?
                != UseDisposition::Committed
        {
            return Err(format!("map tool use did not commit: {response:?}").into());
        }
        if rope {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            loop {
                let landed = tile(client.world_spatial().actor_position)?;
                if landed.floor == target.floor + 1
                    && landed.x.abs_diff(target.x).max(landed.y.abs_diff(target.y)) == 1
                {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err("rope did not publish an adjacent upstairs landing".into());
                }
                client.service_liveness(Duration::from_millis(50)).await?;
                client.take_events();
            }
        } else {
            if tile(client.world_spatial().actor_position)? != before {
                return Err("shovel use moved the character before stepping".into());
            }
            let result = client.step_retrying(*direction).await?;
            let landed = tile(client.world_spatial().actor_position)?;
            if result.disposition != StepDisposition::Moved
                || result.world_spatial_delta.is_none()
                || landed.floor >= target.floor
            {
                return Err("opened shovel hole did not produce a lower-floor step".into());
            }
        }
        client.take_events();
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

#[cfg(test)]
#[path = "map_model_tests.rs"]
mod tests;
