//! S06 client replay capture: the outfit selection and the unearned-addon refusal. The native
//! appearance catalogue is read before and after an APPEARANCE_CHANGE that selects an earned outfit
//! with new colours; the raw `gated_snapshot` payloads (`s06-before.bin`, `s06-after.bin`) and the
//! raw command results (`s06-commit.bin`, `s06-refusal.bin`) are written byte for byte, and
//! `s06-expected.txt` holds the selection, the committed revision and the content revision.
//! Mount taming is not driven here: the full row's taming step needs content the world lacks.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::appearance;

struct Observed {
    before: Vec<u8>,
    after: Vec<u8>,
    commit: Vec<u8>,
    refusal: Vec<u8>,
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

async fn change(
    client: &mut DevClientSession,
    request: &appearance::Change,
) -> TestResult<(CommandStatus, Vec<u8>)> {
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
    Ok((reply.status, reply.payload))
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
    let colours = match before.catalogue.selected.as_ref().map(|s| s.selection.colours) {
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
    let (status, commit) = change(&mut client, &request).await?;
    let committed = match appearance::ChangeResult::decode(&commit) {
        Ok(appearance::ChangeResult::Committed(value)) if status == CommandStatus::Accepted => value,
        observed => return Err(format!("step=select status={status:?} result={observed:?}").into()),
    };
    let (after_bytes, after) = raw_snapshot(&mut client)?;
    if after.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&selection)
        || after.catalogue.root_revision != committed
    {
        return Err("step=select the catalogue does not show the committed selection".into());
    }
    let mut forbidden = selection.clone();
    forbidden.addons = option.permitted_addons.saturating_add(1);
    if forbidden.addons == 0 || forbidden.addons == selection.addons {
        forbidden.mount_key = Some("oteryn:content.mount/not-earned".into());
    }
    let refusal = appearance::Change {
        context: after.catalogue.context,
        expected_root_revision: after.catalogue.root_revision,
        prior: after.catalogue.selected.clone(),
        selection: forbidden,
        content_revision: revision.clone(),
    };
    let (status, refused) = change(&mut client, &refusal).await?;
    if status != CommandStatus::Rejected
        || appearance::ChangeResult::decode(&refused) != Ok(appearance::ChangeResult::Rejected)
    {
        return Err(format!("step=refusal status={status:?} expected Rejected").into());
    }
    let [head, body, legs, feet] = colours;
    let expected = format!(
        "outfit={}\naddons=0\ncolours={head},{body},{legs},{feet}\ncommitted={committed}\ncontent_revision={revision}\n",
        selection.outfit_key
    );
    let _ = client.session_mut().logout().await;
    Ok(Observed {
        before: before_bytes,
        after: after_bytes,
        commit,
        refusal: refused,
        expected,
    })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(120), flow(clients)).await {
        Ok(Ok(observed)) => {
            if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let root = Path::new(&root);
                for (name, bytes) in [
                    ("s06-before.bin", observed.before.as_slice()),
                    ("s06-after.bin", observed.after.as_slice()),
                    ("s06-commit.bin", observed.commit.as_slice()),
                    ("s06-refusal.bin", observed.refusal.as_slice()),
                    ("s06-expected.txt", observed.expected.as_bytes()),
                ] {
                    if let Err(error) = std::fs::write(root.join(name), bytes) {
                        evidence(&format!("S06_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
                    }
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
