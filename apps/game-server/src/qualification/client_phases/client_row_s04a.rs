//! S04a client replay capture: the real VIP list replies for Add, the edited/grouped Query and
//! the duplicate-Add refusal. The consumer is `request`/`apply_result`/`panel_text` in
//! `crates/client-ui/src/features/0043-vip`. The VIP presence deltas of the full row are typed
//! session events whose raw bytes the session does not retain; they are not captured here.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::vip::vip as wire;
use oteryn_protocol_oteryn::CommandStatus;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("missing S04a capture directory")?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&root).join(name))?
        .write_all(bytes)?;
    Ok(())
}

fn request(operation: wire::Operation, number: u64, text: &str) -> wire::Request {
    wire::Request {
        operation,
        number,
        text: text.to_owned(),
        icon: 0,
        notify: false,
        groups: Vec::new(),
    }
}

/// The raw VIP reply payload and its decoded result.
async fn raw(
    client: &mut DevClientSession,
    request: &wire::Request,
) -> TestResult<(Vec<u8>, wire::ResultView, u64)> {
    let (kind, payload) =
        wire::encode_request(request).map_err(|e| format!("VIP encode: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_VIP_V1, kind, &payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("VIP command status {:?}", reply.status).into());
    }
    let decoded =
        wire::decode_result(&reply.payload).map_err(|e| format!("VIP reply undecodable: {e:?}"))?;
    Ok((reply.payload, decoded, reply.server_sequence))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_S04A_ACCOUNT").await?;
    let (accounts, characters) = dedicated.replace_slot(clients, 0)?;
    let clients = &SeamClients { accounts: &accounts, characters: &characters, ..*clients };
    let mut watcher = connect_character(clients, 0x4b, 0).await?;
    character_prep::grant_premium(&watcher).await?;
    evidence("S04A_FIXTURE account=q_s04a input=premium");
    let mut target = connect_character(clients, 0x4c, 1).await?;
    let name = super::full_row_s04a::peer_name(&mut watcher, &mut target).await?;
    evidence("S04A_CLIENT_DIAG step=discover-target observed=served-player-name");

    let query = request(wire::Operation::Query, 0, "");
    let (_, initial, _) = raw(&mut watcher, &query).await?;
    let initial_view: serde_json::Value = serde_json::from_str(&initial.view)?;
    if initial.code != 1 || initial_view["entries"] != serde_json::json!([])
        || initial_view["groups"] != serde_json::json!([])
        || initial_view["next_entry"].as_u64() != Some(0)
    {
        return Err("S04a client fixture needs an empty VIP list".into());
    }
    let (added_bytes, added, added_sequence) = raw(&mut watcher, &request(wire::Operation::Add, 0, &name)).await?;
    if added.code != 1 || added.number == 0 {
        return Err(format!("Add reply mismatch: {added:?}").into());
    }
    capture("s04a-added.bin", &added_bytes)?;
    let entry = added.number;
    let (_, after_add, _) = raw(&mut watcher, &query).await?;
    let added_view: serde_json::Value = serde_json::from_str(&after_add.view)?;
    let new_entry = &added_view["entries"][0];
    if after_add.code != 1 || after_add.revision != added.revision
        || new_entry["entry"].as_u64() != Some(entry)
        || new_entry["name"].as_str() != Some(&name)
        || new_entry["description"].as_str() != Some("")
        || new_entry["icon"].as_u64() != Some(0)
        || new_entry["notify"].as_bool() != Some(false)
        || new_entry["groups"] != serde_json::json!([])
        || added_view["groups"] != initial_view["groups"]
    {
        return Err("S04a client Add did not create the expected unedited entry".into());
    }
    let (_, group, _) = raw(
        &mut watcher,
        &request(wire::Operation::GroupAdd, 0, "Raid Friends"),
    )
    .await?;
    if group.code != 1 || group.number == 0 {
        return Err(format!("GroupAdd reply mismatch: {group:?}").into());
    }
    let (_, after_group, _) = raw(&mut watcher, &query).await?;
    let group_view: serde_json::Value = serde_json::from_str(&after_group.view)?;
    if after_group.code != 1 || after_group.revision != group.revision
        || group_view["entries"] != added_view["entries"]
        || group_view["groups"] != serde_json::json!([{"group": group.number, "name": "Raid Friends"}])
    {
        return Err("S04a client GroupAdd did not create the expected group".into());
    }
    let group_number = u8::try_from(group.number)?;
    let (_, edited, _) = raw(
        &mut watcher,
        &wire::Request {
            operation: wire::Operation::Edit,
            number: entry,
            text: "node-boot friend".into(),
            icon: 7,
            notify: true,
            groups: vec![group_number],
        },
    )
    .await?;
    let (_, after_edit, _) = raw(&mut watcher, &query).await?;
    let edit_view: serde_json::Value = serde_json::from_str(&after_edit.view)?;
    let edited_entry = &edit_view["entries"][0];
    if after_edit.code != 1 || after_edit.revision != edited.revision
        || edited_entry["entry"].as_u64() != Some(entry)
        || edited_entry["name"].as_str() != Some(&name)
        || edited_entry["description"].as_str() != Some("node-boot friend")
        || edited_entry["icon"].as_u64() != Some(7)
        || edited_entry["notify"].as_bool() != Some(true)
        || edited_entry["groups"] != serde_json::json!([group.number])
        || edit_view["groups"] != group_view["groups"]
    {
        return Err("S04a client Edit did not persist the expected metadata".into());
    }
    let (_, renamed, _) = raw(
        &mut watcher,
        &request(wire::Operation::GroupRename, group.number, "Raid Team"),
    )
    .await?;
    if edited.code != 1 || renamed.code != 1 {
        return Err(format!("Edit/GroupRename mismatch: {edited:?} {renamed:?}").into());
    }
    let (listed_bytes, listed, listed_sequence) = raw(&mut watcher, &request(wire::Operation::Query, 0, "")).await?;
    let view: serde_json::Value = serde_json::from_str(&listed.view)?;
    let entry_ok = view["entries"].as_array().is_some_and(|entries| {
        entries.iter().any(|item| {
            item["entry"].as_u64() == Some(entry)
                && item["name"].as_str() == Some(&name)
                && item["description"].as_str() == Some("node-boot friend")
                && item["icon"].as_u64() == Some(7)
                && item["notify"].as_bool() == Some(true)
        })
    });
    let group_ok = view["groups"].as_array().is_some_and(|groups| {
        groups.iter().any(|item| {
            item["group"].as_u64() == Some(group.number)
                && item["name"].as_str() == Some("Raid Team")
        })
    });
    if listed.code != 1 || listed.revision != renamed.revision || !entry_ok || !group_ok
        || view["entries"] != edit_view["entries"] {
        return Err(format!("Query view mismatch: {view}").into());
    }
    capture("s04a-listed.bin", &listed_bytes)?;

    let (duplicate_bytes, duplicate, duplicate_sequence) =
        raw(&mut watcher, &request(wire::Operation::Add, 0, &name)).await?;
    if duplicate.code != 3 || listed.revision.checked_add(1) != Some(duplicate.revision) {
        return Err(format!("duplicate Add mismatch: {duplicate:?}").into());
    }
    let (_, after_refusal, _) = raw(&mut watcher, &request(wire::Operation::Query, 0, "")).await?;
    if after_refusal.code != 1
        || after_refusal.revision != duplicate.revision
        || serde_json::from_str::<serde_json::Value>(&after_refusal.view)? != view
    {
        return Err("duplicate Add changed the VIP list".into());
    }
    capture("s04a-duplicate.bin", &duplicate_bytes)?;
    if !(added_sequence < listed_sequence && listed_sequence < duplicate_sequence) {
        return Err("VIP capture sequences did not advance".into());
    }
    let session = uuid_text(watcher.join_snapshot().game_session_id.as_bytes());
    let expected = format!(
        "name={name}\nentry={entry}\ngroup={}\nrevision={}\nadded_revision={}\nduplicate_code={}\nsession={session}\nadded_sequence={added_sequence}\nlisted_sequence={listed_sequence}\nduplicate_sequence={duplicate_sequence}\n",
        group.number, listed.revision, added.revision, duplicate.code
    );
    capture("s04a-expected.txt", expected.as_bytes())?;
    let old_session = watcher.join_snapshot().game_session_id;
    if watcher.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("S04a watcher logout before relog refused".into());
    }
    drop(watcher);
    let mut watcher = connect_character(clients, 0x4d, 0).await?;
    let (_, persisted, _) = raw(&mut watcher, &query).await?;
    if watcher.join_snapshot().game_session_id == old_session
        || persisted.code != 1 || persisted.revision != duplicate.revision
        || serde_json::from_str::<serde_json::Value>(&persisted.view)? != view
    {
        return Err("S04a client relog did not recover the exact edited list".into());
    }
    for client in [&mut watcher, &mut target] {
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("S04a client logout refused".into());
        }
    }
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=S04a.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S04a.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=S04a.client status=FAIL evidence=timeout"),
    }
}
