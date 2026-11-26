//! Captures the real actor-vitals payload consumed by the native session reducer for I15.
use super::*;
use oteryn_dev_client::{
    DevClientSession, SessionEvent, native_session::item_movement::MoveRequest,
};
use oteryn_protocol_oteryn::{actor_spell, item_view as items, logout::LogoutResult};

const FIELD: TilePos = TilePos {
    x: 32474,
    y: 32249,
    floor: -8,
};
const FIELD_DAMAGE: u32 = 100;
const RING_COST: u32 = 2 * FIELD_DAMAGE;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=I15.client status=PASS evidence=actor-vitals-2-to-1-mana-shield",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=I15.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=I15.client status=FAIL evidence=damage-delta-timeout"),
    }
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = Path::new(&path).join(name);
        std::fs::File::create_new(path)?.write_all(bytes)?;
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("I15 logout refused: {outcome:?}").into());
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
    let field = world
        .base()
        .tile(FIELD.x, FIELD.y, FIELD.floor)
        .ok_or("I15 native field tile missing")?;
    if field.protection_zone()
        || !(0..field.depths().iter().filter(|depth| **depth == 0).count())
            .filter_map(|ordinal| u8::try_from(ordinal).ok())
            .any(|ordinal| field.appearance(ordinal) == Some(2126))
    {
        return Err(format!("I15 expected native Energy Field 2126 at {FIELD:?}").into());
    }
    let index = provisioning::item_index()?;
    let base = provisioning::item_reference(&index, 3051)?;
    let active = provisioning::item_reference(&index, 3088)?;

    character_prep::declare_seeded(
        "I15.client",
        "level-100 knight,one Energy Ring 3051",
        "observe a full 100-point energy hit through the actor-vitals reducer",
    );
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I15_ACCOUNT").await?;
    let clients = dedicated.clients(clients);
    let mut client = connect_character(&clients, 0xb2, 0).await?;
    let seed_session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(seed_session.as_bytes()),
        "--profile",
        "explorer".into(),
    )
    .await?;
    character_prep::grant_item(&client, 3051, 1).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(&clients, 0xb2, 0).await?;
    if seed_session == client.join_snapshot().game_session_id {
        return Err("I15 fixture read reused the preparation session".into());
    }
    let initial = *client.actor_vitals().ok_or("I15 admission vitals absent")?;
    if (initial.max_health, initial.max_mana) != (1565, 550) {
        return Err(
            format!("I15 expected level-100 knight vitals 1565/550, got {initial:?}").into(),
        );
    }
    let inventory = client.inventory().ok_or("I15 inventory absent")?;
    let rings: Vec<_> = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == base)
        .collect();
    let [ring] = rings.as_slice() else {
        return Err("I15 expected exactly one carried Energy Ring 3051".into());
    };
    if ring.count.get() != 1
        || inventory
            .equipment
            .iter()
            .any(|entry| entry.slot == items::EquipmentSlot::Ring)
    {
        return Err("I15 expected one inactive Energy Ring 3051 and an empty Ring slot".into());
    }
    let source = ring.handle;
    let moved = client
        .session_mut()
        .move_item_counted(MoveRequest {
            source,
            destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
            count: None,
        })
        .await
        .map_err(|error| format!("I15 equip command failed: {error}"))?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("I15 equip refused: {moved:?}").into());
    }
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(&clients, 0xb2, 0).await?;
    let equipped = client
        .inventory()
        .ok_or("I15 relog inventory absent")?
        .equipment
        .iter()
        .any(|entry| {
            entry.slot == items::EquipmentSlot::Ring
                && entry.item.item_definition_ref == active
                && entry.item.count.get() == 1
        });
    if session == client.join_snapshot().game_session_id || !equipped {
        return Err("I15 active Energy Ring 3088 absent on fresh admission".into());
    }

    let mut path = route_live(&mut client, world, |at| at == FIELD)
        .await?
        .ok_or_else(|| format!("I15 no route to native Energy Field {FIELD:?}"))?;
    let entry = path.pop().ok_or("I15 already standing on Energy Field")?;
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        step(&mut client, world, from, direction).await?;
    }
    client.take_events();
    let before = *client.actor_vitals().ok_or("I15 pre-hit vitals absent")?;
    let expected_mana = before
        .mana
        .checked_sub(RING_COST)
        .ok_or("I15 insufficient pre-hit mana")?;
    let from = tile(client.world_spatial().actor_position)?;
    let sequence = step(&mut client, world, from, entry).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let after = loop {
        if let Some(delta) = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::ActorVitals(delta) => Some(delta),
                _ => None,
            })
        {
            if delta.server_sequence <= sequence
                || delta.value.health != before.health
                || delta.value.mana != expected_mana
            {
                return Err(format!("I15 field damage={FIELD_DAMAGE} expected health={} mana={expected_mana}, step_sequence={sequence}, got {delta:?}", before.health).into());
            }
            break delta.value;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I15 native Energy Field step emitted no ActorVitals delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };

    logout(&mut client).await?;
    let before = actor_spell::encode_actor_vitals(&before)
        .map_err(|error| format!("I15 before encode: {error:?}"))?;
    let after_payload = actor_spell::encode_actor_vitals(&after)
        .map_err(|error| format!("I15 after encode: {error:?}"))?;
    capture("i15-before.bin", &before)?;
    capture("i15-after.bin", &after_payload)?;
    capture(
        "i15-expected.txt",
        format!("{} {}\n", after.health, expected_mana).as_bytes(),
    )?;
    Ok(())
}

async fn step(
    client: &mut DevClientSession,
    world: &BundleWorld,
    from: TilePos,
    direction: StepDirection,
) -> TestResult<u64> {
    let expected = destination(world, from, direction).ok_or("I15 step has no pinned landing")?;
    let outcome = client.step(direction).await?;
    if outcome.disposition != StepDisposition::Moved
        || outcome.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != expected
    {
        return Err(format!(
            "I15 step {direction:?} from {from:?} toward {expected:?}: {outcome:?}"
        )
        .into());
    }
    client
        .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
        .await?;
    Ok(outcome.result_server_sequence)
}
