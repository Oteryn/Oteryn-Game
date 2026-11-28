//! Capture the inventory and own-vitals payloads exposed after a real rune command.
use super::*;
use oteryn_protocol_oteryn::{actor_spell, item_view, world_object};
use std::io::Write;

const LIGHT_MAGIC_MISSILE_RUNE: u32 = 3174;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(capture(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=C06.client status=PASS evidence=inventory-and-vitals-payloads",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C06.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("e2e_all scenario=C06.client status=FAIL evidence=240s-journey-deadline")
        }
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let capture_root = required("E2E_ALL_CAPTURE_DIR")?;
    let caster =
        character_prep::dedicated_character(clients, "NODE_BOOT_C06_CASTER_ACCOUNT").await?;
    let (accounts, characters) = caster.replace_slot(clients, 1)?;
    let scope = SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let clients = &scope;
    let mut client = connect_world(clients, 0xc6).await?;
    character_prep::grant_item(&client, LIGHT_MAGIC_MISSILE_RUNE, 1).await?;
    let preparation = client.session_mut().logout().await?;
    if preparation.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C06 reagent preparation logout: {:?}", preparation.result).into());
    }
    drop(client);
    let mut client = connect_world(clients, 0xc6).await?;
    let before_inventory = client.inventory().cloned().ok_or("C06 inventory absent")?;
    let before_vitals = client.actor_vitals().copied().ok_or("C06 vitals absent")?;
    let index = provisioning::item_index()?;
    let rune = provisioning::item_reference(&index, LIGHT_MAGIC_MISSILE_RUNE)?;
    let handle = before_inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == rune)
        .ok_or("C06 prepared rune absent")?
        .handle;

    // A real backpack handle reaches the rune bridge. Its cross-floor target must
    // be refused before charging the item or changing vitals.
    let position = client.world_spatial().actor_position;
    let invalid = world_object::UseIntent {
        target: world_object::UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: None,
        use_with_position: Some(actor_spell::SpellTargetPosition {
            x: position.x,
            y: position.y,
            floor: position
                .floor
                .checked_sub(1)
                .ok_or("C06 cross-floor target overflow")?,
        }),
    };
    let payload = world_object::encode_item_use_intent(&invalid)
        .map_err(|error| format!("C06 refusal encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (payload, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("C06 refusal detail: {error:?}"))?;
    let disposition = world_object::decode_use_result(payload)
        .map_err(|error| format!("C06 use result: {error:?}"))?;
    if reply.status != CommandStatus::Accepted
        || disposition != world_object::UseDisposition::TargetIllegal
    {
        return Err(format!(
            "C06 expected TargetIllegal, got {:?}/{disposition:?}",
            reply.status
        )
        .into());
    }
    client.service_liveness(Duration::from_millis(20)).await?;
    if client.inventory() != Some(&before_inventory)
        || client.actor_vitals() != Some(&before_vitals)
    {
        return Err("C06 invalid target changed client inventory or vitals".into());
    }

    let session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C06 refusal readback logout: {:?}", logout.result).into());
    }
    let session_api = client.session_mut();
    let (sequence, raw_body) = session_api.qualification_join_body();
    let file = Path::new(&capture_root).join(format!(
        "c06-{}-{sequence}.snapshot",
        uuid_text(session.as_bytes())
    ));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)?
        .write_all(raw_body)?;
    let (inventory, vitals) = session_api.qualification_replay_join_inventory_and_vitals(
        session,
        sequence,
        &std::fs::read(&file)?,
    )?;
    if inventory != before_inventory || vitals != before_vitals {
        return Err("C06 wire replay changed inventory or vitals".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xc7).await?;
    let fresh_inventory = fresh.inventory().ok_or("C06 relog inventory absent")?;
    let charges = |inventory: &item_view::CharacterInventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == rune)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>()
    };
    if fresh.join_snapshot().game_session_id == session
        || charges(fresh_inventory) != charges(&before_inventory)
    {
        return Err("C06 refusal changed durable rune custody".into());
    }

    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C06 final logout: {:?}", logout.result).into());
    }

    Ok(())
}
