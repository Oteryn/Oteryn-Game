//! Capture the inventory and own-vitals payloads exposed after a real rune command.
use super::*;
use oteryn_protocol_oteryn::{actor_spell, item_view, refusal_detail::RefusalDetail, world_object};
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
    let (payload, detail) =
        oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
            .map_err(|error| format!("C06 refusal detail: {error:?}"))?;
    let disposition = world_object::decode_use_result(payload)
        .map_err(|error| format!("C06 use result: {error:?}"))?;
    let refusal = check_floor_refusal(reply.status, disposition, detail);
    let barrier = client.session_mut().attack_target(None).await?;
    if barrier.status != CommandStatus::Accepted
        || barrier.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
        || barrier.result_server_sequence <= reply.server_sequence
    {
        return Err(format!("C06 post-refusal command barrier failed: {barrier:?}").into());
    }
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
    if client.inventory() != Some(&before_inventory)
        || client.actor_vitals() != Some(&before_vitals)
    {
        return Err("C06 client state changed before logout completed".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xc7).await?;
    let fresh_session = fresh.join_snapshot().game_session_id;
    if fresh_session == session {
        return Err("C06 relog reused the refusal session".into());
    }
    let fresh_inventory = fresh
        .inventory()
        .cloned()
        .ok_or("C06 relog inventory absent")?;
    let fresh_vitals = fresh
        .actor_vitals()
        .copied()
        .ok_or("C06 relog vitals absent")?;
    check_durable_state(
        &before_inventory,
        before_vitals,
        &fresh_inventory,
        fresh_vitals,
    )?;

    let session_api = fresh.session_mut();
    let (sequence, raw_body) = session_api.qualification_join_body();
    let file = Path::new(&capture_root).join(format!(
        "c06-{}-{sequence}.snapshot",
        uuid_text(fresh_session.as_bytes())
    ));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)?
        .write_all(raw_body)?;
    let (inventory, vitals) = session_api.qualification_replay_join_inventory_and_vitals(
        fresh_session,
        sequence,
        &std::fs::read(&file)?,
    )?;
    if inventory != fresh_inventory || vitals != fresh_vitals {
        return Err("C06 wire replay differs from the fresh admission".into());
    }
    check_durable_state(&before_inventory, before_vitals, &inventory, vitals)?;
    evidence(&format!(
        "C06_CLIENT_CHECK post-refusal-relog-and-replay session={} sequence={sequence}",
        uuid_text(fresh_session.as_bytes())
    ));

    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C06 final logout: {:?}", logout.result).into());
    }

    refusal
}

fn check_floor_refusal(
    status: CommandStatus,
    disposition: world_object::UseDisposition,
    detail: Option<RefusalDetail>,
) -> TestResult {
    if status == CommandStatus::Accepted
        && disposition == world_object::UseDisposition::TargetIllegal
        && detail == Some(RefusalDetail::FirstGoUpstairs)
    {
        return Ok(());
    }
    Err(format!(
        "C06 expected TargetIllegal/FirstGoUpstairs, got {status:?}/{disposition:?}/{detail:?}"
    )
    .into())
}

fn check_durable_state(
    before: &item_view::CharacterInventory,
    before_vitals: actor_spell::ActorVitals,
    after: &item_view::CharacterInventory,
    after_vitals: actor_spell::ActorVitals,
) -> TestResult {
    fn without_handles(
        mut inventory: item_view::CharacterInventory,
    ) -> item_view::CharacterInventory {
        for entry in inventory
            .main_backpack
            .iter_mut()
            .chain(&mut inventory.entries)
            .chain(
                inventory
                    .equipment
                    .iter_mut()
                    .map(|equipped| &mut equipped.item),
            )
        {
            entry.handle = std::num::NonZeroU64::MIN;
        }
        inventory
    }
    if without_handles(before.clone()) != without_handles(after.clone()) {
        return Err("C06 refusal changed durable inventory".into());
    }
    if before_vitals != after_vitals {
        return Err(format!(
            "C06 refusal changed durable vitals: {before_vitals:?} -> {after_vitals:?}"
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c06_floor_refusal_rejects_generic_and_wrong_direction() {
        let target_illegal = world_object::UseDisposition::TargetIllegal;
        assert!(check_floor_refusal(CommandStatus::Accepted, target_illegal, None).is_err());
        assert!(
            check_floor_refusal(
                CommandStatus::Accepted,
                target_illegal,
                Some(RefusalDetail::FirstGoDownstairs),
            )
            .is_err()
        );
        assert!(
            check_floor_refusal(
                CommandStatus::Accepted,
                target_illegal,
                Some(RefusalDetail::FirstGoUpstairs),
            )
            .is_ok()
        );
    }

    #[test]
    fn c06_relog_rejects_vitals_and_inventory_changes_with_equal_rune_count() -> TestResult {
        let item = item_view::ItemEntry {
            appearance_id: 3174,
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::new(LIGHT_MAGIC_MISSILE_RUNE)
                .ok_or("rune id")?,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        };
        let before = item_view::CharacterInventory {
            entries: vec![item],
            ..Default::default()
        };
        let vitals = actor_spell::ActorVitals {
            health: 150,
            max_health: 150,
            mana: 55,
            max_mana: 55,
            soul: 1,
            harmony: 0,
            serene: false,
        };
        let mut after = before.clone();
        after.entries[0].handle = std::num::NonZeroU64::new(2).ok_or("handle")?;
        check_durable_state(&before, vitals, &after, vitals)?;
        assert!(
            check_durable_state(
                &before,
                vitals,
                &after,
                actor_spell::ActorVitals {
                    health: 149,
                    ..vitals
                }
            )
            .is_err()
        );
        after.equipment.push(item_view::EquippedItem {
            slot: item_view::EquipmentSlot::Head,
            item,
        });
        assert!(check_durable_state(&before, vitals, &after, vitals).is_err());
        Ok(())
    }
}
