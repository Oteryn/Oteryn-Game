//! Capture the inventory and own-vitals payloads exposed after a real rune command.
use super::*;
use oteryn_protocol_oteryn::{actor_spell, item_view};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(capture(clients))).await;
    match result {
        Ok(Ok(())) => evidence("e2e_all scenario=C06.client status=PASS evidence=inventory-and-vitals-payloads"),
        Ok(Err(error)) => evidence(&format!("e2e_all scenario=C06.client status=FAIL evidence={error}")),
        Err(_) => evidence("e2e_all scenario=C06.client status=FAIL evidence=240s-journey-deadline"),
    }
}

fn verify_refusal(status: CommandStatus, payload: &[u8]) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    let disposition = decode_use_result(payload).map_err(|error| format!("C06 refusal decode: {error:?}"))?;
    if status != CommandStatus::Accepted || disposition != UseDisposition::TargetIllegal {
        return Err(format!("C06 owned cross-floor rune expected TargetIllegal, observed {status:?}/{disposition:?}").into());
    }
    Ok(())
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc6).await?;
    let definitions = provisioning::item_index()?;
    let runes = client.session_mut().spell_catalog()?
        .ok_or("C06 spell catalog absent")?
        .entries.into_iter().filter_map(|entry| entry.rune_item)
        .filter_map(|id| provisioning::item_reference(&definitions, id).ok())
        .collect::<std::collections::BTreeSet<_>>();
    let owned = |client: &oteryn_dev_client::DevClientSession| {
        client.inventory()?.entries.iter().find(|entry| runes.contains(&entry.item_definition_ref))
            .map(|entry| entry.handle)
    };
    if owned(&client).is_none() {
        let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
        let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
        let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
        let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
        full_gold::buy(&mut client, world, &metadata.npcs, 3188, 1).await?;
    }
    let rune = owned(&client).ok_or("C06 spell catalog has no owned rune")?;
    let before_inventory = client.inventory().cloned().ok_or("C06 inventory absent")?;
    let before_vitals = client.actor_vitals().copied().ok_or("C06 vitals absent")?;
    let position = client.world_spatial().actor_position;
    let invalid = oteryn_protocol_oteryn::world_object::UseIntent {
        target: oteryn_protocol_oteryn::world_object::UseTarget::Item(rune),
        use_with: None,
        use_with_map_item: None,
        use_with_position: Some(actor_spell::SpellTargetPosition {
            x: position.x,
            y: position.y,
            floor: position.floor.saturating_sub(1),
        }),
    };
    let payload = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&invalid)
        .map_err(|error| format!("C06 refusal encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    verify_refusal(reply.status, &reply.payload)?;
    client.service_liveness(Duration::from_millis(20)).await?;
    if client.inventory() != Some(&before_inventory) || client.actor_vitals() != Some(&before_vitals)
    {
        return Err("C06 invalid target changed client inventory or vitals".into());
    }

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let inventory = item_view::encode_character_inventory(&before_inventory)
            .map_err(|error| format!("C06 inventory encode: {error:?}"))?;
        let vitals = actor_spell::encode_actor_vitals(&before_vitals)
            .map_err(|error| format!("C06 vitals encode: {error:?}"))?;
        let expected = format!(
            "{} {} {} {} {} {}\n",
            before_vitals.health,
            before_vitals.max_health,
            before_vitals.mana,
            before_vitals.max_mana,
            before_vitals.soul,
            before_inventory.entries.len()
        );
        for (name, bytes) in [
            ("c06-inventory.bin", inventory.as_slice()),
            ("c06-vitals.bin", vitals.as_slice()),
            ("c06-expected.txt", expected.as_bytes()),
        ] {
            if let Err(error) = std::fs::write(Path::new(&path).join(name), bytes) {
                evidence(&format!("C06_CAPTURE_WRITE_ERROR file={name} error={error}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, encode_use_result};

    #[test]
    fn rune_probe_requires_a_target_illegal_refusal() -> TestResult {
        let verify = |disposition| {
            let payload = encode_use_result(disposition).map_err(|error| format!("{error:?}"))?;
            verify_refusal(CommandStatus::Accepted, &payload)
        };
        assert!(verify(UseDisposition::Committed).is_err());
        assert!(verify(UseDisposition::NothingToUse).is_err());
        assert!(verify(UseDisposition::TargetIllegal).is_ok());
        Ok(())
    }
}
