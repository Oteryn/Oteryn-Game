//! S06 client replay capture: the outfit selection and the unearned-addon refusal. The native
//! appearance catalogue is read before and after an APPEARANCE_CHANGE that selects an earned outfit
//! with new colours; the admission snapshot and the reconstructed live appearance snapshot (`s06-before.bin`, `s06-after.bin`) and the
//! raw command results (`s06-commit.bin`, `s06-refusal.bin`) are written byte for byte, and
//! `s06-expected.txt` holds the selection, the committed revision and the content revision.
//! The full row drives the separate Hunter loot and War Bear taming journey.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::appearance;

struct Observed {
    before: Vec<u8>,
    after: Vec<u8>,
    commit: Vec<u8>,
    refusal: Vec<u8>,
    request: Vec<u8>,
    refused_request: Vec<u8>,
    expected: String,
}

fn raw_snapshot(client: &mut DevClientSession) -> TestResult<(Vec<u8>, appearance::Snapshot)> {
    let state = client
        .session_mut()
        .gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)
        .ok_or("step=snapshot appearance domain missing")?;
    let payload = state.payload.clone();
    let decoded = appearance::Snapshot::decode(&payload)
        .map_err(|error| format!("step=snapshot decode {error:?}"))?;
    Ok((payload, decoded))
}

struct CapturedReply {
    status: CommandStatus,
    command_id: u64,
    server_sequence: u64,
    payload: Vec<u8>,
}

async fn change(
    client: &mut DevClientSession,
    request: &appearance::Change,
) -> TestResult<CapturedReply> {
    let payload = request
        .encode()
        .map_err(|error| format!("appearance change encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            appearance::CAPABILITY_APPEARANCE_V1,
            appearance::COMMAND_TYPE_APPEARANCE_CHANGE,
            &payload,
        )
        .await?;
    Ok(CapturedReply {
        status: reply.status,
        command_id: reply.command_id,
        server_sequence: reply.server_sequence,
        payload: reply.payload,
    })
}

fn content_revision() -> TestResult<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content/cosmetics/outfits/authoring.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    value["document"]["content_revision"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "step=input missing content revision".into())
}

async fn flow(clients: &SeamClients<'_>) -> TestResult<Observed> {
    let mut client = connect_world(clients, 0x69).await?;
    if !client
        .selected_capabilities()
        .contains(&appearance::CAPABILITY_APPEARANCE_V1)
    {
        return Err("step=admission appearance capability not selected".into());
    }
    let (before_bytes, before) = raw_snapshot(&mut client)?;
    let option = before
        .catalogue
        .outfits
        .first()
        .ok_or("step=select no earned outfit")?;
    let colours = match before
        .catalogue
        .selected
        .as_ref()
        .map(|s| s.selection.colours)
    {
        Some([4, 3, 2, 1]) => [1, 2, 3, 4],
        _ => [4, 3, 2, 1],
    };
    let selection = appearance::Selection {
        outfit_key: option.key.clone(),
        colours,
        addons: 0,
        mount_key: None,
    };
    let revision = content_revision()?;
    let request = appearance::Change {
        context: before.catalogue.context,
        expected_root_revision: before.catalogue.root_revision,
        prior: before.catalogue.selected.clone(),
        selection: selection.clone(),
        content_revision: revision.clone(),
    };
    let commit = change(&mut client, &request).await?;
    let committed = match appearance::ChangeResult::decode(&commit.payload) {
        Ok(appearance::ChangeResult::Committed(value))
            if commit.status == CommandStatus::Accepted =>
        {
            value
        }
        observed => {
            return Err(
                format!("step=select status={:?} result={observed:?}", commit.status).into(),
            );
        }
    };
    let mut after = before.clone();
    full_row_s06::wait_for_selection(&mut client, &mut after, committed, &selection).await?;
    let after_bytes = after
        .encode()
        .map_err(|error| format!("step=encode-live-appearance {error:?}"))?;
    if after.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&selection)
        || after.catalogue.root_revision != committed
    {
        return Err("step=select the catalogue does not show the committed selection".into());
    }
    let mut forbidden = selection.clone();
    forbidden.addons = (!option.permitted_addons) & 3;
    if forbidden.addons == 0 {
        return Err("no locked addon in fixture catalogue".into());
    }
    let refusal = appearance::Change {
        context: after.catalogue.context,
        expected_root_revision: after.catalogue.root_revision,
        prior: after.catalogue.selected.clone(),
        selection: forbidden,
        content_revision: revision.clone(),
    };
    let refused = change(&mut client, &refusal).await?;
    if refused.status != CommandStatus::Rejected
        || appearance::ChangeResult::decode(&refused.payload)
            != Ok(appearance::ChangeResult::Rejected)
    {
        return Err(format!("step=refusal status={:?} expected Rejected", refused.status).into());
    }
    client.service_liveness(Duration::from_millis(500)).await?;
    let mut after_refusal = after.clone();
    for event in client.take_events() {
        if let oteryn_dev_client::SessionEvent::Appearance(delta) = event {
            full_row_s06::apply_appearance(&mut after_refusal, delta.value);
        }
    }
    if after_refusal.catalogue != after.catalogue {
        return Err("refusal changed appearance catalogue".into());
    }
    let session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("appearance logout refused".into());
    }
    let mut fresh = connect_world(clients, 0x6a).await?;
    let (_, persisted) = raw_snapshot(&mut fresh)?;
    if fresh.join_snapshot().game_session_id == session
        || persisted.catalogue.selected != after.catalogue.selected
    {
        return Err("appearance selection did not persist across fresh admission".into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("fresh appearance logout refused".into());
    }
    let [head, body, legs, feet] = colours;
    let expected = format!(
        "outfit={}\naddons=0\ncolours={head},{body},{legs},{feet}\ncommitted={committed}\ncontent_revision={revision}\nsession={}\ncommit_id={}\ncommit_sequence={}\ncommit_status={:?}\nrefusal_id={}\nrefusal_sequence={}\nrefusal_status={:?}\n",
        selection.outfit_key,
        uuid_text(session.as_bytes()),
        commit.command_id,
        commit.server_sequence,
        commit.status,
        refused.command_id,
        refused.server_sequence,
        refused.status
    );
    Ok(Observed {
        before: before_bytes,
        after: after_bytes,
        commit: commit.payload,
        refusal: refused.payload,
        request: request.encode().map_err(|error| format!("{error:?}"))?,
        refused_request: refusal.encode().map_err(|error| format!("{error:?}"))?,
        expected,
    })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let capture = std::env::var_os("NODE_BOOT_S06_CAPTURE_DIR")
        .or_else(|| std::env::var_os("E2E_ALL_CAPTURE_DIR"));
    let Some(root) = capture else {
        evidence("e2e_all scenario=S06.client status=FAIL evidence=capture-directory-missing");
        return;
    };
    let root = Path::new(&root);
    match verdict::indexed_timeout(Duration::from_secs(120), flow(clients)).await {
        Ok(Ok(observed)) => {
            for (name, bytes) in [
                ("s06-before.bin", observed.before.as_slice()),
                ("s06-after.bin", observed.after.as_slice()),
                ("s06-commit.bin", observed.commit.as_slice()),
                ("s06-refusal.bin", observed.refusal.as_slice()),
                ("s06-expected.txt", observed.expected.as_bytes()),
                ("s06-request.bin", observed.request.as_slice()),
                (
                    "s06-refused-request.bin",
                    observed.refused_request.as_slice(),
                ),
            ] {
                if let Err(error) = std::fs::write(root.join(name), bytes) {
                    evidence(&format!(
                        "S06_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
                    ));
                    evidence(
                        "e2e_all scenario=S06.client status=FAIL evidence=capture-write-failed",
                    );
                    return;
                }
            }
            evidence(&format!(
                "e2e_all scenario=S06.client status=CAPTURED evidence=snapshot_bytes={}/{}",
                observed.before.len(),
                observed.after.len()
            ));
        }
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S06.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S06.client status=FAIL evidence=timeout_120s"),
    }
}
