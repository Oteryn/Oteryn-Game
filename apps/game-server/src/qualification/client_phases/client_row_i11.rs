//! I11 client-capture scenario, derived from the matrix/audit and the production craft owner:
//! 1. Admit the provisioned character and walk beside a compiled-world oven.
//! 2. Bake fixture-supplied dough using native USE-with and verify the dough debit and bread credit.
//! 3. Send native USE-with using an unsupported carried item -> the same oven; expect a rejected
//!    NothingToUse reply and no inventory delta (the refusal path).
//! 4. Logout, freshly admit the same character, and re-check the durable dough/bread quantities.
use super::*;
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use std::io::Write;

const DOUGH: u32 = 3604;
const BREAD: u32 = 3600;
const ROPE: u32 = 3003;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "client-inventory-payload-captured".to_owned()),
        Ok(Err(error)) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
        Err(_) => ("FAIL", "step-timeout-observed=no-reply".to_owned()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I11.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I11_ACCOUNT").await?;
    let clients = identity.clients(clients);
    character_prep::declare_seeded(
        "I11.client",
        "dough=1,rope=1",
        "the client row exercises baking and inventory replay",
    );
    let mut client = super::full_row_i11::prepare_inputs(&clients, &[DOUGH, ROPE]).await?;

    let (oven, ordinal) = super::full_row_i11::reach_oven(&mut client, world, &bytes).await?;

    let index = provisioning::item_index()?;
    let dough = provisioning::item_reference(&index, DOUGH)?;
    let bread = provisioning::item_reference(&index, BREAD)?;
    let rope = provisioning::item_reference(&index, ROPE)?;
    let before_dough = quantity(&client, dough)?;
    let before_bread = quantity(&client, bread)?;
    if before_dough != 1 {
        return Err(format!("step-2_observed=expected-one-dough-actual-{before_dough}").into());
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("I11 inventory snapshot absent")?
        .clone();
    let source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == dough)
        })
        .ok_or("step-2_observed=no-carried-item-for-real-use")?
        .handle;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(
        &crate::map::view::placement_key(oven, ordinal)
            .ok_or("step-2_observed=oven-placement-unavailable")?
            .to_be_bytes(),
    );
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("step-2_observed=encode-{error:?}"))?;
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await
        .map_err(|error| verdict::context(format!("step=bake phase=USE-result observed={error}"), error))?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("step-2_observed=reply-decode-{error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!(
            "step-2_observed=status-{:?}-disposition-{disposition:?}",
            reply.status
        )
        .into());
    }
    let delta = wait_quantities(
        &mut client,
        dough,
        before_dough - 1,
        bread,
        before_bread + 1,
    )
    .await
    .map_err(|error| verdict::context(format!("step=bake phase=inventory-update observed={error}"), error))?;
    if delta.base_revision != snapshot.revision {
        return Err("step-2_observed=bake-delta-does-not-follow-admission-inventory".into());
    }
    let delta_payload = item_view::encode_character_inventory(&delta.value)
        .map_err(|error| format!("I11 inventory capture encode: {error:?}"))?;
    let refusal_source = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == rope)
        })
        .ok_or("step-3_observed=no-unsupported-carried-item")?
        .handle;
    let mut refusal_placement = world.base().digest().to_vec();
    refusal_placement.extend_from_slice(
        &crate::map::view::placement_key(oven, ordinal)
            .ok_or("step-3_observed=oven-placement-unavailable")?
            .to_be_bytes(),
    );
    let refusal_payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(refusal_source),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement: refusal_placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(|error| format!("step-3_observed=encode-{error:?}"))?;
    let after_bake = client.inventory().cloned();
    let refusal = client
        .session_mut()
        .world_context_use(&refusal_payload)
        .await?;
    let refusal_disposition = decode_use_result(&refusal.payload)
        .map_err(|error| format!("step-3_observed=reply-decode-{error:?}"))?;
    if refusal.status != CommandStatus::Accepted
        || refusal_disposition != UseDisposition::NothingToUse
    {
        return Err(format!(
            "step-3_observed=status-{:?}-disposition-{refusal_disposition:?}",
            refusal.status
        )
        .into());
    }
    client.service_liveness(Duration::from_millis(100)).await?;
    if client.inventory() != after_bake.as_ref() {
        return Err("step-3_observed=unsupported-use-mutated-inventory".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut reloaded = connect_character(&clients, 0xb9, 0).await?;
    if quantity(&reloaded, dough)? != before_dough - 1
        || quantity(&reloaded, bread)? != before_bread + 1
    {
        return Err("step-4_observed=baked-quantities-not-durable".into());
    }
    logout(&mut reloaded).await?;
    capture("i11-result.bin", &reply.payload)?;
    capture("i11-snapshot.bin", &snapshot.payload)?;
    capture("i11-inventory.bin", &delta_payload)?;
    capture(
        "i11-revisions.txt",
        format!(
            "{} {} {} {}",
            snapshot.revision, delta.base_revision, delta.new_revision, delta.server_sequence
        )
        .as_bytes(),
    )?;
    capture(
        "i11-expected.txt",
        visible_inventory(&delta.value).as_bytes(),
    )?;
    Ok(())
}

fn quantity(
    client: &oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("I11 inventory frame absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn wait_quantities(
    client: &mut oteryn_dev_client::DevClientSession,
    first: std::num::NonZeroU32,
    first_expected: u64,
    second: std::num::NonZeroU32,
    second_expected: u64,
) -> TestResult<oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::Inventory(delta) = event
                && client.inventory() == Some(&delta.value)
                && quantity(client, first)? == first_expected
                && quantity(client, second)? == second_expected
            {
                return Ok(delta);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-2_observed=missing-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

fn visible_inventory(inventory: &oteryn_dev_client::CharacterInventory) -> String {
    inventory
        .entries
        .iter()
        .map(|item| format!("{} {}\n", item.item_definition_ref, item.count))
        .collect()
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::Path::new(&path).join(name))
            .and_then(|mut file| file.write_all(bytes))
            .map_err(|error| format!("I11_CAPTURE_WRITE_ERROR file={name} error={error}"))?;
    }
    Ok(())
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-4_observed=logout-refused".into());
    }
    Ok(())
}
