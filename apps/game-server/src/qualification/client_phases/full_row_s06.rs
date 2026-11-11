//! S06 full-row player scenario, derived from the matrix/re-audit and the appearance/taming owners.
//!
//! 1. Admit the provisioned node-boot character and read its native appearance catalogue. Send an
//!    APPEARANCE_CHANGE selecting an earned starter outfit with new colours and no addons/mount;
//!    expect Accepted/Committed plus a newer catalogue/binding showing that exact selection.
//! 2. Send APPEARANCE_CHANGE for an unearned addon (or mount when no addon refusal candidate is
//!    served); expect Rejected and no catalogue selection/root-revision change.
//! 3. Acquire a taming item only through ordinary world loot or NPC trade, walk beside its authored
//!    creature, and send the native USE-with-creature command; expect a committed attempt and its
//!    inventory/creature/message delta. A failed random attempt is retried only after another item
//!    is legitimately acquired. If the compiled player world exposes no such acquisition route,
//!    still send the hotkey USE for an exact taming definition and report the observed refusal.
//! 4. After a successful tame, select the newly earned mount with APPEARANCE_CHANGE; expect
//!    Accepted/Committed and an own-actor appearance mapping carrying the new selection revision.
//! 5. Log out, freshly admit the same character, and re-read the catalogue/binding; expect the
//!    outfit colours/addons, earned mount availability, selected mount, and revisions to persist.
//!
//! This row deliberately cannot turn absent acquisition content into a source-derived verdict: the
//! native taming command is sent before a runtime FAIL is emitted, with its status/disposition.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{appearance, world_object};

const ACTIONS: &[&str] = &[
    "outfit-select",
    "unearned-refusal",
    "taming-attempt",
    "mount-select",
    "fresh-admission",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
    failure: Option<String>,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: Default::default(),
            started: std::time::Instant::now(),
            failure: None,
        }
    }

    fn pass(&mut self, action: &'static str, detail: impl std::fmt::Display) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("S06_CHECK {action} PASS {detail}"));
        } else {
            self.failure = Some(format!("invalid-or-duplicate-step-{action}"));
        }
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            self.failure = Some(error.to_string().replace(' ', "_"));
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("S06_GAP {action} unproven"));
            }
        }
        let complete = self.failure.is_none() && ACTIONS.iter().all(|a| self.passed.contains(a));
        let suffix = self
            .failure
            .as_deref()
            .map(|failure| format!(" evidence={failure}"))
            .unwrap_or_default();
        evidence(&format!(
            "e2e_all scenario=S06.full status={} seconds={:.3}{suffix}",
            if complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete(journey(&mut row, clients)).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err(
            "step=deadline_observed=timeout_120s_need=responsive_appearance_or_use_route".into(),
        ),
    });
}

fn snapshot(client: &mut DevClientSession) -> TestResult<appearance::Snapshot> {
    let state = client
        .session_mut()
        .gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)
        .ok_or("step=appearance-snapshot observed=missing need=appearance-state-domain")?;
    appearance::Snapshot::decode(&state.payload)
        .map_err(|error| format!("step=appearance-snapshot observed=decode-{error:?}").into())
}

fn content_revision() -> TestResult<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content/cosmetics/outfits/authoring.json");
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    value["document"]["content_revision"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "step=appearance-input observed=missing-content-revision".into())
}

async fn change(
    client: &mut DevClientSession,
    request: &appearance::Change,
) -> TestResult<(CommandStatus, appearance::ChangeResult, u64)> {
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
    let result = appearance::ChangeResult::decode(&reply.payload)
        .map_err(|error| format!("appearance reply decode: {error:?}"))?;
    Ok((reply.status, result, reply.server_sequence))
}

fn own_binding<'a>(
    client: &DevClientSession,
    state: &'a appearance::Snapshot,
) -> TestResult<&'a appearance::Binding> {
    let entities = client.world_entities().ok_or("own entity set absent")?;
    state
        .bindings
        .iter()
        .find(|binding| binding.entity.identity == *entities.own_identity())
        .ok_or_else(|| "own appearance binding absent".into())
}

async fn outfit_and_refusal(
    row: &mut Row,
    client: &mut DevClientSession,
) -> TestResult<appearance::Selection> {
    let before = snapshot(client)?;
    let option = before
        .catalogue
        .outfits
        .first()
        .ok_or("step=outfit-select observed=no-earned-outfit need=starter-outfit-grant")?;
    let selection = appearance::Selection {
        outfit_key: option.key.clone(),
        colours: [1, 2, 3, 4],
        addons: 0,
        mount_key: None,
    };
    let request = appearance::Change {
        context: before.catalogue.context,
        expected_root_revision: before.catalogue.root_revision,
        prior: before.catalogue.selected.clone(),
        selection: selection.clone(),
        content_revision: content_revision()?,
    };
    let (status, result, sequence) = change(client, &request).await?;
    let committed = match result {
        appearance::ChangeResult::Committed(revision) if status == CommandStatus::Accepted => revision,
        observed => return Err(format!("step=outfit-select observed=status-{status:?}-result-{observed:?} need=committed-selection").into()),
    };
    let after = snapshot(client)?;
    if after.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&selection)
        || after.catalogue.root_revision != committed
        || own_binding(client, &after)?.selection_revision
            != after.catalogue.selected.as_ref().map(|s| s.revision)
    {
        return Err(format!("step=outfit-select observed=missing-selection-delta-sequence-{sequence} need=catalogue-and-own-binding-delta").into());
    }
    row.pass(
        "outfit-select",
        format!("committed_revision={committed} sequence={sequence}"),
    );

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
        content_revision: content_revision()?,
    };
    let (status, result, refusal_sequence) = change(client, &refusal).await?;
    let unchanged = snapshot(client)?;
    if status != CommandStatus::Rejected
        || result != appearance::ChangeResult::Rejected
        || unchanged.catalogue.root_revision != after.catalogue.root_revision
        || unchanged.catalogue.selected != after.catalogue.selected
    {
        return Err(format!("step=unearned-refusal observed=status-{status:?}-result-{result:?}-state-changed-{} need=rejected-with-unchanged-state", unchanged.catalogue != after.catalogue).into());
    }
    row.pass(
        "unearned-refusal",
        format!("rejected sequence={refusal_sequence}"),
    );
    Ok(selection)
}

async fn taming_attempt(client: &mut DevClientSession) -> TestResult {
    let definitions = provisioning::item_index()?;
    let reference = provisioning::item_reference(&definitions, 16251)?;
    let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
        target: world_object::UseTarget::ItemByDefinition(reference),
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("taming USE encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (result, _) = world_object::decode_use_result_with_gate(&reply.payload)
        .map_err(|error| format!("step=taming-attempt observed=reply-decode-{error:?}"))?;
    Err(format!(
        "step=taming-attempt observed=status-{status:?}-disposition-{result:?}-sequence-{sequence}-definition-oteryn:item.tibia.i16251 need=legitimate-player-acquisition-authored-creature-and-inventory-creature-message-deltas",
        status = reply.status,
        sequence = reply.server_sequence,
    )
    .into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0xb6).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
    if !client
        .selected_capabilities()
        .contains(&appearance::CAPABILITY_APPEARANCE_V1)
    {
        return Err(
            "step=admission observed=appearance-capability-not-selected need=capability-25".into(),
        );
    }
    let selected = outfit_and_refusal(row, &mut client).await?;
    // This is an actual native taming mechanic command. Failure returns only after the server's
    // reply was decoded, and therefore cannot become a canned/source-only verdict.
    taming_attempt(&mut client).await?;

    let state = snapshot(&mut client)?;
    let mount = state
        .catalogue
        .mounts
        .first()
        .cloned()
        .ok_or("step=mount-select observed=no-earned-mount")?;
    let mut mounted = selected;
    mounted.mount_key = Some(mount.clone());
    let request = appearance::Change {
        context: state.catalogue.context,
        expected_root_revision: state.catalogue.root_revision,
        prior: state.catalogue.selected.clone(),
        selection: mounted.clone(),
        content_revision: content_revision()?,
    };
    let (status, result, sequence) = change(&mut client, &request).await?;
    let committed = match result {
        appearance::ChangeResult::Committed(revision) if status == CommandStatus::Accepted => revision,
        observed => return Err(format!("step=mount-select observed=status-{status:?}-result-{observed:?} need=tamed-mount-selection").into()),
    };
    let mounted_state = snapshot(&mut client)?;
    let mounted_selected = mounted_state.catalogue.selected.as_ref();
    let mounted_binding = own_binding(&client, &mounted_state)?;
    if mounted_selected.map(|selected| &selected.selection) != Some(&mounted)
        || mounted_state.catalogue.root_revision != committed
        || mounted_selected.map(|selected| selected.revision) != mounted_binding.selection_revision
        || mounted_binding.root_revision != committed
    {
        return Err(format!("step=mount-select observed=missing-catalogue-or-own-binding-delta-sequence-{sequence} need=committed-revision-{committed}-and-selected-mount-{mount}").into());
    }
    let selected_revision = mounted_selected
        .map(|selected| selected.revision)
        .ok_or("step=mount-select observed=missing-selected-revision")?;
    row.pass(
        "mount-select",
        format!("committed_revision={committed} selected_revision={selected_revision} sequence={sequence}"),
    );
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=fresh-admission observed=logout-refused".into());
    }
    row_needs_logout_0 = false;
    let mut fresh = connect_world(clients, 0xb7).await?;
    let row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let persisted = snapshot(&mut fresh)?;
    let persisted_binding = own_binding(&fresh, &persisted)?;
    if persisted.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&mounted)
        || !persisted.catalogue.mounts.contains(&mount)
        || persisted.catalogue.root_revision != committed
        || persisted.catalogue.selected.as_ref().map(|s| s.revision) != Some(selected_revision)
        || persisted_binding.root_revision != committed
        || persisted_binding.selection_revision != Some(selected_revision)
    {
        return Err(format!("step=fresh-admission observed=selection-unlock-or-revision-mismatch need=mount-{mount}-root-{committed}-selection-{selected_revision}").into());
    }
    row.pass(
        "fresh-admission",
        format!("selected-outfit-addon-mount-and-unlock-persisted root_revision={committed} selected_revision={selected_revision}"),
    );
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { row_logout(&mut fresh).await?; }
    row_result

    }.await;
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_requires_the_complete_lifecycle_once() {
        let mut row = Row::new();
        for action in ACTIONS {
            row.pass(action, "unit");
        }
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        assert!(row.failure.is_none());
    }
}
