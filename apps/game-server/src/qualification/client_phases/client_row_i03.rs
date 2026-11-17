//! Captures the real I03 LOOK command-result payloads consumed by the native client.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_look as wire;
use std::num::NonZeroU64;

async fn look(
    client: &mut DevClientSession,
    target: wire::ItemLookTarget,
) -> TestResult<(CommandStatus, Vec<u8>, wire::ItemLookResult)> {
    let selection = wire::ItemLookSelection { extended: true };
    let request = wire::encode_item_look_intent_for(wire::ItemLookIntent { target }, selection)
        .map_err(|error| format!("I03.client LOOK encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::ITEM_LOOK_CAPABILITY_ID.ok_or("I03.client LOOK capability unallocated")?,
            wire::ITEM_LOOK_COMMAND_ID.ok_or("I03.client LOOK command unallocated")?,
            &request,
        )
        .await?;
    let decoded = wire::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("I03.client LOOK decode: {error:?}"))?;
    Ok((reply.status, reply.payload, decoded))
}

fn capture(name: &str, bytes: &[u8]) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&directory).join(name), bytes) {
        evidence(&format!(
            "e2e_all scenario=I03.client status=CAPTURE-WRITE-ERROR evidence={name}:{error}"
        ));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x03).await?;
    let entities = client.world_entities().ok_or("I03.client own actor view missing")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("I03.client own actor missing")?
        .entity;
    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("I03.client starter backpack missing")?;

    let own_reply = look(&mut client, wire::ItemLookTarget::Creature(own)).await?;
    let item_reply = look(
        &mut client,
        wire::ItemLookTarget::Item(backpack.handle),
    )
    .await?;
    let absent = NonZeroU64::new(u64::MAX).ok_or("I03.client invalid absent handle")?;
    let refusal_reply = look(&mut client, wire::ItemLookTarget::Item(absent)).await?;
    if own_reply.0 != CommandStatus::Accepted
        || item_reply.0 != CommandStatus::Accepted
        || refusal_reply.0 != CommandStatus::Accepted
    {
        return Err("I03.client LOOK command was not accepted".into());
    }
    let (wire::ItemLookResult::Text(own_text), wire::ItemLookResult::Text(item_text)) =
        (&own_reply.2, &item_reply.2)
    else {
        return Err("I03.client positive LOOK did not return text".into());
    };
    if refusal_reply.2
        != wire::ItemLookResult::Refused(wire::ItemLookRefusal::NotAvailable)
    {
        return Err("I03.client absent item did not return NotAvailable".into());
    }

    capture("i03-self.bin", &own_reply.1);
    capture("i03-item.bin", &item_reply.1);
    capture("i03-refusal.bin", &refusal_reply.1);
    capture(
        "i03-expected.txt",
        format!("self={own_text:?}\nitem={item_text:?}\nrefusal=Look refused\n").as_bytes(),
    );
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence("e2e_all scenario=I03.client status=PASS"),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=I03.client status=FAIL evidence={}",
            error.to_string().replace([' ', '\n'], "_")
        )),
        Err(_) => evidence("e2e_all scenario=I03.client status=FAIL evidence=timeout"),
    }
}
