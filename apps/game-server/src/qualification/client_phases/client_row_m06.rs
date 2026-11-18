//! Captures the native bundle door transition for a matching carried key.

use super::*;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[derive(Clone, Copy)]
struct Door {
    at: TilePos,
    ordinal: u8,
}

fn key_doors(bytes: &[u8], world: &BundleWorld) -> TestResult<Vec<Door>> {
    let mut found = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            let at = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                let appearance = crate::map::palette_appearance(
                    &manifest.palette[item.palette as usize].key,
                );
                if appearance == Some(1628) && world.facts().doors().at(at) {
                    found.push(Door {
                        at,
                        ordinal: u8::try_from(ordinal).map_err(|_| {
                            oteryn_world_bundle::Error::Limit("M06 client ordinal".into())
                        })?,
                    });
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

async fn matching_key(clients: &SeamClients<'_>, client: &oteryn_dev_client::DevClientSession) -> TestResult<std::collections::BTreeMap<u16, oteryn_dev_client::ItemEntry>> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let matches: Vec<(String, i32)> = sqlx::query_as(
        "SELECT i.definition_production_key,k.key_number
         FROM game_item_container_entries e
         JOIN game_item_instances i USING (world_id,item_instance_id)
         JOIN game_item_keys k USING (item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid AND i.lifecycle=1"
    ).bind(clients.characters[1].as_slice()).bind(clients.world.as_slice()).fetch_all(&mut db).await?;
    let index = provisioning::item_index()?;
    let inventory = client.inventory().ok_or("M06 inventory absent")?;
    let mut carried = std::collections::BTreeMap::new();
    for (key, actual) in matches {
        let Ok(number) = u16::try_from(actual) else { continue; };
        let Some(id) = key.strip_prefix("oteryn:item.tibia.i").and_then(|id| id.parse().ok()) else { continue; };
        let definition = provisioning::item_reference(&index, id)?;
        let mut candidates = inventory.entries.iter().filter(|entry| entry.item_definition_ref == definition);
        if let Some(item) = candidates.next().copied()
            && is_door_key(definition.get(), u16::try_from(actual).ok(), number, candidates.count() + 1)
        {
            carried.insert(number, item);
        }
    }
    Ok(carried)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc6).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let keys = matching_key(clients, &client).await?;
    let ((door, path), carried) = select_key_door(key_doors(&bytes, world)?, |door| {
        route(world, from, |at| {
            at.floor == door.at.floor
                && at.x.abs_diff(door.at.x).max(at.y.abs_diff(door.at.y)) == 1
        }).map(|path| {
            let key = crate::map::view::placement_key(door.at, door.ordinal)
                .and_then(|key| world.facts().action_at(key))
                .and_then(|number| keys.get(&number).copied());
            ((door, path), key)
        })
    }).ok_or("M06 no reachable authored locked door with a matching carried key")?;
    let mut map = client_row_m05::DoorMap::read(&mut client)?;
    walk_observing(&mut client, world, &path, |event| map.observe(&event)).await?;
    let key = crate::map::view::placement_key(door.at, door.ordinal)
        .ok_or("M06 placement key absent")?;
    for event in client.take_events() { map.observe(&event); }
    let (revision, appearance) = map.state(door.at, door.ordinal)
        .ok_or("M06 locked door absent from current map")?;
    if appearance != 1628 {
        return Err(format!("M06 expected locked door 1628, observed {appearance}").into());
    }
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let cutoff = client.last_server_sequence();
    let intent = UseIntent {
        target: UseTarget::Item(carried.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget { placement, expected_revision: revision })),
        use_with_position: None,
    };
    let payload = encode_item_use_intent(&intent).map_err(|error| format!("{error:?}"))?;
    let result = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&result.payload)
        .map_err(|error| format!("{error:?}"))?;
    if result.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "M06 client key action observed={:?}/{disposition:?}; expected Accepted/Committed",
            result.status
        )
        .into());
    }
    let next = revision.checked_add(1).ok_or("M06 door revision exhausted")?;
    client_row_m05::capture_transition(&mut client, "m06-key-unlocked", &mut map, cutoff, door.at, (door.ordinal, next, 1630)).await?;
    client_row_c08::require_logout(&mut client, Duration::from_secs(15)).await?;
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(journey(clients)).await;
    evidence(&format!(
        "e2e_all scenario=M06.client status={}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        result
            .err()
            .map(|error| format!(" evidence={}", error.to_string().replace(' ', "-")))
            .unwrap_or_default()
    ));
}

fn select_key_door<C, T, K>(
    candidates: impl IntoIterator<Item = C>,
    evaluate: impl FnMut(C) -> Option<(T, Option<K>)>,
) -> Option<(T, K)> {
    candidates.into_iter().filter_map(evaluate).find_map(|(door, key)| key.map(|key| (door, key)))
}

fn is_door_key(definition: u32, actual: Option<u16>, expected: u16, matching_instances: usize) -> bool {
    definition > 0 && actual == Some(expected) && matching_instances == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_action_must_match_door_and_resolve_one_carried_handle() {
        assert_eq!(select_key_door([99, 100, 101], |door| (door != 99).then_some((door, (door == 101).then_some(door)))), Some((101, 101)));
        assert!(!is_door_key(0, Some(100), 100, 1));
        assert!(!is_door_key(121, Some(101), 100, 1));
        assert!(!is_door_key(121, None, 100, 1));
        assert!(!is_door_key(121, Some(100), 100, 2));
        assert!(is_door_key(121, Some(100), 100, 1));
    }
}
