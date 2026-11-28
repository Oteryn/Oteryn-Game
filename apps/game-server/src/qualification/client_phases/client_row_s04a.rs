//! S04a client replay capture: the real VIP list replies for Add, the edited/grouped Query and
//! the duplicate-Add refusal. The consumer is `request`/`apply_result`/`panel_text` in
//! `crates/client-ui/src/features/0043-vip`. The VIP presence deltas of the full row are typed
//! session events whose raw bytes the session does not retain; they are not captured here.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::vip::vip as wire;
use oteryn_protocol_oteryn::CommandStatus;
use sha2::{Digest, Sha256};

fn payload_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

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

fn only_entry(view: &serde_json::Value) -> TestResult<&serde_json::Value> {
    match view["entries"].as_array().map(Vec::as_slice) {
        Some([entry]) if view["next_entry"].as_u64() == Some(0) => Ok(entry),
        _ => Err("S04a client readback requires exactly one VIP entry".into()),
    }
}

fn custom_group_id(id: u64) -> bool {
    (4..=8).contains(&id)
}

fn group_receipt_matches(actual: u64, expected: u64) -> bool {
    custom_group_id(actual) && actual == expected
}

fn require_next_revision(step: &str, before: u64, after: u64) -> TestResult {
    if before.checked_add(1) != Some(after) {
        return Err(format!("S04a {step} revision did not advance by one: {before} -> {after}").into());
    }
    Ok(())
}

fn group_add_view_matches(view: &serde_json::Value, before: &serde_json::Value, group: u64) -> bool {
    view["next_entry"].as_u64() == Some(0)
        && view["entries"] == before["entries"]
        && view["groups"] == serde_json::json!([{"group": group, "name": "Raid Friends"}])
}

fn renamed_group_matches(view: &serde_json::Value, group: u64) -> bool {
    view["groups"] == serde_json::json!([{"group": group, "name": "Raid Team"}])
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
    require_next_revision("Add", initial.revision, added.revision)?;
    capture("s04a-added.bin", &added_bytes)?;
    let entry = added.number;
    let (after_add_bytes, after_add, after_add_sequence) = raw(&mut watcher, &query).await?;
    let added_view: serde_json::Value = serde_json::from_str(&after_add.view)?;
    let new_entry = only_entry(&added_view)?;
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
    let (group_bytes, group, group_sequence) = raw(
        &mut watcher,
        &request(wire::Operation::GroupAdd, 0, "Raid Friends"),
    )
    .await?;
    if group.code != 1 || !custom_group_id(group.number) {
        return Err(format!("GroupAdd reply mismatch: {group:?}").into());
    }
    require_next_revision("GroupAdd", after_add.revision, group.revision)?;
    let (after_group_bytes, after_group, after_group_sequence) = raw(&mut watcher, &query).await?;
    let group_view: serde_json::Value = serde_json::from_str(&after_group.view)?;
    if after_group.code != 1 || after_group.revision != group.revision
        || !group_add_view_matches(&group_view, &added_view, group.number)
    {
        return Err("S04a client GroupAdd did not create the expected group".into());
    }
    let group_number = u8::try_from(group.number)?;
    let (edited_bytes, edited, edited_sequence) = raw(
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
    require_next_revision("Edit", after_group.revision, edited.revision)?;
    let (after_edit_bytes, after_edit, after_edit_sequence) = raw(&mut watcher, &query).await?;
    let edit_view: serde_json::Value = serde_json::from_str(&after_edit.view)?;
    let edited_entry = only_entry(&edit_view)?;
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
    let (renamed_bytes, renamed, renamed_sequence) = raw(
        &mut watcher,
        &request(wire::Operation::GroupRename, group.number, "Raid Team"),
    )
    .await?;
    if edited.code != 1 || renamed.code != 1 || !group_receipt_matches(renamed.number, group.number) {
        return Err(format!("Edit/GroupRename mismatch: {edited:?} {renamed:?}").into());
    }
    require_next_revision("GroupRename", after_edit.revision, renamed.revision)?;
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
    if listed.code != 1 || listed.revision != renamed.revision || !entry_ok || !renamed_group_matches(&view, group.number)
        || view["next_entry"].as_u64() != Some(0)
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
    let mut expected = format!(
        "name={name}\nentry={entry}\ngroup={}\nrevision={}\nadded_revision={}\nduplicate_code={}\nsession={session}\nadded_sequence={added_sequence}\nlisted_sequence={listed_sequence}\nduplicate_sequence={duplicate_sequence}\n",
        group.number, listed.revision, added.revision, duplicate.code
    );
    use std::fmt::Write;
    let refresh_captures = [
        ("after-add", &after_add_bytes, after_add_sequence),
        ("group", &group_bytes, group_sequence),
        ("after-group", &after_group_bytes, after_group_sequence),
        ("edited", &edited_bytes, edited_sequence),
        ("after-edit", &after_edit_bytes, after_edit_sequence),
        ("renamed", &renamed_bytes, renamed_sequence),
    ];
    let mut prior = added_sequence;
    for (label, bytes, sequence) in refresh_captures {
        if sequence <= prior || sequence >= listed_sequence {
            return Err("VIP mutation and refresh capture sequences are out of order".into());
        }
        capture(&format!("s04a-{label}.bin"), bytes)?;
        writeln!(expected, "{label}_sequence={sequence}")?;
        writeln!(expected, "{label}_sha256={}", payload_digest(bytes))?;
        prior = sequence;
    }
    for (label, bytes) in [
        ("added", &added_bytes),
        ("listed", &listed_bytes),
        ("duplicate", &duplicate_bytes),
    ] {
        writeln!(expected, "{label}_sha256={}", payload_digest(bytes))?;
    }
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

#[test]
fn s04a_client_readbacks_reject_extra_entries_and_groups() {
    let mut view = serde_json::json!({
        "entries": [{"entry": 1}],
        "next_entry": 0,
        "groups": [{"group": 4, "name": "Raid Team"}]
    });
    assert!(only_entry(&view).is_ok());
    assert!(renamed_group_matches(&view, 4));
    view["entries"] = serde_json::json!([{"entry": 1}, {"entry": 2}]);
    assert!(only_entry(&view).is_err());
    view["groups"] = serde_json::json!([
        {"group": 4, "name": "Raid Team"}, {"group": 5, "name": "Extra"}
    ]);
    assert!(!renamed_group_matches(&view, 4));
}

#[test]
fn s04a_mutations_reject_stale_and_skipped_revisions() {
    for (step, before) in [("Add", 0), ("GroupAdd", 1), ("Edit", 2), ("GroupRename", 3)] {
        assert!(require_next_revision(step, before, before + 1).is_ok());
        assert!(require_next_revision(step, before, before).is_err());
        assert!(require_next_revision(step, before, before + 2).is_err());
    }
    assert!(require_next_revision("GroupRename", u64::MAX, 0).is_err());
}

#[test]
fn s04a_group_add_rejects_invalid_continuation_cursor() {
    let before = serde_json::json!({"entries": [{"entry": 1}]});
    let mut view = serde_json::json!({
        "entries": [{"entry": 1}],
        "groups": [{"group": 4, "name": "Raid Friends"}],
        "next_entry": 0
    });
    assert!(group_add_view_matches(&view, &before, 4));
    for invalid_cursor in [serde_json::json!(1), serde_json::json!("0"), serde_json::Value::Null] {
        view["next_entry"] = invalid_cursor;
        assert!(!group_add_view_matches(&view, &before, 4));
    }
}

#[test]
fn s04a_group_receipts_reject_reserved_out_of_range_and_wrong_group_ids() {
    assert!(group_receipt_matches(4, 4));
    assert!(group_receipt_matches(8, 8));
    for invalid in [0, 3, 9, 255] {
        assert!(!custom_group_id(invalid));
        assert!(!group_receipt_matches(invalid, invalid));
    }
    assert!(!group_receipt_matches(5, 4));
}
