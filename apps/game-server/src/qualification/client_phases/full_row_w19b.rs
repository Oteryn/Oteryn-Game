//! W19b carries out Crystal’s ice flower seed USE on a carried empty pot.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::world_object as use_wire;
use std::collections::BTreeMap;

const ICE_FLOWER_SEEDS: u32 = 13844;
const EMPTY_POT: u32 = 306;
const PROBES: [(&str, u32); 2] = [
    ("ice flower seeds", ICE_FLOWER_SEEDS),
    ("empty pot", EMPTY_POT),
];
const PLANTED_POT: u32 = 14031;
const NEEDED: &str = "needed:ice-flower-seeds-13844-in-inventory;only-source=thawing-day-ice-flower-harvest-25pct-roll(crates/gameplay/items/src/event_planting.rs:200)";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "planted-pot-and-seed-cost-verified-after-relog".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W19b.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut result = BTreeMap::new();
    for entry in &client
        .inventory()
        .ok_or("step=admit observed=no-inventory")?
        .entries
    {
        *result.entry(entry.item_definition_ref.get()).or_default() += u64::from(entry.count.get());
    }
    Ok(result)
}

async fn use_by_definition(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<(CommandStatus, use_wire::UseDisposition)> {
    let payload = use_wire::encode_item_use_intent(&use_wire::UseIntent {
        target: use_wire::UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("step=use observed=encode-{error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (body, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("step=refused-use observed=refusal-detail-{error:?}"))?;
    let disposition = use_wire::decode_use_result(body)
        .map_err(|error| format!("step=refused-use observed=decode-{error:?}"))?;
    Ok((reply.status, disposition))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={:?}", reply.result).into());
    }
    Ok(())
}

fn item_handle(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<std::num::NonZeroU64> {
    let inventory = client
        .inventory()
        .ok_or("step=prepare observed=no-inventory")?;
    let mut entries = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference);
    match (entries.next(), entries.next()) {
        (Some(entry), None) if entry.count.get() == 1 => Ok(entry.handle),
        _ => Err(format!("step=prepare expected=one-carried-item reference={reference}").into()),
    }
}

async fn verify_decay(clients: &SeamClients<'_>, planting_session: &[u8; 16]) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let stages: Vec<(i32, i64, i64, String, String)> = sqlx::query_as(
        "SELECT s.source_stage, s.duration_millis, s.expires_at_unix_ms - r.occurred_at,
                s.definition_production_key, s.target_production_key
           FROM game_item_consume_transform_records r
           JOIN game_item_instances i ON i.minted_transaction_id = r.transaction_id
           JOIN game_spell_field_temporal_schedules s
             ON s.item_instance_id = i.item_instance_id
            AND s.event_source_transaction_id = r.transaction_id
          WHERE r.game_session_id = encode($1,'hex')::uuid
            AND r.character_id = encode($2,'hex')::uuid
            AND r.recipe = 'oteryn:planting.ice-flower'
          ORDER BY s.source_stage LIMIT 3",
    )
    .bind(planting_session.as_slice())
    .bind(clients.characters[1].as_slice())
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    let expected = [
        (
            1,
            72_000_000,
            72_000_000,
            "oteryn:item.tibia.i14031".into(),
            "oteryn:item.tibia.i14032".into(),
        ),
        (
            2,
            72_000_000,
            144_000_000,
            "oteryn:item.tibia.i14032".into(),
            "oteryn:item.tibia.i306".into(),
        ),
    ];
    if stages != expected {
        return Err(format!("step=decay observed={stages:?} expected={expected:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W19B_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 1)?;
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let mut client = connect_world(clients, 0xd3).await?;
    let before = quantities(&client)?;
    let index = provisioning::item_index()?;
    let seed = provisioning::item_reference(&index, ICE_FLOWER_SEEDS)?;
    let pot = provisioning::item_reference(&index, EMPTY_POT)?;
    let planted = provisioning::item_reference(&index, PLANTED_POT)?;
    for (label, id) in PROBES {
        let reference = provisioning::item_reference(&index, id)?;
        if before.contains_key(&reference.get()) {
            return Err(format!("step=refused-use observed=unexpected-carried-{label}").into());
        }
        let (status, disposition) = use_by_definition(&mut client, reference).await?;
        if (status, disposition)
            != (
                CommandStatus::Accepted,
                use_wire::UseDisposition::NothingToUse,
            )
        {
            return Err(format!("step=refused-use-{label} expected=Accepted/NothingToUse observed={status:?}/{disposition:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        if quantities(&client)? != before {
            return Err(format!("step=refused-use-{label} observed=inventory-changed").into());
        }
    }

    character_prep::grant_item(&client, ICE_FLOWER_SEEDS, 1)
        .await
        .map_err(|error| format!("{NEEDED} {error}"))?;
    character_prep::grant_item(&client, EMPTY_POT, 1).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_world(clients, 0xd3).await?;
    let mut prepared = before.clone();
    prepared.insert(seed.get(), 1);
    prepared.insert(pot.get(), 1);
    if quantities(&client)? != prepared || before.contains_key(&planted.get()) {
        return Err("step=prepare expected=only-one-seed-and-one-empty-pot-added".into());
    }
    let planting_session = *client.join_snapshot().game_session_id.as_bytes();
    let payload = use_wire::encode_item_use_intent(&use_wire::UseIntent {
        target: use_wire::UseTarget::Item(item_handle(&client, seed)?),
        use_with: None,
        use_with_map_item: Some(use_wire::MapUseTarget::Item(item_handle(&client, pot)?)),
        use_with_position: None,
    })
    .map_err(|error| format!("step=plant observed=encode-{error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (body, _) = oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
        .map_err(|error| format!("step=plant observed=refusal-detail-{error:?}"))?;
    let disposition = use_wire::decode_use_result(body)
        .map_err(|error| format!("step=plant observed=decode-{error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != use_wire::UseDisposition::Committed
    {
        return Err(format!("step=plant observed={:?}/{disposition:?}", reply.status).into());
    }
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_world(clients, 0xd3).await?;
    let mut expected = before;
    expected.insert(planted.get(), 1);
    let actual = quantities(&client)?;
    if actual != expected {
        return Err(format!("step=relog expected={expected:?} observed={actual:?}").into());
    }
    logout(&mut client).await?;
    verify_decay(clients, &planting_session).await?;
    evidence(
        "W19B_CHECK empty_pot_consumed=1 seed_consumed=1 planted_pot=14031 relog=true decay_stages=2",
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needed_precondition_is_one_report_token() {
        assert!(NEEDED.starts_with("needed:"));
        assert!(!compact(NEEDED).contains(char::is_whitespace));
    }

    #[test]
    fn compact_bounds_and_joins() {
        assert_eq!(compact("a  b\nc"), "a-b-c");
        assert_eq!(compact(" "), "unknown");
        assert_eq!(compact(&"x".repeat(900)).len(), 400);
    }
}
