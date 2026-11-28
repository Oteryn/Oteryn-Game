//! C02 native-client verification applies the first live ammunition debit and
//! checks its durable equipment state on a fresh admission.

use super::*;
use full_row_c02::{approach_rat, clear_bow_offhand, count, equip, logout, reference, retreat_from_combat, stop};
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, item_view};
use std::{io::Write, num::NonZeroU32, path::PathBuf};

const BOW: u32 = 3350;
const ARROW: u32 = 3447;
const PROOF_RANGE: u16 = 4;

fn ammo(inventory: &item_view::CharacterInventory, arrow: NonZeroU32) -> Option<u32> {
    inventory.equipment.iter().find_map(|entry| {
        (entry.slot == item_view::EquipmentSlot::Ammo && entry.item.item_definition_ref == arrow)
            .then_some(entry.item.count.get())
    })
}

fn replay_capture(
    client: &mut DevClientSession,
    before: &[u8],
    after: &[u8],
    result_sequence: u64,
    debit_sequence: u64,
    arrow: NonZeroU32,
) -> TestResult {
    let root = PathBuf::from(required("E2E_ALL_CAPTURE_DIR")?);
    let admission = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let metadata = format!("{admission} {result_sequence} {debit_sequence} {arrow}\n");
    for (name, bytes) in [
        ("c02-before.wire", before),
        ("c02-debit.wire", after),
        ("c02-admission.txt", metadata.as_bytes()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))?
            .write_all(bytes)?;
    }
    if std::fs::read_to_string(root.join("c02-admission.txt"))? != metadata {
        return Err("C02 replay admission mismatch".into());
    }
    let before = std::fs::read(root.join("c02-before.wire"))?;
    let after = std::fs::read(root.join("c02-debit.wire"))?;
    let pre_sequence = oteryn_protocol_oteryn::decode_wire_envelope(&before)?.server_sequence();
    let session = client.session_mut();
    let (_, pre_revision, pre_inventory) =
        session.replay_inventory_frame(&before, pre_sequence)?;
    let (base_revision, _, post_inventory) =
        session.replay_inventory_frame(&after, debit_sequence)?;
    if pre_sequence >= result_sequence
        || result_sequence >= debit_sequence
        || pre_revision != base_revision
        || ammo(&pre_inventory, arrow) != Some(2)
        || ammo(&post_inventory, arrow) != Some(1)
        || ammo_slot_handle(&pre_inventory, arrow) != ammo_slot_handle(&post_inventory, arrow)
    {
        return Err("C02 original inventory replay did not prove the exact arrow debit".into());
    }
    evidence(&format!(
        "C02_CLIENT_REPLAY status=PASS session={admission} pre={pre_sequence} debit={debit_sequence}"
    ));
    Ok(())
}

fn ammo_slot_handle(
    inventory: &item_view::CharacterInventory,
    arrow: NonZeroU32,
) -> Option<item_view::ItemHandle> {
    inventory.equipment.iter().find_map(|entry| {
        (entry.slot == item_view::EquipmentSlot::Ammo && entry.item.item_definition_ref == arrow)
            .then_some(entry.item.handle)
    })
}

async fn first_debit(
    client: &mut DevClientSession,
    arrow: NonZeroU32,
    result_sequence: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    verdict::Harness::global().bounded_wait(deadline, "ammunition-debit");
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event {
                if delta.server_sequence <= result_sequence || ammo(&delta.value, arrow) != Some(1)
                {
                    return Err(format!("C02 unexpected first inventory delta: {delta:?}").into());
                }
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C02 first attack emitted no inventory delta".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_C02_ACCOUNT",
        Some("Ranged Client"),
    )
    .await?;
    let clients = dedicated.clients(clients);
    if !requires_platform_grant(&clients.accounts[0]) {
        return Err(full_row_c02::PLATFORM_ADMISSION_GAP.into());
    }

    let mut client = connect_character(&clients, 0xc2, 0).await?;
    character_prep::grant_item(&client, BOW, 1).await?;
    character_prep::grant_item(&client, ARROW, 2).await?;
    let prepared_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&clients, 0xc2, 0).await?;
    let equipped_session = client.join_snapshot().game_session_id;
    let bow = reference(BOW)?;
    let arrow = reference(ARROW)?;
    if prepared_session == equipped_session
        || count(&client, bow)? != 1
        || count(&client, arrow)? != 2
    {
        return Err("C02 ranged kit preparation did not persist".into());
    }
    client.session_mut().enable_inventory_replay_capture();
    clear_bow_offhand(&mut client).await?;
    equip(&mut client, bow, item_view::EquipmentSlot::RightHand).await?;
    equip(&mut client, arrow, item_view::EquipmentSlot::Ammo).await?;
    approach_rat(&mut client, world, &metadata.spawns, PROOF_RANGE).await?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Creature
                    && entity.position.floor == here.floor
                    && entity
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entity.position.y.abs_diff(here.y))
                        <= u32::from(PROOF_RANGE)
                    && entity.display_name.is_some_and(|name| {
                        name.as_str()
                            .is_ok_and(|name| name.eq_ignore_ascii_case("rat"))
                    })
            })
        })
        .ok_or("C02 attack route found no Rat in firing range")?
        .entity;
    if client
        .inventory()
        .and_then(|inventory| ammo(inventory, arrow))
        != Some(2)
    {
        return Err("C02 ATTACK must start with two equipped arrows".into());
    }
    client.take_events();
    let before = client.session_mut().take_inventory_replay_frame()
        .ok_or("C02 original pre-attack inventory envelope absent")?;
    let reply = client.session_mut().attack_target(Some(&target)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C02 ATTACK refused: {reply:?}").into());
    }
    let debit_sequence = first_debit(&mut client, arrow, reply.result_server_sequence).await?;
    let after = client.session_mut().take_inventory_replay_frame()
        .ok_or("C02 original debit inventory envelope absent")?;
    replay_capture(&mut client, &before, &after, reply.result_server_sequence, debit_sequence, arrow)?;
    stop(&mut client).await?;
    if client
        .inventory()
        .and_then(|inventory| ammo(inventory, arrow))
        != Some(1)
    {
        return Err("C02 ammunition changed again before disengagement".into());
    }
    retreat_from_combat(&mut client, world).await?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&clients, 0xc3, 0).await?;
    if fresh.join_snapshot().game_session_id == equipped_session
        || fresh
            .inventory()
            .and_then(|inventory| ammo(inventory, arrow))
            != Some(1)
        || count(&fresh, arrow)? != 1
        || count(&fresh, bow)? != 1
    {
        return Err("C02 readmission did not preserve the exact ammunition debit".into());
    }
    logout(&mut fresh).await?;
    evidence("C02_CLIENT_CHECK attack=Ok equipped_ammo=2->1 readmission_ammo=1");
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::observe(journey(clients)).await;
    let status = match result {
        Ok(()) => "PASS".to_owned(),
        Err(error) => format!(
            "FAIL evidence={}",
            error.to_string().replace([' ', '\n', '\r'], "-")
        ),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C02.client status={status} seconds={:.3}",
        started.elapsed().as_secs_f64(),
    ));
}
