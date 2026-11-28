//! S06 full-row player scenario, derived from the matrix/re-audit and the appearance/taming owners.
//!
//! 1. Admit the provisioned node-boot character and read its native appearance catalogue. Send an
//!    APPEARANCE_CHANGE selecting an earned outfit with new colours and any earned addons;
//!    expect Accepted/Committed plus a newer catalogue/binding showing that exact selection.
//! 2. Send APPEARANCE_CHANGE for an unearned addon; expect Rejected and no catalogue selection/root-revision change.
//! 3. Use the outfit and addon items on the dedicated account, then walk beside an authored
//!    bear and send one slingshot USE-with-creature command with a fixture-armed draw.
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
        Err(_) => {
            Err("step=deadline_observed=timeout_360s_need=tame-and-appearance-lifecycle".into())
        }
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
    if before
        .selected
        .as_ref()
        .is_some_and(|previous| previous.selection == selection)
        || look_matches(state, &selection, own_binding(client, state)?)
    {
        return Err("step=outfit-select need=pre-state-different-from-requested-outfit".into());
    }
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
        return Err("step=unearned-refusal need=a-locked-addon".into());
    }
    let after = state.catalogue.clone();
    let before_refusal_binding = (*own_binding(client, state)?).clone();
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
    let after_refusal_binding = own_binding(client, state)?;
    if status != CommandStatus::Rejected
        || result != appearance::ChangeResult::Rejected
        || state.catalogue != after
        || after_refusal_binding.root_revision != before_refusal_binding.root_revision
        || after_refusal_binding.selection_revision != before_refusal_binding.selection_revision
        || after_refusal_binding.look != before_refusal_binding.look
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
const OUTFIT_PARTS: [u32; 2] = [16252, 16253];
const SLINGSHOTS: u32 = 1;
const BEAR: &str = "oteryn:creature.bear";

fn carried(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    let inventory = client
        .inventory()
        .ok_or("step=inventory need=inventory-snapshot")?;
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
) -> TestResult<oteryn_dev_client::EntityRef> {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(move |point| {
                spawns
                    .creatures
                    .get(point.creature as usize)
                    .filter(|definition| definition.key == BEAR)
                    .map(|_| TilePos {
                        x: point.x,
                        y: point.y,
                        floor: source.floor,
                    })
            })
        })
        .collect();
    let nearby = |at: TilePos, point: TilePos, distance| {
        at.floor == point.floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= distance
    };
    let path = find_route(
        from,
        |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && points.iter().any(|&point| nearby(at, point, 1))
        },
        |at, direction| {
            destination(world, at, direction).filter(|next| {
                next.x.abs_diff(from.x).max(next.y.abs_diff(from.y)) <= 256
                    && next.floor.abs_diff(from.floor) <= 2
            })
        },
    )
    .map_err(|visited| {
        format!("no walkable route to an authored bear among {visited} tiles from {from:?}")
    })?;
    walk(client, world, &path).await?;
    let here = tile(client.world_spatial().actor_position)?;
    let anchor = points
        .into_iter()
        .find(|&point| nearby(here, point, 1))
        .ok_or("bear route terminated outside the authored spawn")?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        .is_some_and(|name| name.eq_ignore_ascii_case("bear"))
                    && tile(entry.position).is_ok_and(|position| nearby(position, anchor, 4))
            })
        })
        .map(|entry| entry.entity)
        .ok_or("authored bear spawn has no visible bear")?;
    // Keep the exact first observed creature as it moves during the approach.
    for _ in 0..32 {
        let here = tile(client.world_spatial().actor_position)?;
        let position = client
            .world_entities()
            .and_then(|entities| entities.get(&target))
            .ok_or("authored bear disappeared before taming")?
            .position;
        let position = tile(position)?;
        if nearby(here, position, 1) {
            return Ok(target);
        }
        let path = route(world, here, |at| nearby(at, position, 1))
            .ok_or("visible bear is unreachable")?;
        let step = path.first().ok_or("bear approach did not advance")?;
        walk(client, world, &[*step]).await?;
    }
    Err("could not approach the authored bear within the route budget".into())
}

async fn earn_addon(
    row: &mut Row,
    client: &mut DevClientSession,
    state: &mut appearance::Snapshot,
) -> TestResult<String> {
    const OUTFIT: &str = "oteryn:content.outfit.soil_guardian_outfits";
    if state
        .catalogue
        .outfits
        .iter()
        .any(|outfit| outfit.key == OUTFIT)
    {
        return Err("step=addon-acquisition observed=outfit-already-earned".into());
    }
    let item_index = provisioning::item_index()?;
    for (item, expected_mask) in OUTFIT_PARTS.into_iter().zip([0, 1]) {
        let reference = provisioning::item_reference(&item_index, item)?;
        if carried(client, reference)? != 1 {
            return Err(format!("step=addon-acquisition item={item} need=one-fixture-item").into());
        }
        let handle = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|entry| entry.item_definition_ref == reference)
            })
            .map(|entry| entry.handle)
            .ok_or_else(|| format!("step=addon-acquisition item={item} need=inventory-handle"))?;
        let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
            target: world_object::UseTarget::Item(handle),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("outfit item USE encode: {error:?}"))?;
        let reply = client.session_mut().world_context_use(&payload).await?;
        let (disposition, _) = world_object::decode_use_result_with_gate(&reply.payload)
            .map_err(|error| format!("outfit item USE reply: {error:?}"))?;
        if reply.status != CommandStatus::Accepted
            || disposition != world_object::UseDisposition::Committed
        {
            return Err(format!(
                "step=addon-acquisition item={item} observed={:?}/{disposition:?}",
                reply.status
            )
            .into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            for event in client.take_events() {
                if let SessionEvent::Appearance(delta) = event {
                    apply_appearance(state, delta.value);
                }
            }
            let actual_mask = state
                .catalogue
                .outfits
                .iter()
                .find(|outfit| outfit.key == OUTFIT)
                .map(|outfit| outfit.permitted_addons);
            if actual_mask == Some(expected_mask) && carried(client, reference)? == 0 {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("step=addon-acquisition item={item} observed={actual_mask:?} need=consumed-and-mask-{expected_mask}").into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        }
    }
    row.pass(
        "addon-acquisition",
        format!("outfit={OUTFIT} earned-mask=1"),
    );
    Ok(OUTFIT.into())
}

// Crystal mounts.lua at 201189f8 defines action 3 as ACTION_NONE.
fn taming_feedback(text: &str) -> Option<(bool, bool, bool)> {
    // (unlocked, bear removed, slingshot consumed)
    match text {
        "You have tamed the war bear." => Some((true, true, true)),
        "The bear ran away." => Some((false, true, false)),
        "Oh no! The slingshot broke." => Some((false, false, true)),
        "The bear is trying to hit you with its claws." => Some((false, false, false)),
        _ => None,
    }
}

async fn tame_war_bear(
    row: &mut Row,
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    state: &mut appearance::Snapshot,
) -> TestResult<u64> {
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
    let funded = carried(client, reference)?;
    if funded != u64::from(SLINGSHOTS) {
        return Err(
            "step=taming-attempt need=exact-fixture-slingshot-count-before-first-use".into(),
        );
    }
    let target = approach_taming_spawn(client, world, &spawns)
        .await
        .map_err(|error| {
            verdict::context(
                format!("step=taming-attempt need=reachable-live-bear observed={error}"),
                error,
            )
        })?;
    client.take_events();
    let payload = world_object::encode_item_use_intent(&world_object::UseIntent {
        target: world_object::UseTarget::ItemByDefinition(reference),
        use_with: Some(target),
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(|error| format!("taming USE encode: {error:?}"))?;
    let command_id = client.next_command_id();
    let armed = request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--taming-draw",
        command_id.to_string(),
    )
    .await
    .map_err(|error| {
        format!("step=taming-attempt need=one-use-qualification-draw observed={error}")
    })?;
    if armed != "armed" {
        return Err(format!("step=taming-attempt expected=armed observed={armed}").into());
    }
    evidence(&format!(
        "S06_FIXTURE input=taming-draw value=0 command_id={command_id} status=armed"
    ));
    let reply = client.session_mut().world_context_use(&payload).await?;
    let (disposition, _) = world_object::decode_use_result_with_gate(&reply.payload)
        .map_err(|error| format!("step=taming-attempt observed=reply-decode-{error:?}"))?;
    if reply.command_id != command_id
        || reply.status != CommandStatus::Accepted
        || disposition != world_object::UseDisposition::Committed
    {
        return Err(format!(
            "step=taming-attempt observed={reply:?}/{disposition:?} need=committed-use-on-bear"
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let system_messages = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_SYSTEM_MESSAGE_V1);
    let mut outcome = None;
    let mut observed_feedback = None;
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::Appearance(delta) => apply_appearance(state, delta.value),
                SessionEvent::ChatLine(delta) => {
                    let feedback = match delta.value {
                        oteryn_dev_client::ChatLine::System { text } if system_messages => {
                            Some(text)
                        }
                        oteryn_dev_client::ChatLine::Private { speaker_name, text }
                            if !system_messages && speaker_name == "Game" =>
                        {
                            Some(text)
                        }
                        _ => None,
                    };
                    if let Some(text) = feedback {
                        let observed = taming_feedback(&text);
                        observed_feedback = Some(text);
                        if let Some(observed) = observed
                            && outcome.replace(observed).is_some()
                        {
                            return Err(
                                "step=taming-attempt observed=multiple-outcome-messages".into()
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        let unlocked = state.catalogue.mounts.iter().any(|mount| mount == WAR_BEAR);
        let removed = client
            .world_entities()
            .is_some_and(|entities| entities.get(&target).is_none());
        if let Some((success, expect_removed, consumed)) = outcome
            && (unlocked, removed, carried(client, reference)?)
                == (success, expect_removed, funded - u64::from(consumed))
        {
            if success {
                row.pass(
                    "taming-attempt",
                    format!(
                        "war-bear-unlocked attempt=1 sequence={}",
                        reply.server_sequence
                    ),
                );
                return Ok(funded - 1);
            }
            return Err(format!("step=taming-attempt observed=first-use-authored-miss feedback={observed_feedback:?} removed={removed} consumed={consumed} unlocked=false").into());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=taming-attempt need=matching-feedback-unlock-bear-and-slingshot-outcome observed=system-messages-{system_messages}-feedback-{observed_feedback:?}-outcome-{outcome:?}-before-count-{funded}-unlock-{unlocked}-removed-{removed}-count-{}", carried(client, reference)?).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_S06_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    character_prep::declare_seeded(
        "S06.full",
        "slingshots,soil-guardian-outfit-parts,premium",
        "appearance and taming actions",
    );
    let mut client = connect_character(clients, 0xb6, 0).await?;
    for (item, count) in [
        (SLINGSHOT, SLINGSHOTS),
        (OUTFIT_PARTS[0], 1),
        (OUTFIT_PARTS[1], 1),
    ] {
        character_prep::grant_item(&client, item, count).await?;
    }
    let original = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=fixture observed=logout-refused".into());
    }
    wait_character_released(clients, 0).await?;
    client = connect_character(clients, 0xb6, 0).await?;
    if client.join_snapshot().game_session_id == original
        || carried(
            &client,
            provisioning::item_reference(&provisioning::item_index()?, SLINGSHOT)?,
        )? != u64::from(SLINGSHOTS)
    {
        return Err("step=fixture need=fresh-admission-with-exact-slingshot-count".into());
    }
    if !client
        .selected_capabilities()
        .contains(&appearance::CAPABILITY_APPEARANCE_V1)
    {
        return Err(
            "step=admission observed=appearance-capability-not-selected need=capability-25".into(),
        );
    }
    let mut state = snapshot(&mut client)?;
    let selected = outfit_and_refusal(row, &mut client, &mut state).await?;
    // The Premium fixture populates the active account view, which logout releases.
    character_prep::grant_premium(&client)
        .await
        .map_err(|error| format!("step=premium-prerequisite observed={error}"))?;
    let earned_outfit = earn_addon(row, &mut client, &mut state).await?;
    let remaining_slingshots = tame_war_bear(row, &mut client, clients, &mut state).await?;

    let mount = WAR_BEAR.to_owned();
    let mounted = appearance::Selection {
        outfit_key: earned_outfit,
        colours: selected.colours,
        addons: 1,
        mount_key: Some(mount.clone()),
    };
    if state
        .catalogue
        .selected
        .as_ref()
        .is_some_and(|s| s.selection == mounted)
    {
        return Err("step=mount-select need=pre-state-different-from-mounted-selection".into());
    }
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
    let previous = client.join_snapshot().game_session_id;
    let logout_deadline = tokio::time::Instant::now() + Duration::from_secs(65);
    while client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight)
    {
        if tokio::time::Instant::now() >= logout_deadline {
            return Err("step=fresh-admission need=wire-combat-state-cleared-before-logout".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=fresh-admission observed=logout-{logout:?}").into());
    }
    wait_character_released(clients, 0).await?;
    let mut fresh = connect_character(clients, 0xb7, 0).await?;
    let persisted = snapshot(&mut fresh)?;
    let persisted_binding = own_binding(&fresh, &persisted)?;
    let item_index = provisioning::item_index()?;
    if carried(
        &fresh,
        provisioning::item_reference(&item_index, SLINGSHOT)?,
    )? != remaining_slingshots
    {
        return Err("step=fresh-admission need=persisted-slingshot-consumption".into());
    }
    for item in OUTFIT_PARTS {
        if carried(&fresh, provisioning::item_reference(&item_index, item)?)? != 0 {
            return Err(
                format!("step=fresh-admission item={item} need=persisted-consumption").into(),
            );
        }
    }
    if fresh.join_snapshot().game_session_id == previous
        || persisted.catalogue.selected.as_ref().map(|s| &s.selection) != Some(&mounted)
        || !persisted.catalogue.mounts.contains(&mount)
        || !persisted.catalogue.outfits.iter().any(|outfit| {
            outfit.key == mounted.outfit_key && outfit.permitted_addons == mounted.addons
        })
        || persisted.catalogue.root_revision < committed
        || persisted.catalogue.selected.as_ref().map(|s| s.revision) != Some(selected_revision)
        || !look_matches(&persisted, &mounted, persisted_binding)
        || persisted_binding.root_revision != persisted.catalogue.root_revision
        || persisted_binding.selection_revision != Some(selected_revision)
    {
        let option = persisted
            .catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.key == mounted.outfit_key);
        return Err(format!(
            "step=fresh-admission observed=new-session-{}-selected-{:?}-outfit-{:?}-mount-owned-{}-catalogue-root-{}-binding-root-{}-binding-selection-{:?}-look-{:?} need=mount-{mount}-matching-root-at-least-{committed}-selection-{selected_revision}",
            fresh.join_snapshot().game_session_id != previous,
            persisted.catalogue.selected,
            option,
            persisted.catalogue.mounts.contains(&mount),
            persisted.catalogue.root_revision,
            persisted_binding.root_revision,
            persisted_binding.selection_revision,
            persisted_binding.look,
        ).into());
    }
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=fresh-admission observed=final-logout-refused".into());
    }
    wait_character_released(clients, 0).await?;
    row.pass(
        "fresh-admission",
        format!("selected-outfit-addon-mount-and-unlock-persisted root_revision={} selected_revision={selected_revision}", persisted.catalogue.root_revision),
    );
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
    fn taming_requires_each_donor_outcome_to_match_its_effects() {
        for (feedback, expected) in [
            ("You have tamed the war bear.", (true, true, true)),
            ("The bear ran away.", (false, true, false)),
            ("Oh no! The slingshot broke.", (false, false, true)),
            (
                "The bear is trying to hit you with its claws.",
                (false, false, false),
            ),
        ] {
            assert_eq!(taming_feedback(feedback), Some(expected), "{feedback}");
        }
        assert_eq!(taming_feedback("Unrelated system feedback"), None);
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
