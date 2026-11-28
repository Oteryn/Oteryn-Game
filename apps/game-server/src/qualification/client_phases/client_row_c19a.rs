//! C19a proves the starter backpack on the live join, after relog, and by replaying
//! the original CharacterInventory domain through the client inventory decoder.
use super::*;
use oteryn_protocol_oteryn::{item_view, logout::LogoutResult};
use std::io::Write as _;

async fn persisted_backpack(
    clients: &SeamClients<'_>,
) -> TestResult<Option<(String, String, i64, bool)>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT s.item_instance_id::text, i.definition_production_key, i.quantity,
                EXISTS (
                    SELECT 1 FROM game_item_container_entries e
                    JOIN game_item_instances carried USING (item_instance_id, world_id)
                    WHERE e.character_id=s.character_id
                      AND e.parent_item_instance_id=s.item_instance_id
                      AND e.world_id=s.world_id
                      AND carried.definition_production_key='oteryn:item.tibia.i2854'
                )
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

fn require_single_backpack(
    inventory: &item_view::CharacterInventory,
    reference: std::num::NonZeroU32,
) -> TestResult {
    let backpack = inventory.main_backpack.ok_or("no starter backpack")?;
    if backpack.item_definition_ref != reference || backpack.count.get() != 1 {
        return Err(format!("unexpected starter backpack {backpack:?}, expected one {reference}").into());
    }
    if inventory.entries.iter().chain(inventory.equipment.iter().map(|slot| &slot.item))
        .any(|item| item.item_definition_ref == reference)
    {
        return Err("extra starter backpack outside the main slot".into());
    }
    Ok(())
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
    require_single_backpack(&inventory, backpack_ref)?;
    let persisted = persisted_backpack(&scoped)
        .await?
        .ok_or("starter backpack not persisted")?;
    if persisted.1 != "oteryn:item.tibia.i2854" || persisted.2 != 1 || persisted.3 {
        return Err(format!("unexpected durable starter backpack {persisted:?}").into());
    }

    let capture_id = format!(
        "{}-{sequence}-{}",
        uuid_text(session.as_bytes()),
        snapshot.revision
    );
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let capture_dir = root.join(format!("c19a-{capture_id}"));
    std::fs::create_dir(&capture_dir)?;
    let path = capture_dir.join("inventory.bin");
    let mut capture = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    capture.write_all(&snapshot.payload)?;
    drop(capture);
    client.session_mut().verify_inventory_replay(
        &std::fs::read(&path)?,
        snapshot.revision,
        sequence,
    )?;
    evidence(&format!(
        "C19A_CLIENT_REPLAY status=PASS capture={capture_id} malformed_delta=rejected_inventory_unchanged"
    ));

    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("C19a first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(&scoped, 0x33, 0).await?;
    require_single_backpack(fresh.inventory().ok_or("relog inventory absent")?, backpack_ref)?;
    if fresh.join_snapshot().game_session_id == session
        || persisted_backpack(&scoped).await?.as_ref() != Some(&persisted)
    {
        return Err("C19a relog did not retain the same durable starter backpack".into());
    }
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("C19a final logout refused: {logout:?}").into());
    }
    evidence("C19A_PG_ASSERTIONS status=PASS prestate=empty poststate=one relog=same_instance");
    Ok(format!(
        "backpack_ref={backpack_ref} capture={capture_id} durable_instance={}",
        persisted.0
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let (status, detail) =
        match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
            .await
        {
            Ok(Ok(observed)) => ("PASS", observed),
            Ok(Err(error)) => ("FAIL", error.to_string().replace(' ', "_")),
            Err(_) => ("FAIL", "timeout".to_owned()),
        };
    evidence(&format!(
        "e2e_all scenario=C19a.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}


#[test]
fn c19a_rejects_an_extra_backpack_with_a_distinct_handle() -> TestResult {
    let reference = std::num::NonZeroU32::new(2854).ok_or("starter definition")?;
    let main = item_view::ItemEntry {
        handle: item_view::ItemHandle::new(1).ok_or("main handle")?,
        item_definition_ref: reference,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
        appearance_id: 0,
    };
    let mut inventory = item_view::CharacterInventory {
        main_backpack: Some(main),
        ..Default::default()
    };
    require_single_backpack(&inventory, reference)?;
    inventory.entries.push(item_view::ItemEntry {
        handle: item_view::ItemHandle::new(2).ok_or("extra handle")?,
        ..main
    });
    let wire = item_view::encode_character_inventory(&inventory).map_err(|e| format!("{e:?}"))?;
    let decoded = item_view::decode_character_inventory_with_appearances(&wire, false, false)
        .map_err(|e| format!("{e:?}"))?;
    // The previous verdict's main-slot predicate accepts this exact wire inventory.
    assert!(decoded.main_backpack.is_some_and(|item| {
        item.item_definition_ref == reference && item.count.get() == 1
    }));
    let Err(error) = require_single_backpack(&decoded, reference) else {
        return Err("duplicate backpack passed".into());
    };
    assert_eq!(error.to_string(), "extra starter backpack outside the main slot");
    Ok(())
}
