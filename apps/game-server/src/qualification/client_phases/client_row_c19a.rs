//! C19a proves the starter backpack on the live join, after relog, and by replaying
//! the original CharacterInventory domain through the client inventory decoder.
use super::*;
use oteryn_protocol_oteryn::{item_view, logout::LogoutResult};
use std::io::Write as _;

async fn persisted_backpack(
    clients: &SeamClients<'_>,
) -> TestResult<Option<(String, String, i64)>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT s.item_instance_id::text, i.definition_production_key, i.quantity
         FROM game_item_container_slots s
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE s.character_id=encode($1,'hex')::uuid
           AND s.world_id=encode($2,'hex')::uuid AND i.lifecycle=1",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .fetch_optional(&mut database)
    .await?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_C19A_ACCOUNT").await?;
    let scoped = fixture.clients(clients);
    if persisted_backpack(&scoped).await?.is_some() {
        return Err("C19a pre-state already has a backpack".into());
    }
    let definitions = provisioning::item_index()?;
    let backpack_ref = provisioning::item_reference(&definitions, 2854)?;
    let mut client = connect_character(&scoped, 0x32, 0).await?;
    let session = client.join_snapshot().game_session_id;
    let snapshot = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("inventory join snapshot absent")?
        .clone();
    let sequence = client.session_mut().last_server_sequence();
    let inventory = client.inventory().cloned().ok_or("inventory not decoded")?;
    let backpack = inventory.main_backpack.ok_or("no starter backpack")?;
    if backpack.item_definition_ref != backpack_ref || backpack.count.get() != 1 {
        return Err(format!(
            "unexpected starter backpack {backpack:?}, expected one {backpack_ref}"
        )
        .into());
    }
    let persisted = persisted_backpack(&scoped)
        .await?
        .ok_or("starter backpack not persisted")?;
    if persisted.1 != "oteryn:item.tibia.i2854" || persisted.2 != 1 {
        return Err(format!("unexpected durable starter backpack {persisted:?}").into());
    }

    let capture_id = format!(
        "{}-{sequence}-{}",
        uuid_text(session.as_bytes()),
        snapshot.revision
    );
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let path = root.join(format!("c19a-{capture_id}.bin"));
    let mut capture = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    capture.write_all(&snapshot.payload)?;
    drop(capture);
    let replay = item_view::decode_character_inventory_with_appearances(
        &std::fs::read(&path)?,
        client
            .selected_capabilities()
            .contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
        client
            .selected_capabilities()
            .contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1),
    )
    .map_err(|error| format!("C19a inventory replay: {error:?}"))?;
    if replay != inventory {
        return Err("C19a replay inventory differs from the admitted inventory".into());
    }
    evidence(&format!(
        "C19A_CLIENT_REPLAY status=PASS capture={capture_id}"
    ));

    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("C19a first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(&scoped, 0x33, 0).await?;
    let fresh_backpack = fresh
        .inventory()
        .and_then(|inventory| inventory.main_backpack);
    if fresh.join_snapshot().game_session_id == session
        || !fresh_backpack
            .is_some_and(|item| item.item_definition_ref == backpack_ref && item.count.get() == 1)
        || persisted_backpack(&scoped).await?.as_ref() != Some(&persisted)
    {
        return Err("C19a relog did not retain the same durable starter backpack".into());
    }
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("C19a final logout refused: {logout:?}").into());
    }
    Ok(format!(
        "backpack_ref={backpack_ref} capture={capture_id} durable_instance={}",
        persisted.0
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C19a.client status=PASS evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C19a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C19a.client status=FAIL evidence=timeout"),
    }
}
