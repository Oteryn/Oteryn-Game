//! I13 client capture: use a real Bag You Covet and retain the inventory payload.
use super::*;
use oteryn_protocol_oteryn::item_view;

fn visible(inventory: &item_view::CharacterInventory) -> String {
    let mut rows = inventory
        .entries
        .iter()
        .map(|entry| (entry.item_definition_ref.get(), entry.count.get()))
        .collect::<Vec<_>>();
    rows.sort_unstable();
    rows.into_iter()
        .map(|(item, count)| format!("{item} {count}\n"))
        .collect()
}

fn capture(path: &std::path::Path, name: &str, bytes: &[u8]) {
    if let Err(error) = std::fs::write(path.join(name), bytes) {
        evidence(&format!(
            "I13_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x13).await?;
    let before = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("I13 inventory snapshot absent")?
        .clone();
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("I13 setup logout refused".into());
    }
    drop(client);

    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let inserted: String = sqlx::query_scalar(
        "WITH parent AS (SELECT item_instance_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid LIMIT 1), item AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i43895','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item.item_instance_id,encode($2,'hex')::uuid,encode($1,'hex')::uuid,parent.item_instance_id,COALESCE((SELECT max(placement_ordinal)+1 FROM game_item_container_entries WHERE parent_item_instance_id=parent.item_instance_id),0),game_character_uuid_v7() FROM item,parent RETURNING item_instance_id::text")
        .bind(clients.characters[0].as_slice()).bind(clients.world.as_slice())
        .fetch_one(&mut connection).await?;

    let mut client = connect_world(clients, 0x14).await?;
    let reference = provisioning::item_reference(&provisioning::item_index()?, 43895)?;
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == reference)
        })
        .map(|entry| entry.handle)
        .ok_or("I13 seeded Bag You Covet absent")?;
    client.take_events();
    let result = client.use_item(handle).await?;
    if result.status != CommandStatus::Accepted
        || result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("I13 Bag You Covet use refused: {result:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(
            item_view::STATE_DOMAIN_CHARACTER_INVENTORY,
            item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
        )
        .await?;
    let inventory = item_view::decode_character_inventory_with_equip_drop(&delta.payload, true)
        .map_err(|error| format!("I13 inventory decode: {error:?}"))?;
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        capture(path, "i13-before.bin", &before.payload);
        capture(path, "i13-after.bin", &delta.payload);
        capture(
            path,
            "i13-revisions.txt",
            format!(
                "{} {} {} {}",
                before.revision, delta.base_revision, delta.new_revision, delta.server_sequence
            )
            .as_bytes(),
        );
        capture(path, "i13-expected.txt", visible(&inventory).as_bytes());
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I13.client status=PASS evidence=bag-instance:{inserted};inventory-revision:{}",
        delta.new_revision
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = journey(clients).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I13.client status=FAIL evidence={error}"
        ));
    }
}
