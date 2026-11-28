//! Live Citizen outfit commit, separate locked-addon refusals and fresh-admission capture.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent, native_session::AppearanceUpdate};
use oteryn_protocol_oteryn::appearance;

const CITIZEN: &str = "oteryn:content.outfit.citizen_outfits";
const FEMALE_CITIZEN_LOOK: u32 = 136;

struct Observed {
    files: Vec<(String, Vec<u8>)>,
    expected: String,
}

fn citizen(catalogue: &appearance::Catalogue) -> TestResult {
    let option = catalogue
        .outfits
        .iter()
        .find(|option| option.key == CITIZEN)
        .ok_or("step=citizen outfit absent")?;
    if option.permitted_addons != 0 || option.look_type != FEMALE_CITIZEN_LOOK {
        return Err(format!(
            "step=citizen expected zero addons and female Citizen look, got {option:?}"
        )
        .into());
    }
    Ok(())
}

fn own_binding<'a>(
    client: &DevClientSession,
    state: &'a appearance::Snapshot,
) -> TestResult<&'a appearance::Binding> {
    let entities = client.world_entities().ok_or("own entity set absent")?;
    let own = entities
        .iter()
        .find(|entity| entity.entity.identity == *entities.own_identity())
        .ok_or("own visible entity absent")?;
    state
        .bindings
        .iter()
        .find(|binding| binding.entity == own.entity)
        .ok_or_else(|| "own appearance binding absent".into())
}

fn require_committed_binding(
    binding: &appearance::Binding,
    expected: &appearance::Look,
    selection_revision: u64,
) -> TestResult {
    require_look(&binding.look, expected)?;
    if binding.selection_revision != Some(selection_revision) {
        return Err(format!(
            "expected visible binding selection revision {selection_revision}, got {:?}",
            binding.selection_revision
        )
        .into());
    }
    Ok(())
}

fn visible_look(client: &mut DevClientSession) -> TestResult<appearance::Look> {
    let own = *client
        .world_entities()
        .ok_or("own entity set absent")?
        .own_identity();
    client
        .session_mut()
        .visible_appearances()
        .into_iter()
        .find(|visible| visible.entity.entity.identity == own)
        .and_then(|visible| visible.look)
        .ok_or_else(|| "own visible appearance absent".into())
}

fn peer_look(
    observer: &mut DevClientSession,
    actor: appearance::EntityRef,
) -> Option<appearance::Look> {
    observer
        .session_mut()
        .visible_appearances()
        .into_iter()
        .find(|visible| visible.entity.entity == actor)
        .and_then(|visible| visible.look)
}

async fn require_peer_look(
    observer: &mut DevClientSession,
    actor: appearance::EntityRef,
    expected: &appearance::Look,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let observed = peer_look(observer, actor);
        if observed.as_ref() == Some(expected) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step=peer-appearance actor={actor:?} expected={expected:?} observed={observed:?}"
            )
            .into());
        }
        observer.service_liveness(Duration::from_millis(25)).await?;
        observer.take_events();
    }
}

fn require_look(observed: &appearance::Look, expected: &appearance::Look) -> TestResult {
    if observed != expected {
        return Err(format!("expected own visible look {expected:?}, got {observed:?}").into());
    }
    Ok(())
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

async fn commit_observed(
    client: &mut DevClientSession,
    state: &mut appearance::Snapshot,
    request: &appearance::Change,
    expected_look: &appearance::Look,
    label: &str,
) -> TestResult<Observed> {
    let commit = change(client, request).await?;
    let committed = match appearance::ChangeResult::decode(&commit.payload) {
        Ok(appearance::ChangeResult::Committed(value))
            if commit.status == CommandStatus::Accepted =>
        {
            value
        }
        observed => {
            return Err(format!(
                "step={label} status={:?} result={observed:?}",
                commit.status
            )
            .into());
        }
    };
    let prior_root = state.catalogue.root_revision;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut observed_update = None;
    loop {
        for event in client.take_events() {
            if let SessionEvent::Appearance(delta) = event {
                if let AppearanceUpdate::Catalogue(catalogue) = &delta.value
                    && catalogue.root_revision == committed
                {
                    if observed_update.is_some() {
                        return Err("step=select duplicate committed catalogue event".into());
                    }
                    observed_update = Some((
                        delta.server_sequence,
                        delta.base_revision,
                        delta.new_revision,
                        catalogue
                            .encode()
                            .map_err(|error| format!("step=encode-catalogue-event {error:?}"))?,
                    ));
                }
                full_row_s06::apply_appearance(state, delta.value);
            }
        }
        if observed_update.is_some() && visible_look(client)? == *expected_look {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=select missing live catalogue event or exact visible look".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let (sequence, base, new, delta) = observed_update.ok_or("missing live catalogue event")?;
    citizen(&state.catalogue)?;
    if state.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&request.selection)
        || state.catalogue.root_revision != committed
        || committed <= prior_root
    {
        return Err("step=select the catalogue does not show the committed selection".into());
    }

    let selected_revision = state
        .catalogue
        .selected
        .as_ref()
        .ok_or("committed selection absent")?
        .revision;
    let binding = own_binding(client, state)?;
    require_committed_binding(binding, expected_look, selected_revision)?;
    if binding.root_revision != committed {
        return Err("step=select visible binding root revision differs from commit".into());
    }

    let files = vec![
        (format!("s06-{label}-event.bin"), delta),
        (format!("s06-{label}.bin"), commit.payload),
        (
            format!("s06-{label}-request.bin"),
            request.encode().map_err(|error| format!("{error:?}"))?,
        ),
    ];
    let expected = format!(
        "{label}_revision={committed}\n{label}_id={}\n{label}_sequence={}\n{label}_status={:?}\n{label}_event_sequence={sequence}\n{label}_event_base={base}\n{label}_event_new={new}\n",
        commit.command_id, commit.server_sequence, commit.status,
    );
    Ok(Observed { files, expected })
}

async fn flow(clients: &SeamClients<'_>) -> TestResult<Observed> {
    let mut observer = connect_character(clients, 0x68, 0).await?;
    let mut client = connect_world(clients, 0x69).await?;
    if !client
        .selected_capabilities()
        .contains(&appearance::CAPABILITY_APPEARANCE_V1)
    {
        return Err("step=admission appearance capability not selected".into());
    }
    let (before_bytes, before) = raw_snapshot(&mut client)?;
    citizen(&before.catalogue)?;
    let actor = own_binding(&client, &before)?.entity;
    let before_look = visible_look(&mut client)?;
    require_peer_look(&mut observer, actor, &before_look).await?;
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
        outfit_key: CITIZEN.to_owned(),
        colours,
        addons: 0,
        mount_key: None,
    };
    let expected_look = appearance::Look::Outfit {
        look_type: FEMALE_CITIZEN_LOOK,
        colours,
        addons: 0,
    };
    if before_look == expected_look {
        return Err("step=select visible look already equals target".into());
    }
    let revision = content_revision()?;
    let request = appearance::Change {
        context: before.catalogue.context,
        expected_root_revision: before.catalogue.root_revision,
        prior: before.catalogue.selected.clone(),
        selection: selection.clone(),
        content_revision: revision.clone(),
    };
    let mut after = before.clone();
    let mut first =
        commit_observed(&mut client, &mut after, &request, &expected_look, "commit").await?;
    require_peer_look(&mut observer, actor, &expected_look).await?;
    evidence("S06_CLIENT_CHECK peer=exact-appearance-after-commit");
    let binding = own_binding(&client, &after)?.clone();
    let control = appearance::Change {
        context: after.catalogue.context,
        expected_root_revision: after.catalogue.root_revision,
        prior: after.catalogue.selected.clone(),
        selection: appearance::Selection {
            colours: [5, 6, 7, 8],
            ..selection.clone()
        },
        content_revision: revision.clone(),
    };
    first.files.push(("s06-before.bin".into(), before_bytes));
    let files = &mut first.files;
    let mut refusals = String::new();
    for addon in [1, 2] {
        let mut refusal = control.clone();
        refusal.selection.addons = addon;
        let refused = change(&mut client, &refusal).await?;
        if refused.status != CommandStatus::Rejected
            || appearance::ChangeResult::decode(&refused.payload)
                != Ok(appearance::ChangeResult::Rejected)
        {
            return Err(format!(
                "step=refusal addon={addon} status={:?} expected Rejected",
                refused.status
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(500)).await?;
        for event in client.take_events() {
            let changed = match event {
                SessionEvent::Appearance(delta) => match delta.value {
                    AppearanceUpdate::Catalogue(catalogue) => catalogue != after.catalogue,
                    AppearanceUpdate::Mappings(mappings) => {
                        mappings.forgets.contains(&binding.entity)
                            || mappings.upserts.iter().any(|updated| {
                                updated.entity == binding.entity && *updated != binding
                            })
                    }
                },
                SessionEvent::GatedSnapshotReplaced(delta)
                    if delta.value.domain_id == appearance::STATE_DOMAIN_APPEARANCE =>
                {
                    let snapshot =
                        appearance::Snapshot::decode(&delta.value.payload).map_err(|error| {
                            format!("step=refusal addon={addon} replacement decode {error:?}")
                        })?;
                    snapshot.catalogue != after.catalogue
                        || own_binding(&client, &snapshot)? != &binding
                }
                _ => false,
            };
            if changed {
                return Err(
                    format!("step=refusal addon={addon} changed catalogue or own binding").into(),
                );
            }
        }
        require_look(&visible_look(&mut client)?, &expected_look)
            .map_err(|error| format!("step=refusal addon={addon} {error}"))?;
        observer
            .service_liveness(Duration::from_millis(500))
            .await?;
        observer.take_events();
        if peer_look(&mut observer, actor).as_ref() != Some(&expected_look) {
            return Err(format!("step=refusal addon={addon} changed peer look").into());
        }
        files.push((format!("s06-refusal-{addon}.bin"), refused.payload));
        files.push((
            format!("s06-refused-request-{addon}.bin"),
            refusal.encode().map_err(|error| format!("{error:?}"))?,
        ));
        use std::fmt::Write as _;
        writeln!(
            refusals,
            "refusal_{addon}_id={}\nrefusal_{addon}_sequence={}\nrefusal_{addon}_status={:?}",
            refused.command_id, refused.server_sequence, refused.status
        )?;
        evidence(&format!(
            "S06_CLIENT_CHECK addon={addon} status=Rejected visible-look=unchanged"
        ));
    }
    let control_look = appearance::Look::Outfit {
        look_type: FEMALE_CITIZEN_LOOK,
        colours: control.selection.colours,
        addons: 0,
    };
    let second =
        commit_observed(&mut client, &mut after, &control, &control_look, "control").await?;
    require_peer_look(&mut observer, actor, &control_look).await?;
    evidence("S06_CLIENT_CHECK valid-control=committed peer=exact-appearance-after-control");
    files.extend(second.files);
    let selected_revision = after
        .catalogue
        .selected
        .as_ref()
        .ok_or("control selection absent")?
        .revision;
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
    citizen(&persisted.catalogue)?;
    require_committed_binding(
        own_binding(&fresh, &persisted)?,
        &control_look,
        selected_revision,
    )
    .map_err(|error| format!("step=relog {error}"))?;
    require_look(&visible_look(&mut fresh)?, &control_look)
        .map_err(|error| format!("step=relog {error}"))?;
    evidence("S06_CLIENT_CHECK relog=exact-catalogue-and-visible-look");
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("fresh appearance logout refused".into());
    }
    if observer.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("observer logout refused".into());
    }
    first.expected = format!(
        "session={}\n{}{refusals}{}",
        uuid_text(session.as_bytes()),
        first.expected,
        second.expected,
    );
    Ok(first)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let (Ok(root), Ok(run)) = (
        std::env::var("NODE_BOOT_S06_CAPTURE_DIR"),
        std::env::var("NODE_BOOT_S06_CAPTURE_RUN"),
    ) else {
        evidence("e2e_all scenario=S06.client status=FAIL evidence=capture-directory-missing");
        return;
    };
    let root = Path::new(&root);
    if run.is_empty() || root.file_name().and_then(|name| name.to_str()) != Some(run.as_str()) {
        evidence("e2e_all scenario=S06.client status=FAIL evidence=capture-run-mismatch");
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(120), flow(clients)).await {
        Ok(Ok(mut observed)) => {
            observed.files.push((
                "s06-expected.txt".into(),
                format!("run={run}\n{}", observed.expected).into_bytes(),
            ));
            for (name, bytes) in observed.files {
                let written = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(root.join(&name))
                    .and_then(|mut file| std::io::Write::write_all(&mut file, &bytes));
                if let Err(error) = written {
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
                "e2e_capture scenario=S06.client status=CAPTURED evidence=run-{run}"
            ));
        }
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S06.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S06.client status=FAIL evidence=timeout_120s"),
    }
}

#[test]
fn s06_citizen_entitlement_and_visible_look_guards_reject_false_positives() -> TestResult {
    let mut catalogue = appearance::Catalogue {
        context: appearance::Context {
            content_generation: [1; 32],
            cosmetics_digest: [2; 32],
        },
        root_revision: 1,
        selected: None,
        palette: Vec::new(),
        outfits: vec![appearance::OutfitOption {
            key: CITIZEN.into(),
            look_type: FEMALE_CITIZEN_LOOK,
            permitted_addons: 0,
        }],
        mounts: Vec::new(),
    };
    citizen(&catalogue)?;
    catalogue.outfits[0].look_type = 128;
    assert!(
        citizen(&catalogue).is_err(),
        "female fixture accepted the male Citizen look"
    );
    catalogue.outfits[0].look_type = FEMALE_CITIZEN_LOOK;
    for invalid in [1, 2, 3] {
        catalogue.outfits[0].permitted_addons = invalid;
        assert!(
            citizen(&catalogue).is_err(),
            "advertised locked addon {invalid}"
        );
    }
    let expected = appearance::Look::Outfit {
        look_type: FEMALE_CITIZEN_LOOK,
        colours: [4, 3, 2, 1],
        addons: 0,
    };
    require_look(&expected, &expected)?;
    for wrong in [
        appearance::Look::Outfit {
            look_type: FEMALE_CITIZEN_LOOK,
            colours: [4, 3, 2, 1],
            addons: 1,
        },
        appearance::Look::Outfit {
            look_type: FEMALE_CITIZEN_LOOK,
            colours: [0; 4],
            addons: 0,
        },
        appearance::Look::Outfit {
            look_type: 128,
            colours: [4, 3, 2, 1],
            addons: 0,
        },
        appearance::Look::EmptyOutfit,
    ] {
        assert!(require_look(&wrong, &expected).is_err());
    }
    Ok(())
}
