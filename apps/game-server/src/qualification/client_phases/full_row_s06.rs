//! S06 full-row player scenario, derived from the matrix/re-audit and the appearance/taming owners.
//!
//! 1. Admit the provisioned node-boot character and read its native appearance catalogue. Send an
//!    APPEARANCE_CHANGE selecting an earned starter outfit with new colours and no addons/mount;
//!    expect Accepted/Committed plus a newer catalogue/binding showing that exact selection.
//! 2. Send APPEARANCE_CHANGE for an unearned addon (or mount when no addon refusal candidate is
//!    served); expect Rejected and no catalogue selection/root-revision change.
//! 3. Claim the Golem Wrench through its authored reward chest if needed. Walk to a
//!    modified gnarlhound spawn and send USE with the carried wrench and live creature.
//!    Require the exact mount grant, target removal, retained wrench and Game feedback.
//! 4. After a successful tame, select the newly earned mount with APPEARANCE_CHANGE; expect
//!    Accepted/Committed and an own-actor appearance mapping carrying the new selection revision.
//! 5. Log out, freshly admit the same character, and re-read the catalogue/binding; expect the
//!    outfit colours/addons, earned mount availability, selected mount, and revisions to persist.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
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
    let result = verdict::indexed_timeout(
        Duration::from_secs(120),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
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

fn taming_committed(status: CommandStatus, disposition: world_object::UseDisposition) -> bool {
    status == CommandStatus::Accepted && disposition == world_object::UseDisposition::Committed
}

const WRENCH_ID: u32 = 16251;
const WRENCH_CLAIM: &str = "oteryn:reward-claim.kv.golem_wrench";
const GNARLHOUND: &str = "oteryn:creature.modified_gnarlhound";
const MOUNT: &str = "oteryn:content.mount.gnarlhound";
const TAME_FEEDBACK: &str = "You now own a modified gnarlhound.";

fn carried_wrench(
    client: &DevClientSession,
    reference: std::num::NonZeroU32,
) -> Option<oteryn_protocol_oteryn::item_view::ItemHandle> {
    client
        .inventory()?
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == reference)
        .map(|entry| entry.handle)
}

async fn claim_wrench(
    client: &mut DevClientSession,
    world: &BundleWorld,
    definitions: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|item| {
            item.key()
                .as_str()
                .strip_prefix("oteryn:item.tibia.i")
                .and_then(|id| id.parse().ok())
                .is_some_and(|id| provisioning::item_reference(definitions, id).is_ok())
        },
    )?;
    let claim = report
        .served
        .iter()
        .find(|claim| claim.key.as_str() == WRENCH_CLAIM)
        .ok_or("step=taming-acquisition observed=no-bound-wrench-claim")?;
    let placement = claim
        .placements
        .first()
        .ok_or("step=taming-acquisition observed=no-wrench-chest")?;
    let target = TilePos {
        x: placement.cell.x,
        y: placement.cell.y,
        floor: placement.cell.floor,
    };
    let start = tile(client.world_spatial().actor_position)?;
    let path = reach_route(client, world, start, |at| {
        at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
    })
    .await?
    .map_err(|searched| {
        format!("step=taming-acquisition observed=no-wrench-chest-route {searched}")
    })?;
    walk(client, world, &path).await?;
    let mut key = world.base().digest().to_vec();
    key.extend_from_slice(&placement.bundle_placement_key.to_be_bytes());
    let reply = client.session_mut().use_object(&key, 0).await?;
    if !taming_committed(reply.status, reply.disposition) {
        return Err(format!("step=taming-acquisition observed=wrench-claim-{reply:?}").into());
    }
    Ok(())
}

fn taming_requires_step(here: ActorPosition, target: ActorPosition) -> bool {
    here.floor != target.floor || here.x.abs_diff(target.x).max(here.y.abs_diff(target.y)) > 1
}

async fn approach_taming_target(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: oteryn_dev_client::EntityRef,
) -> TestResult {
    for _ in 0..32 {
        let here = client.world_spatial().actor_position;
        let target_position = client
            .world_entities()
            .and_then(|entities| entities.get(&target))
            .map(|entry| entry.position)
            .ok_or("step=taming-attempt observed=named-target-left-view")?;
        if !taming_requires_step(here, target_position) {
            return Ok(());
        }
        let path = route(world, tile(here)?, |at| {
            !taming_requires_step(
                ActorPosition {
                    x: at.x.into(),
                    y: at.y.into(),
                    floor: -i16::from(at.floor),
                },
                target_position,
            )
        })
        .ok_or("step=taming-attempt observed=no-route-to-named-target")?;
        let step = path
            .first()
            .ok_or("step=taming-attempt observed=empty-approach-route")?;
        walk(client, world, std::slice::from_ref(step)).await?;
    }
    Err("step=taming-attempt observed=named-target-outside-use-range-after-approach".into())
}

async fn taming_attempt(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<String> {
    let definitions = provisioning::item_index()?;
    let reference = provisioning::item_reference(&definitions, WRENCH_ID)?;
    let before = snapshot(client)?;
    if before.catalogue.mounts.iter().any(|mount| mount == MOUNT) {
        return Err("step=taming-attempt observed=gnarlhound-already-earned".into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    if carried_wrench(client, reference).is_none() {
        claim_wrench(client, world, &definitions).await?;
    }
    let source = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(handle) = carried_wrench(client, reference) {
                return Ok::<_, oteryn_dev_client::DevClientError>(handle);
            }
            client.take_events();
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await
    .map_err(|_| "step=taming-acquisition observed=no-carried-wrench")??;
    travel_to_spawn(client, world, &metadata.spawns, GNARLHOUND).await?;
    let target = full_row_w02::named_creature(client, "modified gnarlhound")
        .await?
        .ok_or("step=taming-attempt observed=no-visible-modified-gnarlhound")?;
    approach_taming_target(client, world, target).await?;
    client.take_events();
    let before_use_sequence = client.last_server_sequence();
    let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
        target: world_object::UseTarget::Item(source),
        use_with: Some(target),
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("taming USE encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (result, _) = world_object::decode_use_result_with_gate(&reply.payload)
        .map_err(|error| format!("step=taming-attempt observed=reply-decode-{error:?}"))?;
    if !taming_committed(reply.status, result) {
        return Err(format!(
            "step=taming-attempt observed=status-{:?}-disposition-{result:?}-sequence-{}",
            reply.status, reply.server_sequence
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut feedback = false;
    loop {
        let state = snapshot(client)?;
        let earned = state.catalogue.mounts.iter().any(|mount| mount == MOUNT);
        let removed = client
            .world_entities()
            .is_some_and(|entities| entities.get(&target).is_none());
        feedback |= client.take_events().iter().any(|event| {
            matches!(event, SessionEvent::ChatLine(delta)
                if delta.server_sequence > before_use_sequence
                    && matches!(&delta.value, oteryn_protocol_oteryn::chat::ChatLine::Private { speaker_name, text }
                        if speaker_name == "Game" && text == TAME_FEEDBACK))
        });
        if earned && removed && feedback && carried_wrench(client, reference) == Some(source) {
            return Ok(MOUNT.to_owned());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=taming-attempt observed=committed-without-grant-removal-retained-wrench-or-feedback earned={earned} removed={removed} feedback={feedback}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb6).await?;
    if !client
        .selected_capabilities()
        .contains(&appearance::CAPABILITY_APPEARANCE_V1)
    {
        return Err(
            "step=admission observed=appearance-capability-not-selected need=capability-25".into(),
        );
    }
    let selected = outfit_and_refusal(row, &mut client).await?;
    let mount = taming_attempt(&mut client, clients).await?;
    row.pass("taming-attempt", &mount);

    let state = snapshot(&mut client)?;
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
    let mut fresh = connect_world(clients, 0xb7).await?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taming_committed_requires_accepted_committed_reply() {
        use world_object::UseDisposition;
        assert!(taming_committed(
            CommandStatus::Accepted,
            UseDisposition::Committed
        ));
        assert!(!taming_committed(
            CommandStatus::Rejected,
            UseDisposition::Committed
        ));
        assert!(!taming_committed(
            CommandStatus::Accepted,
            UseDisposition::NothingToUse
        ));
    }

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

#[cfg(test)]
mod delta_review_tests {
    use super::*;
    #[test]
    fn named_taming_target_two_tiles_away_requires_approach() {
        let here = ActorPosition {
            x: 10,
            y: 15,
            floor: 7,
        };
        assert!(taming_requires_step(here, ActorPosition { x: 12, ..here }));
        assert!(taming_requires_step(
            here,
            ActorPosition { floor: 8, ..here }
        ));
        assert!(!taming_requires_step(
            here,
            ActorPosition {
                x: 11,
                y: 16,
                ..here
            }
        ));
    }
}
