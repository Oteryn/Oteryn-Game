//! Captures the real I03 LOOK command-result payloads consumed by the native client.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::world_interaction::{
    Action, Command, Selection, Target, look_payload, send_observing_result,
};
use oteryn_protocol_oteryn::item_look as wire;
use std::num::NonZeroU64;

async fn look(
    client: &mut DevClientSession,
    target: Target,
) -> TestResult<(
    oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
    wire::ItemLookResult,
)> {
    let selection = wire::ItemLookSelection { extended: true };
    let request = look_payload(&target)?;
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
    Ok((reply, decoded))
}

fn capture(
    directory: &Path,
    name: &str,
    session: &str,
    reply: &oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse,
) -> TestResult {
    use std::io::Write;
    let envelope = serde_json::to_vec(&serde_json::json!({
        "command_id": reply.command_id,
        "game_session_id": session,
        "payload": reply.payload,
        "server_sequence": reply.server_sequence,
        "status": reply.status as u32,
    }))?;
    std::fs::File::create_new(directory.join(name))?.write_all(&envelope)?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let directory = std::fs::canonicalize(required("E2E_ALL_CAPTURE_DIR")?)?;
    let mut client = connect_world(clients, 0x03).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let entities = client
        .world_entities()
        .ok_or("I03.client own actor view missing")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .ok_or("I03.client own actor missing")?;
    let (own, own_position) = (own.entity, own.position);
    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("I03.client starter backpack missing")?;

    let absent = NonZeroU64::new(u64::MAX).ok_or("I03.client invalid absent handle")?;
    let inventory_before = client.inventory().ok_or("missing I03 inventory")?.clone();
    let position_before = client.world_spatial().actor_position;
    if inventory_before
        .main_backpack
        .iter()
        .chain(&inventory_before.entries)
        .any(|entry| entry.handle == absent)
        || inventory_before
            .equipment
            .iter()
            .any(|entry| entry.item.handle == absent)
    {
        return Err("I03 negative handle is already present".into());
    }
    let assert_unchanged = |client: &DevClientSession, step: &str| -> TestResult {
        if client.inventory() != Some(&inventory_before)
            || client.world_spatial().actor_position != position_before
        {
            return Err(format!("I03.client {step} changed inventory or position").into());
        }
        Ok(())
    };
    let own_command = Command {
        action: Action::Look,
        target: Target::Creature(own),
        selection: Some(Selection {
            position: own_position,
            binding: None,
        }),
    };
    let mut own_result = None;
    let own_text = send_observing_result(client.session_mut(), &own_command, false, |reply| {
        own_result = Some(reply.clone());
    })
    .await?;
    let own_reply = own_result.ok_or("I03.client self LOOK did not dispatch")?;
    assert_unchanged(&client, "self LOOK")?;
    let mut item_result = None;
    let item_command = Command {
        action: Action::Look,
        target: Target::Item(backpack.handle),
        selection: None,
    };
    let item_text = send_observing_result(client.session_mut(), &item_command, false, |reply| {
        item_result = Some(reply.clone());
    })
    .await?;
    let item_reply = item_result.ok_or("I03.client backpack LOOK did not dispatch")?;
    assert_unchanged(&client, "backpack LOOK")?;
    let stale_command = Command {
        target: Target::Item(absent),
        ..item_command
    };
    let mut dispatched = false;
    let stale = send_observing_result(client.session_mut(), &stale_command, false, |_| {
        dispatched = true;
    })
    .await?;
    if dispatched || stale != "Target changed" {
        return Err(format!(
            "I03.client native stale selection: {stale:?} dispatched={dispatched}"
        )
        .into());
    }
    assert_unchanged(&client, "native stale LOOK")?;
    let refusal_reply = look(&mut client, Target::Item(absent)).await?;
    if own_reply.status != CommandStatus::Accepted
        || item_reply.status != CommandStatus::Accepted
        || refusal_reply.0.status != CommandStatus::Accepted
    {
        return Err("I03.client LOOK command was not accepted".into());
    }
    if refusal_reply.1 != wire::ItemLookResult::Refused(wire::ItemLookRefusal::NotAvailable) {
        return Err("I03.client absent item did not return NotAvailable".into());
    }

    assert_unchanged(&client, "absent-item LOOK")?;
    let expected_own = "You see yourself (Level 1, sorcerer).";
    let expected_item = "You see backpack (Vol:20).\nImbuement slots: 1.\nIt weighs 18.00 oz.";
    if own_text != expected_own || item_text != expected_item {
        return Err(format!(
            "I03.client unexpected LOOK text: self={own_text:?} item={item_text:?}"
        )
        .into());
    }
    let (logout, detail, _, _) = logout_once(&mut client).await?;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("I03.client logout refused: {detail}").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0x03).await?;
    if uuid_text(fresh.join_snapshot().game_session_id.as_bytes()) == session {
        return Err("I03.client relog reused the LOOK session".into());
    }
    assert_unchanged(&fresh, "post-relog LOOK state")?;
    let (logout, detail, _, _) = logout_once(&mut fresh).await?;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("I03.client post-relog logout refused: {detail}").into());
    }
    capture(&directory, "i03-self.json", &session, &own_reply)?;
    capture(&directory, "i03-item.json", &session, &item_reply)?;
    capture(&directory, "i03-refusal.json", &session, &refusal_reply.0)?;
    use std::io::Write;
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !head.status.success() {
        return Err("I03.client capture commit unavailable".into());
    }
    let head = std::str::from_utf8(&head.stdout)?.trim();
    std::fs::File::create_new(directory.join("i03-run.txt"))?
        .write_all(format!("{session}\n{head}\n{}\n", directory.display()).as_bytes())?;
    evidence(&format!("I03_CAPTURE_SESSION {session}"));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(60), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence("e2e_capture scenario=I03.client status=CAPTURED"),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=I03.client status=FAIL evidence={}",
            error.to_string().replace([' ', '\n'], "_")
        )),
        Err(_) => evidence("e2e_all scenario=I03.client status=FAIL evidence=timeout"),
    }
}
