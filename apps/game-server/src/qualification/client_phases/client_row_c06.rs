//! Capture the inventory and own-vitals payloads exposed after a real rune command.
use super::*;
use oteryn_protocol_oteryn::{actor_spell, item_view};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(capture(clients)).await;
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
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let mut client = connect_world(clients, 0xc6).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let before_inventory = client.inventory().cloned().ok_or("C06 inventory absent")?;
        let before_vitals = client.actor_vitals().copied().ok_or("C06 vitals absent")?;

        // Drive the real rune-use route with an impossible cross-floor target. This is the minimal
        // action that produces a server-observed refusal while proving charges and vitals unchanged.
        let position = client.world_spatial().actor_position;
        let rune = client
            .session_mut()
            .spell_catalog()?
            .ok_or("C06 spell catalog absent")?
            .entries
            .into_iter()
            .find_map(|entry| entry.rune_item)
            .and_then(std::num::NonZeroU32::new)
            .ok_or("C06 spell catalog has no rune")?;
        let invalid = oteryn_protocol_oteryn::world_object::UseIntent {
            target: oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(rune),
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
        let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
            .map_err(|error| format!("C06 refusal decode: {error:?}"))?;
        if reply.status != CommandStatus::Accepted
            || disposition != oteryn_protocol_oteryn::world_object::UseDisposition::TargetIllegal
        {
            return Err(format!(
                "C06 expected cross-floor TargetIllegal, observed {:?}/{disposition:?}",
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

        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let inventory =
                item_view::encode_character_inventory_with_equip_drop(&before_inventory, true)
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
                std::fs::write(Path::new(&path).join(name), bytes)?;
            }
        }
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}
