//! S06 full-row player scenario, derived from the matrix/re-audit and the appearance/taming owners.
//!
//! 1. Admit the provisioned node-boot character and read its native appearance catalogue. Send an
//!    APPEARANCE_CHANGE selecting an earned outfit with new colours and any earned addons;
//!    expect Accepted/Committed plus a newer catalogue/binding showing that exact selection.
//! 2. Send APPEARANCE_CHANGE for an unearned addon (or mount when no addon refusal candidate is
//!    served); expect Rejected and no catalogue selection/root-revision change.
//! 3. Loot a slingshot from a real hunt, walk beside an authored bear, and send the native
//!    USE-with-creature command. A refused tame can be retried after earning another slingshot.
//! 4. After a successful tame, select the newly earned mount with APPEARANCE_CHANGE; expect
//!    Accepted/Committed and an own-actor appearance mapping carrying the new selection revision.
//! 5. Log out, freshly admit the same character, and re-read the catalogue/binding; expect the
//!    outfit colours/addons, earned mount availability, selected mount, and revisions to persist.

use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent, native_session::AppearanceUpdate};
use oteryn_protocol_oteryn::{appearance, world_object};

const ACTIONS: &[&str] = &[
    "outfit-select",
    "addon-acquisition",
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
        if let Some(failure) = &self.failure {
            evidence(&format!("S06_GAP {failure}"));
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
        Duration::from_secs(360),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err(
            "step=deadline_observed=timeout_360s_need=reachable-hunt-tame-and-appearance-lifecycle"
                .into(),
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

pub(super) fn apply_appearance(state: &mut appearance::Snapshot, update: AppearanceUpdate) {
    match update {
        AppearanceUpdate::Catalogue(catalogue) => state.catalogue = catalogue,
        AppearanceUpdate::Mappings(mappings) => {
            state.bindings.retain(|binding| {
                !mappings.forgets.contains(&binding.entity)
                    && !mappings
                        .upserts
                        .iter()
                        .any(|next| next.entity == binding.entity)
            });
            state.bindings.extend(mappings.upserts);
            state
                .bindings
                .sort_unstable_by_key(|binding| binding.entity);
            if let Some(maximum) = state
                .bindings
                .iter()
                .map(|binding| binding.appearance_ref)
                .max()
            {
                state.high_water = state.high_water.max(maximum);
            }
        }
    }
}

fn look_matches(
    state: &appearance::Snapshot,
    requested: &appearance::Selection,
    binding: &appearance::Binding,
) -> bool {
    state
        .catalogue
        .outfits
        .iter()
        .find(|outfit| outfit.key == requested.outfit_key)
        .is_some_and(|outfit| {
            binding.look
                == appearance::Look::Outfit {
                    look_type: outfit.look_type,
                    colours: requested.colours,
                    addons: requested.addons,
                }
        })
}

pub(super) async fn wait_for_selection(
    client: &mut DevClientSession,
    state: &mut appearance::Snapshot,
    committed: u64,
    requested: &appearance::Selection,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Appearance(delta) = event {
                apply_appearance(state, delta.value);
            }
        }
        if let Some(selected) = &state.catalogue.selected
            && &selected.selection == requested
            && state.catalogue.root_revision == committed
        {
            let binding = own_binding(client, state)?;
            if binding.root_revision == committed
                && binding.selection_revision == Some(selected.revision)
                && look_matches(state, requested, binding)
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=appearance-selection need=catalogue-and-own-binding-deltas revision={committed}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
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

fn unearned_addon(permitted: u8) -> u8 {
    0b11 & !permitted
}

async fn outfit_and_refusal(
    row: &mut Row,
    client: &mut DevClientSession,
    state: &mut appearance::Snapshot,
) -> TestResult<appearance::Selection> {
    let before = &state.catalogue;
    let option = before
        .outfits
        .iter()
        .max_by_key(|outfit| outfit.permitted_addons.count_ones())
        .ok_or("step=outfit-select observed=no-earned-outfit need=starter-outfit-grant")?;
    let permitted = option.permitted_addons;
    let selection = appearance::Selection {
        outfit_key: option.key.clone(),
        colours: [1, 2, 3, 4],
        addons: option.permitted_addons,
        mount_key: None,
    };
    let request = appearance::Change {
        context: before.context,
        expected_root_revision: before.root_revision,
        prior: before.selected.clone(),
        selection: selection.clone(),
        content_revision: content_revision()?,
    };
    let (status, result, sequence) = change(client, &request).await?;
    let committed = match result {
        appearance::ChangeResult::Committed(revision) if status == CommandStatus::Accepted => revision,
        observed => return Err(format!("step=outfit-select observed=status-{status:?}-result-{observed:?} need=committed-selection").into()),
    };
    wait_for_selection(client, state, committed, &selection).await?;
    row.pass(
        "outfit-select",
        format!("committed_revision={committed} sequence={sequence}"),
    );

    let mut forbidden = selection.clone();
    forbidden.addons = unearned_addon(permitted);
    if forbidden.addons == 0 {
        forbidden.mount_key = Some(WAR_BEAR.into());
    }
    let after = state.catalogue.clone();
    let refusal = appearance::Change {
        context: after.context,
        expected_root_revision: after.root_revision,
        prior: after.selected.clone(),
        selection: forbidden,
        content_revision: content_revision()?,
    };
    let (status, result, refusal_sequence) = change(client, &refusal).await?;
    client.service_liveness(Duration::from_millis(500)).await?;
    for event in client.take_events() {
        if let SessionEvent::Appearance(delta) = event {
            apply_appearance(state, delta.value);
        }
    }
    if status != CommandStatus::Rejected
        || result != appearance::ChangeResult::Rejected
        || state.catalogue.root_revision != after.root_revision
        || state.catalogue.selected != after.selected
    {
        return Err(format!("step=unearned-refusal observed=status-{status:?}-result-{result:?}-state-changed-{} need=rejected-with-unchanged-state", state.catalogue != after).into());
    }
    row.pass(
        "unearned-refusal",
        format!("rejected sequence={refusal_sequence}"),
    );
    Ok(selection)
}

const SLINGSHOT: u32 = 5907;
const WAR_BEAR: &str = "oteryn:content.mount.war_bear";
const HUNTER: &str = "oteryn:creature.hunter";
const BEAR: &str = "oteryn:creature.bear";

fn carried(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    let inventory = client
        .inventory()
        .ok_or("step=taming-loot need=inventory-snapshot")?;
    Ok(inventory
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == reference)
        .map(|item| u64::from(item.count.get()))
        .sum())
}

async fn approach_taming_spawn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(move |point| {
                spawns
                    .creatures
                    .get(point.creature as usize)
                    .filter(|definition| definition.key == creature)
                    .map(|_| (point.x, point.y, source.floor))
            })
        })
        .collect();
    // The Hunter woods are outside the shared local route radius from Thais.
    let path = find_route(
        from,
        |at| {
            points.iter().any(|&(x, y, floor)| {
                at.floor == floor && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 1
            })
        },
        |at, direction| {
            destination(world, at, direction).filter(|next| {
                next.x.abs_diff(from.x).max(next.y.abs_diff(from.y)) <= 256
                    && next.floor.abs_diff(from.floor) <= 2
            })
        },
    )
    .map_err(|visited| {
        format!("no walkable route to {creature} among {visited} compiled tiles from {from:?}")
    })?;
    walk(client, world, &path).await?;
    travel_to_spawn(client, world, spawns, creature).await
}

async fn hunt_hunter(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    let here = client.world_spatial().actor_position;
    let entities = client
        .world_entities()
        .ok_or("step=hunter-loot need=live-entities")?;
    let target = entities
        .iter()
        .find(|entry| {
            entry.kind == EntityKind::Creature
                && entry.position.floor == here.floor
                && entry
                    .position
                    .x
                    .abs_diff(here.x)
                    .max(entry.position.y.abs_diff(here.y))
                    <= 1
                && entry
                    .display_name
                    .as_ref()
                    .and_then(|name| name.as_str().ok())
                    .is_some_and(|name| name.eq_ignore_ascii_case("Hunter"))
        })
        .copied()
        .ok_or("step=hunter-loot need=adjacent-live-Hunter")?;
    let corpses_before: std::collections::BTreeSet<_> = entities
        .iter()
        .filter(|e| e.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("hunt attack: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut removed = false;
    let mut last_position = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event {
                if let Some(updated) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target.entity)
                {
                    last_position = updated.position;
                }
                removed |= delta.value.leave.contains(&target.entity);
            }
        }
        if let Some(corpse) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.position == last_position
                        && !corpses_before.contains(&entry.entity)
                })
            })
            .copied()
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("hunt produced no corpse before deadline".into());
        }
    };
    if !removed
        || client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_some())
    {
        return Err("hunt did not observe damage and target removal".into());
    }
    let (EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } | EntityDetail::ObjectWithAppearance {
        item_handle: Some(handle),
        ..
    }) = corpse.detail
    else {
        return Err("hunt corpse has no server-issued item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err("hunt corpse USE refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .open_container()
        .is_none_or(|view| view.container_handle != Some(handle))
    {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("hunt corpse container delta missing".into());
        }
    }
    Ok(())
}

async fn tame_war_bear(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    state: &mut appearance::Snapshot,
) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let spawns = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?.spawns;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let reference = provisioning::item_reference(&provisioning::item_index()?, SLINGSHOT)?;
    if state.catalogue.mounts.iter().any(|mount| mount == WAR_BEAR) {
        return Err(
            "step=taming-attempt observed=war-bear-already-unlocked need=unearned-mount".into(),
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let mut attempt = 0_u32;
    let mut previous_hunt = None;
    let creatures = &spawns.creatures;
    while tokio::time::Instant::now() < deadline {
        let here = tile(client.world_spatial().actor_position)?;
        if let Some(site) = previous_hunt
            && here == site
        {
            let path = route(world, here, |at| {
                at.floor == here.floor && at.x.abs_diff(here.x).max(at.y.abs_diff(here.y)) >= 12
            })
            .ok_or("step=hunter-loot need=walkable-retreat-for-Hunter-respawn")?;
            walk(client, world, &path).await?;
            let delay = spawns
                .sources
                .iter()
                .flat_map(|source| {
                    source.points.iter().filter(move |point| {
                        source.floor == site.floor
                            && creatures
                                .get(point.creature as usize)
                                .is_some_and(|creature| creature.key == HUNTER)
                            && site.x.abs_diff(point.x).max(site.y.abs_diff(point.y)) <= 4
                    })
                })
                .map(|point| point.respawn_ms)
                .max()
                .ok_or("step=hunter-loot need=authored-respawn-for-Hunter")?;
            let wait = Duration::from_millis(u64::from(delay));
            if tokio::time::Instant::now() + wait >= deadline {
                break;
            }
            client.service_liveness(wait).await?;
            client.take_events();
        }
        let before = carried(client, reference)?;
        approach_taming_spawn(client, world, &spawns, HUNTER).await.map_err(|error| {
            verdict::context(format!("step=taming-loot need=reachable-hunter-for-slingshot-i5907 observed={error}"), error)
        })?;
        hunt_hunter(client).await?;
        previous_hunt = Some(tile(client.world_spatial().actor_position)?);
        if carried(client, reference)? == before {
            let item = client.open_container().and_then(|view| {
                view.entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == reference)
            });
            let Some(item) = item else {
                evidence("S06_CHECK step=hunter-loot observed=empty-slingshot-roll retry=true");
                continue;
            };
            let moved = client
                .move_item(&oteryn_dev_client::ItemMoveIntent {
                    source: item.handle,
                    destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
                })
                .await?;
            if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
                return Err(format!(
                    "step=taming-loot observed={moved:?} need=slingshot-i5907-in-backpack"
                )
                .into());
            }
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let funded = loop {
            let count = carried(client, reference)?;
            if count > before {
                break count;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(
                    "step=taming-loot observed=no-inventory-increase need=looted-slingshot-i5907"
                        .into(),
                );
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        };
        approach_taming_spawn(client, world, &spawns, BEAR)
            .await
            .map_err(|error| {
                verdict::context(
                    format!("step=taming-attempt need=reachable-live-bear observed={error}"),
                    error,
                )
            })?;
        attempt += 1;
        let here = client.world_spatial().actor_position;
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Creature
                        && entry.position.floor == here.floor
                        && entry
                            .position
                            .x
                            .abs_diff(here.x)
                            .max(entry.position.y.abs_diff(here.y))
                            <= 1
                        && entry
                            .display_name
                            .as_ref()
                            .and_then(|name| name.as_str().ok())
                            .is_some_and(|name| name.eq_ignore_ascii_case("bear"))
                })
            })
            .map(|entry| entry.entity)
            .ok_or("step=taming-attempt need=adjacent-server-identified-bear")?;
        client.take_events();
        let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
            target: world_object::UseTarget::ItemByDefinition(reference),
            use_with: Some(target),
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("taming USE encode: {error:?}"))?;
        let reply = client.session_mut().world_context_use(&payload).await?;
        let (disposition, _) = world_object::decode_use_result_with_gate(&reply.payload)
            .map_err(|error| format!("step=taming-attempt observed=reply-decode-{error:?}"))?;
        if reply.status != CommandStatus::Accepted
            || disposition != world_object::UseDisposition::Committed
        {
            return Err(format!(
                "step=taming-attempt observed={reply:?}/{disposition:?} need=committed-use-on-bear"
            )
            .into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let mut success_seen = false;
        loop {
            let mut failed = false;
            for event in client.take_events() {
                match event {
                    SessionEvent::Appearance(delta) => apply_appearance(state, delta.value),
                    SessionEvent::ChatLine(delta) => {
                        if let oteryn_dev_client::ChatLine::Private { speaker_name, text } =
                            delta.value
                            && speaker_name == "Game"
                        {
                            success_seen |= text == "You have tamed the war bear.";
                            failed |= matches!(
                                text.as_str(),
                                "The bear ran away."
                                    | "Oh no! The slingshot broke."
                                    | "The bear is trying to hit you with its claws."
                            );
                        }
                    }
                    _ => {}
                }
            }
            if failed {
                break;
            }
            let unlocked = state.catalogue.mounts.iter().any(|mount| mount == WAR_BEAR);
            let removed = client
                .world_entities()
                .is_some_and(|entities| entities.get(&target).is_none());
            if success_seen && unlocked && removed && carried(client, reference)? + 1 == funded {
                row.pass(
                    "taming-attempt",
                    format!(
                        "war-bear-unlocked attempt={attempt} sequence={}",
                        reply.server_sequence
                    ),
                );
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("step=taming-attempt need=war-bear-unlock-bear-removal-and-slingshot-consumption observed=unlock-{unlocked}-removed-{removed}").into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        }
    }
    Err("step=hunter-loot-or-taming observed=acquisition-deadline need=looted-slingshot-and-successful-war-bear-tame".into())
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
    let mut state = snapshot(&mut client)?;
    let admission_catalogue = state.catalogue.clone();
    let selected = outfit_and_refusal(row, &mut client, &mut state).await?;
    tame_war_bear(row, &mut client, clients, &mut state).await?;

    let mount = WAR_BEAR.to_owned();
    let mut mounted = selected;
    mounted.mount_key = Some(mount.clone());
    let original_addons = admission_catalogue
        .outfits
        .iter()
        .find(|outfit| outfit.key == mounted.outfit_key)
        .map_or(0, |outfit| outfit.permitted_addons);
    let earned_addons = state
        .catalogue
        .outfits
        .iter()
        .find(|outfit| outfit.key == mounted.outfit_key)
        .map_or(0, |outfit| outfit.permitted_addons & !original_addons);
    mounted.addons |= earned_addons;
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
    wait_for_selection(&mut client, &mut state, committed, &mounted).await?;
    let selected_revision = state
        .catalogue
        .selected
        .as_ref()
        .map(|selected| selected.revision)
        .ok_or("step=mount-select observed=missing-selected-revision")?;
    row.pass(
        "mount-select",
        format!("committed_revision={committed} selected_revision={selected_revision} sequence={sequence}"),
    );
    if earned_addons != 0 {
        row.pass(
            "addon-acquisition",
            format!("earned-and-selected-mask={earned_addons}"),
        );
    }
    let previous = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=fresh-admission observed=logout-refused".into());
    }
    let mut fresh = connect_world(clients, 0xb7).await?;
    let persisted = snapshot(&mut fresh)?;
    let persisted_binding = own_binding(&fresh, &persisted)?;
    if fresh.join_snapshot().game_session_id == previous
        || persisted.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&mounted)
        || !persisted.catalogue.mounts.contains(&mount)
        || persisted.catalogue.root_revision != committed
        || persisted.catalogue.selected.as_ref().map(|s| s.revision) != Some(selected_revision)
        || !look_matches(&persisted, &mounted, persisted_binding)
        || persisted_binding.root_revision != committed
        || persisted_binding.selection_revision != Some(selected_revision)
    {
        return Err(format!("step=fresh-admission observed=selection-unlock-or-revision-mismatch need=mount-{mount}-root-{committed}-selection-{selected_revision}").into());
    }
    row.pass(
        "fresh-admission",
        format!("selected-outfit-addon-mount-and-unlock-persisted root_revision={committed} selected_revision={selected_revision}"),
    );
    if earned_addons == 0 {
        evidence("S06_GAP step=addon-acquisition need=player-earned-addon-after-admission");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_uses_a_real_unearned_addon_bit() {
        assert_eq!(unearned_addon(2), 1);
        assert_eq!(unearned_addon(3), 0);
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

#[test]
fn full_verdict_requires_evidence_of_addon_acquisition() {
    assert!(ACTIONS.contains(&"addon-acquisition"));
}
