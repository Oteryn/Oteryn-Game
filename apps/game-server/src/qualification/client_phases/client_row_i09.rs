//! CLIENT-I09 capture: rotate compiled, fixed chair 2358 through ordinary USE, then capture the
//! domain-17 map delta that reports chair 2359. The replay owner is `oteryn-session`:
//! `crates/session/src/map_transport.rs::apply` decodes and validates the delta and queues the
//! client-visible `SessionEvent::WorldMap`; `Session::take_events` exposes it to the native client.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};

// This non-House chair is adjacent to the chair reserved for I09.full.
const CHAIR: TilePos = TilePos {
    x: 32354,
    y: 32206,
    floor: -7,
};

#[test]
fn i09_client_and_full_each_rotate_a_revision_zero_chair_in_either_order() -> TestResult {
    use crate::map::facts::BundleFacts;
    use crate::map::view::EntryFacts;

    let before = EntryFacts {
        definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
        terrain_kind: None,
        appearance_id: 2358,
        blocks_projectile: false,
        pickupable: false,
        bound: false,
        count: 1,
        sub_type: 0,
    };
    let after = EntryFacts {
        appearance_id: 2359,
        ..before
    };
    let full = super::full_e2ef3::FULL_I09_CHAIR;
    for order in [[CHAIR, full], [full, CHAIR]] {
        let facts = BundleFacts::default();
        for at in order {
            let key = crate::map::view::placement_key(at, 1).ok_or("chair placement")?;
            assert_eq!(facts.rotate_base(key, 0, before, after), UseDisposition::Committed);
        }
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=PASS evidence=domain-17-furniture-rotation-replay-captured",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=FAIL evidence={}",
            error
                .to_string()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("-")
        )),
        Err(_) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=I09.client status=FAIL evidence=journey-timeout",
        ),
    }
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::write(Path::new(&directory).join(name), bytes)?;
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let at = CHAIR;
    let mut client = connect_world(clients, 0xc9).await?;
    let path = route(
        world,
        tile(client.world_spatial().actor_position)?,
        |cell| cell.floor == at.floor && cell.x.abs_diff(at.x).max(cell.y.abs_diff(at.y)) <= 1,
    )
    .ok_or("no route to fixed chair 2358")?;
    walk(&mut client, world, &path).await?;
    // STEP can return before the trailing map-view update. Apply it before
    // taking the predecessor so it cannot be mistaken for the rotation.
    client.service_liveness(Duration::from_millis(500)).await?;
    let chair_position = ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    };
    let map = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("I09 map snapshot absent")?;
    let ordinal = map
        .tiles
        .iter()
        .find(|tile| tile.position == chair_position)
        .and_then(|tile| {
            tile.items.iter().find_map(|item| match item.origin {
                MapOrigin::BaseOrdinal {
                    ordinal,
                    object_revision: 0,
                } if item.appearance_id == 2358 => Some(ordinal),
                _ => None,
            })
        })
        .ok_or("I09 initial chair 2358 absent from served map")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(at, ordinal)
            .ok_or("chair placement")?
            .to_be_bytes(),
    );
    let intent = |revision| {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::WorldObject(WorldObjectTarget {
                placement: placement.clone(),
                expected_revision: revision,
            }),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("I09 intent encode: {error:?}"))
    };
    let before = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .cloned()
        .ok_or("I09 predecessor map snapshot absent")?;
    let before_sequence = client.last_server_sequence();
    client.take_events();
    let reply = client.session_mut().world_context_use(&intent(0)?).await?;
    if reply.status != CommandStatus::Accepted
        || decode_use_result(&reply.payload)
            .map_err(|error| format!("I09 result decode: {error:?}"))?
            != UseDisposition::Committed
    {
        return Err(format!("rotation refused: {:?}", reply.status).into());
    }
    capture("i09-result.bin", &reply.payload)?;
    let delta = verdict::indexed_timeout(Duration::from_secs(5), async {
        loop {
            for event in client.take_events() {
                match event {
                    SessionEvent::WorldMap(delta)
                        if delta.server_sequence > reply.server_sequence
                            && delta
                                .value
                                .tiles
                                .iter()
                                .any(|tile| tile.position == chair_position) =>
                    {
                        return TestResult::Ok(delta);
                    }
                    SessionEvent::WorldMapSnapshot(_) => {
                        return Err("I09 map replaced between predecessor and rotation".into());
                    }
                    _ => {}
                }
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        }
    })
    .await
    .map_err(|_| "I09 rotation sent no map delta")??;
    if before.revision != delta.base_revision || before_sequence >= delta.server_sequence {
        return Err("I09 rotation did not follow the captured map predecessor".into());
    }
    let sequence = delta.server_sequence - 1;
    capture("i09-map-before.bin", &before.payload)?;
    capture(
        "i09-map-before.txt",
        format!(
            "{sequence} {} {} {}\n",
            before.domain_id, before.snapshot_type, before.revision
        )
        .as_bytes(),
    )?;
    let tile = delta
        .value
        .tiles
        .iter()
        .find(|tile| tile.position == chair_position)
        .ok_or("rotation delta omitted target tile")?;
    let item = tile.items.iter().find(|item| matches!(item.origin, MapOrigin::BaseOrdinal { ordinal: value, object_revision: 1 } if value == ordinal)).ok_or("rotation delta omitted revised furniture")?;
    let MapDefinition::Item(definition) = item.definition else {
        return Err("rotated furniture was not an item".into());
    };
    if item.appearance_id != 2359 {
        return Err("I09 rotated chair has the wrong appearance".into());
    }
    let payload = oteryn_protocol_oteryn::world_map::encode_world_map_delta(
        &delta.value.header,
        &delta.value,
    )
    .map_err(|error| format!("I09 capture encode: {error:?}"))?;
    capture("i09-map-delta.bin", &payload)?;
    capture(
        "i09-expected.txt",
        format!(
            "{} {} {} {} {}\n",
            delta.base_revision,
            delta.new_revision,
            delta.server_sequence,
            definition,
            item.appearance_id
        )
        .as_bytes(),
    )?;
    let stale = client.session_mut().world_context_use(&intent(0)?).await?;
    if decode_use_result(&stale.payload).map_err(|error| format!("I09 stale decode: {error:?}"))?
        != UseDisposition::StaleState
    {
        return Err("stale rotation was not refused".into());
    }
    full_e2ef5::player_logout(&mut client).await
}
