//! C19a proves the starter backpack on the live join, after relog, and by replaying
//! the original CharacterInventory domain through the client inventory decoder.
use super::*;
use oteryn_protocol_oteryn::{item_view, logout::LogoutResult};
use std::io::Write as _;

async fn persisted_backpack(
    clients: &SeamClients<'_>,
) -> TestResult<Option<(String, String, i64, bool)>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    read_backpack_custody(&mut database, &clients.characters[0], &clients.world).await
}

async fn read_backpack_custody(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
    world: &[u8; 16],
) -> TestResult<Option<(String, String, i64, bool)>> {
    use oteryn_gameplay_durability::durability::item_loop_move::{
        ITEM_LOOP_DEPTH_MAX, ITEM_LOOP_TREE_ITEMS_MAX,
    };
    let main = sqlx::query_as(
        "WITH RECURSIVE main AS (
           SELECT item_instance_id, world_id FROM game_item_container_slots
           WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
         ), roots AS (
           SELECT item_instance_id FROM main
           UNION ALL
           SELECT item_instance_id FROM game_character_equipment_slots
           WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid
         ), carried(item, depth) AS (
           SELECT item_instance_id, 0 FROM roots
           UNION ALL
           SELECT child.item_instance_id, c.depth+1 FROM carried c
           CROSS JOIN LATERAL (
             SELECT e.item_instance_id FROM game_item_container_entries e
             WHERE e.parent_item_instance_id=c.item AND e.world_id=encode($2,'hex')::uuid
               AND e.character_id=encode($1,'hex')::uuid
             UNION ALL
             SELECT n.item_instance_id FROM game_item_nested_entries n
             WHERE n.parent_item_instance_id=c.item AND n.world_id=encode($2,'hex')::uuid
           ) child WHERE c.depth <= $3
         ), bounded AS (
           SELECT item, depth FROM carried LIMIT ($4 + 1)
         )
         SELECT s.item_instance_id::text, i.definition_production_key, i.quantity,
                (SELECT count(*) > $4 OR count(*) <> count(DISTINCT b.item)
                     OR COALESCE(bool_or(b.depth > $3 OR carried_item.lifecycle IS DISTINCT FROM 1
                        OR (b.item <> s.item_instance_id
                            AND carried_item.definition_production_key='oteryn:item.tibia.i2854')), false)
                 FROM bounded b LEFT JOIN game_item_instances carried_item
                   ON carried_item.item_instance_id=b.item AND carried_item.world_id=s.world_id)
         FROM main s JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE i.lifecycle=1",
    )
    .bind(character.as_slice())
    .bind(world.as_slice())
    .bind(ITEM_LOOP_DEPTH_MAX)
    .bind(ITEM_LOOP_TREE_ITEMS_MAX + 9)
    .fetch_optional(database)
    .await?;
    Ok(main)
}

fn require_single_backpack(
    inventory: &item_view::CharacterInventory,
    reference: std::num::NonZeroU32,
) -> TestResult {
    let backpack = inventory.main_backpack.ok_or("no starter backpack")?;
    if backpack.item_definition_ref != reference || backpack.count.get() != 1 {
        return Err(
            format!("unexpected starter backpack {backpack:?}, expected one {reference}").into(),
        );
    }
    if inventory
        .entries
        .iter()
        .chain(inventory.equipment.iter().map(|slot| &slot.item))
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
    wait_character_released(&scoped, 0).await?;
    let token = if requires_platform_grant(&scoped.accounts[0]) {
        platform_row_grant(&scoped, 0).await?
    } else {
        let generation = platform_generation(scoped.descriptor, &scoped.accounts[0]).await?;
        let (nonce, attempt) = fresh_grant_material(0x32)?;
        sign_grant(
            &Grant {
                signing: scoped.signing,
                key_id: scoped.key_id,
                account_id: &scoped.accounts[0],
                character_id: scoped.characters[0],
                world_id: scoped.world,
                channel_id: scoped.channel,
                security_generation: generation,
                scope_generation: scoped.scope_generation,
                nonce,
                attempt,
            },
            now_seconds()?,
        )
    };
    use oteryn_dev_client::native_session as native;
    let deadline = Duration::from_secs(20);
    let transport = oteryn_dev_client::connect_transport(oteryn_dev_client::RecoveryTransport {
        address: scoped.address,
        server_name: "localhost",
        root_certificate: scoped.certificate,
        deadline,
    })
    .await?;
    let admission = native::Admission {
        appearance_items: None,
        schema_revision: 1,
        character_id: crate::foundation::CharacterId::decode(&scoped.characters[0])?,
        admission_material: token.as_bytes(),
        client_build_id: "oteryn-dev-client/world-qualification",
        supported_capabilities: native::CLIENT_SUPPORTED_CAPABILITIES,
        deadline,
    };
    let (transport, recording) = native::AdmissionCapture::record(transport);
    let mut client = native::Session::admit(transport, admission).await?;
    let transcript = recording
        .finish()
        .ok_or("C19a admission capture unavailable")?;
    verdict::Harness::global().progress("admission");
    let session = client.join_snapshot().game_session_id;
    let snapshot = client
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("inventory join snapshot absent")?
        .clone();
    let sequence = client.last_server_sequence();
    let inventory = client.inventory().cloned().ok_or("inventory not decoded")?;
    require_single_backpack(&inventory, backpack_ref)?;
    let persisted = persisted_backpack(&scoped)
        .await?
        .ok_or("starter backpack not persisted")?;
    if persisted.1 != "oteryn:item.tibia.i2854" || persisted.2 != 1 || persisted.3 {
        return Err(format!(
            "unexpected durable starter backpack or invalid carried tree {persisted:?}"
        )
        .into());
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
    let path = capture_dir.join("admission.bin");
    let mut capture = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    capture.write_all(&transcript)?;
    drop(capture);
    let bytes = std::fs::read(&path)?;
    if bytes != transcript {
        return Err("C19a saved frames differ from this admission".into());
    }
    let mut replay = native::replay_recorded_admission(
        &bytes,
        native::Admission {
            admission_material: b"local-replay",
            ..admission
        },
    )
    .await?;
    if replay.join_snapshot().game_session_id != session || replay.inventory() != Some(&inventory) {
        return Err("C19a fresh replay differs from the live admission".into());
    }
    replay.verify_inventory_replay(&snapshot.payload, snapshot.revision, sequence)?;
    evidence(&format!(
        "C19A_CLIENT_REPLAY status=PASS capture={capture_id} fresh_admission=verified malformed_delta=rejected_inventory_unchanged"
    ));
    let logout = client.logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("C19a first logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(&scoped, 0x33, 0).await?;
    require_single_backpack(
        fresh.inventory().ok_or("relog inventory absent")?,
        backpack_ref,
    )?;
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
    assert!(
        decoded
            .main_backpack
            .is_some_and(|item| { item.item_definition_ref == reference && item.count.get() == 1 })
    );
    let Err(error) = require_single_backpack(&decoded, reference) else {
        return Err("duplicate backpack passed".into());
    };
    assert_eq!(
        error.to_string(),
        "extra starter backpack outside the main slot"
    );
    Ok(())
}

#[tokio::test]
async fn c19a_custody_rejects_backpack_inside_closed_container() -> TestResult {
    let url = match std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") {
        Ok(url) => url,
        Err(_) => return Ok(()),
    };
    let mut database = sqlx::PgConnection::connect(&url).await?;
    sqlx::raw_sql(
        "CREATE TEMP TABLE game_item_container_slots(character_id uuid, world_id uuid, item_instance_id uuid);
         CREATE TEMP TABLE game_character_equipment_slots(LIKE game_item_container_slots);
         CREATE TEMP TABLE game_item_container_entries(LIKE game_item_container_slots, parent_item_instance_id uuid);
         CREATE TEMP TABLE game_item_nested_entries(world_id uuid, item_instance_id uuid, parent_item_instance_id uuid);
         CREATE TEMP TABLE game_item_instances(item_instance_id uuid, world_id uuid,
             definition_production_key text, quantity bigint, lifecycle smallint);
         INSERT INTO game_item_container_slots VALUES
             ('01010101-0101-0101-0101-010101010101','02020202-0202-0202-0202-020202020202','03030303-0303-0303-0303-030303030303');
         INSERT INTO game_item_container_entries VALUES
             ('01010101-0101-0101-0101-010101010101','02020202-0202-0202-0202-020202020202','04040404-0404-0404-0404-040404040404','03030303-0303-0303-0303-030303030303');
         INSERT INTO game_item_instances VALUES
             ('03030303-0303-0303-0303-030303030303','02020202-0202-0202-0202-020202020202','oteryn:item.tibia.i2854',1,1),
             ('04040404-0404-0404-0404-040404040404','02020202-0202-0202-0202-020202020202','closed-container',1,1),
             ('05050505-0505-0505-0505-050505050505','02020202-0202-0202-0202-020202020202','oteryn:item.tibia.i2854',1,1);",
    ).execute(&mut database).await?;
    let before = read_backpack_custody(&mut database, &[1; 16], &[2; 16])
        .await?
        .ok_or("test main backpack absent")?;
    assert!(!before.3);
    sqlx::raw_sql("INSERT INTO game_item_nested_entries VALUES
        ('02020202-0202-0202-0202-020202020202','05050505-0505-0505-0505-050505050505','04040404-0404-0404-0404-040404040404')")
        .execute(&mut database).await?;
    let after = read_backpack_custody(&mut database, &[1; 16], &[2; 16])
        .await?
        .ok_or("test main backpack absent after nested insert")?;
    assert_eq!(
        (&before.0, &before.1, before.2),
        (&after.0, &after.1, after.2)
    );
    assert!(after.3, "nested duplicate escaped the custody query");
    eprintln!("C19A_NESTED_BACKPACK_PG_ASSERTIONS status=PASS");
    Ok(())
}
