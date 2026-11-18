//! MAPBOT-1: bounded map checks through a real admitted client session.
include!("map_model.rs");

pub(super) async fn run(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    model: &MapModel<'_>,
) -> TestResult {
    let start = tile(client.world_spatial().actor_position)?;
    let mut totals: BTreeMap<&str, [usize; 3]> = [
        "blocking_item",
        "closed_door",
        "field",
        "hole_down",
        "ladder",
        "monster_spawn",
        "npc",
        "pz_border",
        "rope_spot",
        "shovel_hole",
        "stairs_down",
        "stairs_up",
        "teleport",
    ]
    .into_iter()
    .map(|kind| (kind, [0; 3]))
    .collect();
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
    for (kind, [pass, fail, skipped]) in &totals {
        evidence(&format!(
            "MAP_SUMMARY kind={kind} pass={pass} fail={fail} skipped={skipped}"
        ));
    }
    map_coverage(&totals)
}

fn map_coverage(totals: &BTreeMap<&str, [usize; 3]>) -> TestResult {
    let failures: usize = totals.values().map(|counts| counts[1]).sum();
    let skipped: usize = totals.values().map(|counts| counts[2]).sum();
    if totals
        .values()
        .copied()
        .any(|counts| incomplete_checks(counts) != 0)
        || !totals.keys().any(|kind| *kind != "return")
    {
        return Err(format!(
            "map journey coverage incomplete: failed={failures} unproven={skipped}"
        )
        .into());
    }
    Ok(())
}

fn incomplete_checks([pass, fail, skipped]: [usize; 3]) -> usize {
    fail + skipped + usize::from(pass + fail + skipped == 0)
}

#[test]
fn qualification_map_skipped_or_absent_kind_fails_coverage() {
    assert_eq!(incomplete_checks([0, 0, 1]), 1);
    assert_eq!(incomplete_checks([0, 0, 0]), 1);
    assert_eq!(incomplete_checks([1, 0, 0]), 0);
    assert_eq!(incomplete_checks([1, 1, 1]), 2);
}

#[test]
fn sampled_map_checks_cannot_all_be_unproven() {
    let totals = BTreeMap::from([("return", [1, 0, 0]), ("stairs", [0, 0, 3])]);
    assert!(map_coverage(&totals).is_err());
    assert!(map_coverage(&BTreeMap::from([("return", [1, 0, 0])])).is_err());
    assert!(
        map_coverage(&BTreeMap::from([
            ("stairs", [3, 0, 0]),
            ("return", [1, 0, 0])
        ]))
        .is_ok()
    );
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
        compare_protection_zone(client, model.tiles[at].protection_zone).await?;
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
    if sample.kind == "ladder" && !transition {
        let ordinal = model.tiles[&target]
            .ladder_ordinal
            .ok_or("catalogue ladder has no supported immovable USE target")?;
        let placement = map_placement(model.world, target, ordinal)?;
        let expected = upstairs_landing(client, model, target)?;
        if travel::use_map_object(client, &placement).await?
            != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        {
            return Err("immovable ladder USE did not commit".into());
        }
        return observe_ascent(client, expected).await.map(|()| None);
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
        let placement = map_placement(model.world, target, ordinal)?;
        let expected_revision = travel::observed_revision(client, &placement, target, ordinal)?;
        let intent = UseIntent {
            target: UseTarget::Item(source),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement,
                expected_revision,
            })),
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent)
            .map_err(|error| format!("map tool encode: {error:?}"))?;
        let before = tile(client.world_spatial().actor_position)?;
        let response = client.session_mut().world_context_use(&payload).await?;
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
        let name = sample
            .content
            .as_ref()
            .and_then(|key| model.actor_names.get(key))
            .ok_or("sampled actor has no authored display name")?;
        let key = sample
            .content
            .as_deref()
            .ok_or("actor sample has no authored key")?;
        let expected = if sample.kind == "npc" {
            npc_display_name(key)?
        } else {
            creature_display_name(key)?.to_owned()
        };
        if !client.world_entities().is_some_and(|entities| {
            entities.iter().any(|entity| {
                entity.kind == kind
                    && has_display_name(entity, name)
                    && entity
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        .is_some_and(|name| name.eq_ignore_ascii_case(&expected))
                    && tile(entity.position).is_ok_and(|at| {
                        at.floor == target.floor
                            && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 3
                    })
            })
        }) {
            return Err(format!(
                "expected {key} ({expected}) has no visible actor within three tiles"
            )
            .into());
        }
        return Ok(None);
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
        compare_protection_zone(client, model.tiles[expected].protection_zone).await?;
    }
    Ok(None)
}

fn map_placement(world: &BundleWorld, at: TilePos, ordinal: u8) -> TestResult<Vec<u8>> {
    let key = crate::map::view::placement_key(at, ordinal).ok_or("map placement missing")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(placement)
}

fn upstairs_landing(
    client: &oteryn_dev_client::DevClientSession,
    model: &MapModel<'_>,
    spot: TilePos,
) -> TestResult<TilePos> {
    travel::first_landing(spot, |at| {
        model.tiles.get(&at).is_some_and(|facts| facts.walkable)
            && !client.world_entities().is_some_and(|entities| {
                entities.iter().any(|entity| {
                    matches!(
                        entity.kind,
                        oteryn_dev_client::EntityKind::Player
                            | oteryn_dev_client::EntityKind::Creature
                            | oteryn_dev_client::EntityKind::Npc
                    ) && tile(entity.position).ok() == Some(at)
                })
            })
    })
    .ok_or_else(|| "map ascent has no predicted free landing".into())
}

async fn observe_ascent(
    client: &mut oteryn_dev_client::DevClientSession,
    expected: TilePos,
) -> TestResult {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if tile(client.world_spatial().actor_position)? == expected {
                return Ok(());
            }
            client.take_events();
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await?
}

async fn compare_protection_zone(
    client: &mut oteryn_dev_client::DevClientSession,
    inside: bool,
) -> TestResult {
    use client_spell_journey::ProtectionProbe;
    let (words, target) = client_spell_journey::fixture_protection_spell()?;
    let comparison = if inside {
        ProtectionProbe::Inside
    } else {
        ProtectionProbe::Outside
    };
    client_spell_journey::protection_zone_probe(client, comparison, words, target).await
}

#[cfg(test)]
#[path = "map_model_tests.rs"]
mod tests;
