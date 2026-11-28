//! S04a client replay capture: the real VIP list replies for Add, the edited/grouped Query and
//! the duplicate-Add refusal. The consumer is `request`/`apply_result`/`panel_text` in
//! `crates/client-ui/src/features/0043-vip`. The VIP presence deltas of the full row are typed
//! session events whose raw bytes the session does not retain; they are not captured here.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::vip::vip as wire;
use oteryn_protocol_oteryn::CommandStatus;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("S04A_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
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
) -> TestResult<(Vec<u8>, wire::ResultView)> {
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
    Ok((reply.payload, decoded))
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

    let (added_bytes, added) = raw(&mut watcher, &request(wire::Operation::Add, 0, &name)).await?;
    if added.code != 1 || added.number == 0 {
        return Err(format!("Add reply mismatch: {added:?}").into());
    }
    capture("s04a-added.bin", &added_bytes);
    let entry = added.number;
    let (_, group) = raw(
        &mut watcher,
        &request(wire::Operation::GroupAdd, 0, "Raid Friends"),
    )
    .await?;
    if group.code != 1 || group.number == 0 {
        return Err(format!("GroupAdd reply mismatch: {group:?}").into());
    }
    let group_number = u8::try_from(group.number)?;
    let (_, edited) = raw(
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
    let (_, renamed) = raw(
        &mut watcher,
        &request(wire::Operation::GroupRename, group.number, "Raid Team"),
    )
    .await?;
    if edited.code != 1 || renamed.code != 1 {
        return Err(format!("Edit/GroupRename mismatch: {edited:?} {renamed:?}").into());
    }
    let (listed_bytes, listed) = raw(&mut watcher, &request(wire::Operation::Query, 0, "")).await?;
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
    if listed.code != 1 || !entry_ok || !group_ok {
        return Err(format!("Query view mismatch: {view}").into());
    }
    capture("s04a-listed.bin", &listed_bytes);

    let (duplicate_bytes, duplicate) =
        raw(&mut watcher, &request(wire::Operation::Add, 0, &name)).await?;
    if duplicate.code != 3 || duplicate.revision != listed.revision {
        return Err(format!("duplicate Add mismatch: {duplicate:?}").into());
    }
    capture("s04a-duplicate.bin", &duplicate_bytes);
    let expected = format!(
        "name={name}\nentry={entry}\ngroup={}\nrevision={}\nadded_revision={}\nduplicate_code={}\n",
        group.number, listed.revision, added.revision, duplicate.code
    );
    capture("s04a-expected.txt", expected.as_bytes());
    let _ = watcher.session_mut().logout().await?;
    let _ = target.session_mut().logout().await?;
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
