//! P15 client replay capture: the real exercise-training replies (STATUS, refused START, running
//! START, STOP) of a legitimately carried exercise weapon and an authored exercise dummy. The
//! consumer is `apply` in `apps/client/src/features/0054-training/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{item_view::ItemHandle, loop_training as wire};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P15_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn take_varint(bytes: &[u8], cursor: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let byte = *bytes.get(*cursor)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return None;
        }
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Some(value);
        }
    }
    None
}

fn push_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Decode a training reply exactly as the wire encoder lays it out (used by P14 and P15).
pub(super) fn training_reply(bytes: &[u8]) -> Option<wire::TrainingResult> {
    let mut values = [None; 4];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let tag = take_varint(bytes, &mut cursor)?;
        if !matches!(tag, 8 | 16 | 24 | 32) {
            return None;
        }
        let slot = &mut values[usize::try_from(tag / 8 - 1).ok()?];
        if slot.is_some() {
            return None;
        }
        *slot = Some(take_varint(bytes, &mut cursor)?);
    }
    let result = wire::TrainingResult {
        disposition: u8::try_from(values[0]?).ok()?,
        revision: values[1]?,
        pool_seconds: u32::try_from(values[2]?).ok()?,
        charges: u32::try_from(values[3]?).ok()?,
    };
    (wire::encode_training_result(result).ok()?.as_slice() == bytes).then_some(result)
}

async fn exercise(
    client: &mut DevClientSession,
    intent: wire::ExerciseIntent,
) -> TestResult<(Vec<u8>, wire::TrainingResult)> {
    let mut payload = vec![8];
    match intent {
        wire::ExerciseIntent::Start { weapon, dummy } => {
            payload.extend([1, 16]);
            push_varint(&mut payload, weapon.get());
            payload.push(24);
            push_varint(&mut payload, dummy.get());
        }
        wire::ExerciseIntent::Status => payload.push(2),
        wire::ExerciseIntent::Stop => payload.push(3),
    }
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TRAINING_V1,
            wire::COMMAND_TYPE_EXERCISE_TRAINING,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("exercise command status={:?}", reply.status).into());
    }
    let decoded = training_reply(&reply.payload).ok_or("malformed training reply")?;
    Ok((reply.payload, decoded))
}

fn carried_weapon(client: &DevClientSession) -> TestResult<ItemHandle> {
    const WEAPONS: &[u32] = &[
        28540, 28541, 28542, 28543, 28544, 28545, 28552, 28553, 28554, 28555, 28556, 28557, 44064,
        44065, 50292, 50293,
    ];
    let index = provisioning::item_index()?;
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    for &id in WEAPONS {
        let reference = provisioning::item_reference(&index, id)?;
        if let Some(item) = inventory
            .entries
            .iter()
            .chain(&inventory.main_backpack)
            .find(|item| item.item_definition_ref == reference)
        {
            return Ok(item.handle);
        }
    }
    Err("no carried exercise weapon in inventory".into())
}

fn dummy_at_route_destination(matches_definition: bool, at_destination: bool) -> bool {
    matches_definition && at_destination
}

#[test]
fn exercise_dummy_must_be_at_the_route_destination() {
    assert!(!dummy_at_route_destination(true, false));
    assert!(!dummy_at_route_destination(false, true));
    assert!(dummy_at_route_destination(true, true));
}

fn visible_dummy(
    view: &oteryn_protocol_oteryn::world_map::MapTile,
    destination: TilePos,
    refs: &BTreeSet<std::num::NonZeroU32>,
) -> Option<ItemHandle> {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
    let at_destination = tile(view.position).ok() == Some(destination);
    view.items
        .iter()
        .find_map(|item| match (item.definition, item.origin) {
            (MapDefinition::Item(reference), MapOrigin::Handle(handle))
                if dummy_at_route_destination(refs.contains(&reference), at_destination) =>
            {
                Some(handle)
            }
            _ => None,
        })
}

fn dummy_positions(bytes: &[u8]) -> TestResult<std::collections::BTreeSet<TilePos>> {
    let mut positions = std::collections::BTreeSet::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            if tile.items.iter().any(|item| {
                let key = &manifest.palette[item.palette as usize].key;
                (28558..=28565).any(|id| key == &format!("oteryn:item.tibia.i{id}"))
            }) {
                positions.insert(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    Ok(positions)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x41, 0).await?;
    let (status_bytes, status) = exercise(&mut client, wire::ExerciseIntent::Status).await?;
    if status.disposition != 2 || status.revision == 0 {
        return Err(format!("STATUS reply={status:?}").into());
    }
    capture("p15-status.bin", &status_bytes);

    let missing = ItemHandle::new(u64::MAX).ok_or("nonzero refusal handle")?;
    let (refused_bytes, refused) = exercise(
        &mut client,
        wire::ExerciseIntent::Start {
            weapon: missing,
            dummy: missing,
        },
    )
    .await?;
    if refused != wire::TrainingResult::REFUSED {
        return Err(format!("invalid START reply={refused:?}").into());
    }
    capture("p15-refused.bin", &refused_bytes);

    let weapon = carried_weapon(&client)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let positions = dummy_positions(&bytes)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| {
        positions.iter().any(|dummy| {
            at.floor == dummy.floor && at.x.abs_diff(dummy.x).max(at.y.abs_diff(dummy.y)) <= 1
        })
    })
    .ok_or("no player-walkable route to an authored exercise dummy")?;
    let end = path
        .iter()
        .try_fold(start, |at, direction| destination(world, at, *direction))
        .ok_or("exercise route landing is unavailable")?;
    let planned = positions
        .iter()
        .copied()
        .find(|at| at.floor == end.floor && at.x.abs_diff(end.x).max(at.y.abs_diff(end.y)) <= 1)
        .ok_or("route planned no adjacent authored exercise dummy")?;
    let position = ActorPosition {
        x: i32::from(planned.x),
        y: i32::from(planned.y),
        floor: -i16::from(planned.floor),
    };
    let index = provisioning::item_index()?;
    let refs = (28558..=28565)
        .map(|id| provisioning::item_reference(&index, id))
        .collect::<TestResult<BTreeSet<_>>>()?;
    let mut projected = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("world-map snapshot missing")?
        .tiles
        .into_iter()
        .find(|tile| tile.position == position);
    walk_observing(&mut client, world, &path, |events| {
        observe_map_tile(&mut projected, position, events)
    })
    .await?;
    let dummy = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            observe_map_tile(&mut projected, position, client.take_events());
            if let Some(handle) = projected
                .as_ref()
                .and_then(|view| visible_dummy(view, planned, &refs))
            {
                return Ok::<_, oteryn_dev_client::DevClientError>(handle);
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await
    .map_err(|_| format!("no live handle for the exercise dummy at {position:?}"))??;
    let (running_bytes, running) =
        exercise(&mut client, wire::ExerciseIntent::Start { weapon, dummy }).await?;
    if running.disposition != 1 || running.revision == 0 || running.charges == 0 {
        return Err(format!("START reply={running:?}").into());
    }
    capture("p15-running.bin", &running_bytes);
    let (stopped_bytes, stopped) = exercise(&mut client, wire::ExerciseIntent::Stop).await?;
    if stopped.disposition != 2 || stopped.charges > running.charges {
        return Err(format!("STOP reply={stopped:?} after {running:?}").into());
    }
    capture("p15-stopped.bin", &stopped_bytes);
    let _ = client.session_mut().logout().await?;
    let expected = format!(
        "status_disposition={}\nstatus_charges={}\nrefused_disposition={}\nrunning_disposition={}\nrunning_charges={}\nstopped_disposition={}\nstopped_charges={}\n",
        status.disposition,
        status.charges,
        refused.disposition,
        running.disposition,
        running.charges,
        stopped.disposition,
        stopped.charges
    );
    capture("p15-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P15.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P15.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P15.client status=FAIL evidence=timeout"),
    }
}
