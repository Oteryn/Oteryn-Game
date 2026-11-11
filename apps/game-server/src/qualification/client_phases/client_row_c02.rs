//! C02 client capture: drive the ordinary ATTACK route and retain the inventory payload that the
//! native session consumes. Capture I/O is evidence-only and never changes the journey verdict.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::item_view;

fn write_capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    row_complete(async {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc2).await?;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.rat",
    )
    .await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| entity.kind == oteryn_dev_client::EntityKind::Creature)
        })
        .ok_or("C02 attack route found no visible creature")?
        .entity;
    let reply = client.session_mut().attack_target(Some(&target)).await?;
    if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("C02 ATTACK refused: {reply:?}").into());
    }
    client
        .service_liveness(std::time::Duration::from_millis(500))
        .await?;
    let event_inventory = client.take_events().into_iter().rev().find_map(|event| {
        if let SessionEvent::Inventory(delta) = event {
            Some(delta.value)
        } else {
            None
        }
    });
    let inventory = event_inventory
        .as_ref()
        .or_else(|| client.inventory())
        .ok_or("C02 session has no inventory state")?;
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_some() {
        match item_view::encode_character_inventory_with_equip_drop(inventory, true) {
            Ok(payload) => write_capture("c02-inventory.bin", &payload)?,
            Err(error) => evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=C02.client capture=c02-inventory.bin error={error:?}"
            )),
        }
        let expected = inventory
            .equipment
            .iter()
            .filter(|item| item.slot == item_view::EquipmentSlot::Ammo)
            .map(|item| {
                format!(
                    "{} {}\n",
                    item.item.item_definition_ref, item.item.count
                )
            })
            .collect::<String>();
        write_capture("c02-expected.txt", expected.as_bytes())?;
    }
    let verdict = format!(
        "SEAM_EVIDENCE e2e_all scenario=C02.client status=CAPTURED attack={:?} ammo={}",
        reply.disposition,
        inventory
            .equipment
            .iter()
            .filter(|item| item.slot == item_view::EquipmentSlot::Ammo)
            .map(|item| u64::from(item.item.count.get()))
            .sum::<u64>()
    );
    Ok(verdict)

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    row_result

    }).await?
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = journey(clients).await.inspect(|line| evidence(line)) {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "-")
        ));
    }
}
