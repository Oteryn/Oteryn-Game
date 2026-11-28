//! G36 drives the authored Soulpit obelisk and checks the durable Soul Core debit.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object::{
    self, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

const OBELISK: (u16, u16, i8) = (32375, 31157, -8);
const ENTRY_SLOT: (u16, u16, i8) = (32375, 31158, -8);
const CORE: u32 = 47381;
const ARENA_ENTRY: (u16, u16, i8) = (32373, 31151, -8);
const LOBBY_RETURN: (u16, u16, i8) = (32373, 31158, -8);
const OBELISK_KEY: &str = "oteryn:item.tibia.i47367";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "verified-refusals-instance-entry-and-persisted-core-debit".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G36.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// The refusal the owner must answer for a core-less character.
fn expected_refusal(at_entry_slot: bool) -> UseDisposition {
    if at_entry_slot {
        UseDisposition::StaleState
    } else {
        UseDisposition::TooFar
    }
}

/// Bundle placement key of the obelisk's base entry: tile x, y, depth and item ordinal.
fn placement_key(x: u16, y: u16, floor: i8, ordinal: u8) -> Option<u64> {
    let depth = u8::try_from(-i16::from(floor)).ok()?;
    Some(u64::from(x) << 32 | u64::from(y) << 16 | u64::from(depth) << 8 | u64::from(ordinal))
}

fn obelisk_ordinal(bytes: &[u8]) -> TestResult<Option<u8>> {
    let mut found = None;
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        if sector.floor != OBELISK.2 {
            return Ok(());
        }
        for tile in sector.tiles {
            if tile.x != OBELISK.0 || tile.y != OBELISK.1 {
                continue;
            }
            for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate() {
                if manifest.palette[item.palette as usize].key == OBELISK_KEY {
                    found = u8::try_from(ordinal).ok();
                }
            }
        }
        Ok(())
    })?;
    Ok(found)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    Ok(())
}

fn position(client: &DevClientSession) -> TestResult<(u16, u16, i8)> {
    let at = tile(client.world_spatial().actor_position)?;
    Ok((at.x, at.y, at.floor))
}

fn core_count(client: &DevClientSession, definition: std::num::NonZeroU32) -> TestResult<u64> {
    let inventory = client.inventory().ok_or("step=inventory;observed=absent")?;
    Ok(inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|equipped| &equipped.item))
        .filter(|entry| entry.item_definition_ref == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn obelisk_use(
    client: &mut DevClientSession,
    payload: &[u8],
    expected: UseDisposition,
) -> TestResult {
    let session = client.join_snapshot().game_session_id;
    let response = client.session_mut().world_context_use(payload).await?;
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        use std::io::Write as _;
        let directory = Path::new(&directory);
        let stem = format!("g36-{expected:?}");
        let mut packet = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{stem}.bin")))?;
        packet.write_all(&response.payload)?;
        let mut metadata = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(format!("{stem}.json")))?;
        serde_json::to_writer(
            &mut metadata,
            &serde_json::json!({
                "session": uuid_text(session.as_bytes()),
                "command_id": response.command_id,
                "server_sequence": response.server_sequence,
                "status": format!("{:?}", response.status),
            }),
        )?;
    }
    let disposition = world_object::decode_use_result(&response.payload).map_err(|error| {
        format!(
            "step=use;observed=decode:{error:?};status={:?}",
            response.status
        )
    })?;
    if response.status != CommandStatus::Accepted || disposition != expected {
        return Err(format!(
            "step=use;observed=status:{:?},disposition:{disposition:?};expected={expected:?}",
            response.status
        )
        .into());
    }
    Ok(())
}

async fn require_refusal(
    client: &mut DevClientSession,
    payload: &[u8],
    definition: std::num::NonZeroU32,
    at_slot: bool,
) -> TestResult {
    let before = position(client)?;
    let session = client.join_snapshot().game_session_id;
    let inventory = client
        .inventory()
        .ok_or("step=inventory;observed=absent")?
        .clone();
    if (before == ENTRY_SLOT) != at_slot || core_count(client, definition)? != 0 {
        return Err("step=refusal;observed=wrong-precondition".into());
    }
    obelisk_use(client, payload, expected_refusal(at_slot)).await?;
    if position(client)? != before
        || client.join_snapshot().game_session_id != session
        || client.inventory() != Some(&inventory)
    {
        return Err("step=refusal;observed=unexpected-state-change".into());
    }
    Ok(())
}

async fn stage_position(clients: &SeamClients<'_>) -> TestResult {
    wait_character_released(clients, 0).await?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let affected = sqlx::query(
        "UPDATE game_character_last_positions p
            SET tile_x=$3, tile_y=$4, tile_floor=$5, write_sequence=write_sequence+1
          WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
            AND NOT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s
                            WHERE s.character_id=p.character_id AND s.session_state<>3)",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i32::from(ENTRY_SLOT.0))
    .bind(i32::from(ENTRY_SLOT.1))
    .bind(-i16::from(ENTRY_SLOT.2))
    .execute(&mut database)
    .await?
    .rows_affected();
    database.close().await?;
    if affected != 1 {
        return Err(
            format!("step=fixture-placement;observed=updated-{affected}-saved-positions").into(),
        );
    }
    evidence("row=G36.full fixture=q_g36 input=entry-slot position=32375,31158,8");
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let ordinal = obelisk_ordinal(&bytes)?.ok_or("step=bundle;needed=obelisk-i47367-on-map")?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let key = placement_key(OBELISK.0, OBELISK.1, OBELISK.2, ordinal).ok_or("step=bundle;key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let index = provisioning::item_index()?;
    let definition = provisioning::item_reference(&index, CORE)?;
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("step=encode;observed={error:?}"))?;

    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_G36_ACCOUNT").await?;
    let clients = dedicated.clients(clients);
    let mut client = connect_character(&clients, 0xe7, 0).await?;
    require_refusal(&mut client, &payload, definition, false).await?;
    logout(&mut client).await?;
    drop(client);

    stage_position(&clients).await?;
    let mut client = connect_character(&clients, 0xe7, 0).await?;
    require_refusal(&mut client, &payload, definition, true).await?;
    character_prep::grant_item(&client, CORE, 1).await?;
    evidence("row=G36.full fixture=q_g36 input=soul-core item=47381 count=1");
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(&clients, 0xe7, 0).await?;
    let channel = client.join_snapshot().game_session_id;
    if position(&client)? != ENTRY_SLOT || core_count(&client, definition)? != 1 {
        return Err(format!(
            "step=entry-pre-state;position={:?};core_count={}",
            position(&client)?,
            core_count(&client, definition)?
        )
        .into());
    }
    obelisk_use(&mut client, &payload, UseDisposition::Committed).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client.join_snapshot().game_session_id == channel || core_count(&client, definition)? != 0
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=entry;observed=missing-instance-transition-or-core-debit".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let instance = client.join_snapshot().game_session_id;
    let scope = client
        .session_mut()
        .activity_scope()
        .ok_or("step=entry;observed=missing-activity-scope")?;
    if scope.source_session != *channel.as_bytes()
        || scope.session != *instance.as_bytes()
        || !matches!(
            scope.scope,
            oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
        )
        || position(&client)? != ARENA_ENTRY
    {
        return Err("step=entry;observed=wrong-instance-or-arrival".into());
    }
    evidence("row=G36.full entry=instance position=32373,31151,8 core_count=0");
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_character(&clients, 0xe7, 0).await?;
    let session = fresh.join_snapshot().game_session_id;
    if session == channel
        || session == instance
        || position(&fresh)? != LOBBY_RETURN
        || core_count(&fresh, definition)? != 0
    {
        return Err(format!(
            "step=relog;position={:?};core_count={}",
            position(&fresh)?,
            core_count(&fresh, definition)?
        )
        .into());
    }
    evidence("row=G36.full relog=verified position=32373,31158,8 core_count=0");
    logout(&mut fresh).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_depends_on_the_entry_slot() {
        assert_eq!(expected_refusal(true), UseDisposition::StaleState);
        assert_eq!(expected_refusal(false), UseDisposition::TooFar);
    }

    #[test]
    fn placement_key_packs_tile_depth_and_ordinal() {
        assert_eq!(
            placement_key(0x0102, 0x0304, -8, 5),
            Some(0x0102_0000_0000 | 0x0304_0000 | 0x0800 | 5)
        );
        assert_eq!(placement_key(1, 1, 3, 0), None);
        assert_eq!(token("a  b"), "a-b");
    }
}
