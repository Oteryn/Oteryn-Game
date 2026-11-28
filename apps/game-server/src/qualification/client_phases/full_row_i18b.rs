//! I18b: move the starter rope between the backpack and the actor's browsed tile.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::BTreeMap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I18b.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantities(client: &DevClientSession) -> TestResult<BTreeMap<u32, u64>> {
    let mut values = BTreeMap::new();
    for item in &client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
    {
        *values.entry(item.item_definition_ref.get()).or_default() += u64::from(item.count.get());
    }
    Ok(values)
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(*quantities(client)?.get(&reference.get()).unwrap_or(&0))
}

async fn wait_quantity(
    client: &mut DevClientSession,
    reference: std::num::NonZeroU32,
    expected: u64,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let SessionEvent::Inventory(delta) = event
                && quantity(client, reference)? == expected
            {
                return Ok(delta.server_sequence);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "inventory delta timeout definition={reference} expected={expected} observed={}",
                quantity(client, reference)?
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("logout refused".into());
    }
    Ok(())
}

use oteryn_dev_client::native_session::item_movement::MoveRequest;
use oteryn_protocol_oteryn::{
    CommandStatus, browse_field as browse, container_tree as tree,
    item_view::{ItemEntry, ItemMoveDestination, ItemMoveOutcome, WorldTilePosition},
};
use std::num::NonZeroU32;

fn view_revision(client: &mut DevClientSession) -> TestResult<u64> {
    Ok(client
        .session_mut()
        .gated_snapshot(tree::STATE_DOMAIN_CONTAINER_VIEWS)
        .ok_or("container views snapshot absent")?
        .revision)
}

async fn fresh_views(
    client: &mut DevClientSession,
    before: u64,
) -> TestResult<tree::ContainerViews> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while view_revision(client)? <= before {
        if tokio::time::Instant::now() >= deadline {
            return Err("fresh container views delta absent".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
        client.take_events();
    }
    client
        .session_mut()
        .container_views()
        .cloned()
        .ok_or_else(|| "container views absent".into())
}

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<tree::ContainerView> {
    if client
        .session_mut()
        .container_views()
        .ok_or("container views absent before browse")?
        .views
        .iter()
        .any(|view| view.browse.is_some_and(|page| page.position == at))
    {
        return Err("step=browse-prestate observed=own-tile-view-already-open".into());
    }
    let before = view_revision(client)?;
    let payload =
        browse::encode_browse_field_intent(&browse::BrowseFieldIntent::Open { position: at })
            .map_err(|error| format!("step=browse-encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            browse::CAPABILITY_BROWSE_FIELD_V1,
            browse::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=browse-result observed={error:?}"))?;
    if reply.status != CommandStatus::Accepted || outcome != tree::ContainerViewOutcome::Opened {
        return Err(format!("step=browse-open observed={:?}/{outcome:?}", reply.status).into());
    }
    fresh_views(client, before)
        .await?
        .views
        .into_iter()
        .find(|view| view.browse.is_some_and(|page| page.position == at))
        .ok_or_else(|| "step=browse-open observed=own-tile-view-absent".into())
}

fn rope_on_tile(
    view: &tree::ContainerView,
    at: WorldTilePosition,
    rope: NonZeroU32,
    expected: usize,
) -> TestResult<Option<ItemEntry>> {
    let page = view.browse.ok_or("expected browse metadata")?;
    if page.position != at || page.index != 0 || usize::from(page.total) != view.entries.len() {
        return Err(format!("expected complete own-tile browse, observed {page:?}").into());
    }
    let mut matches = view
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == rope);
    let first = matches.next().copied();
    if usize::from(first.is_some()) != expected
        || first.is_some_and(|entry| entry.count.get() != 1)
        || matches.next().is_some()
    {
        return Err(format!("step=browse-rope observed={first:?};expected={expected}").into());
    }
    Ok(first)
}

async fn move_rope(
    client: &mut DevClientSession,
    entry: ItemEntry,
    destination: ItemMoveDestination,
    expected: ItemMoveOutcome,
) -> TestResult {
    let result = client
        .session_mut()
        .move_item_counted(MoveRequest {
            source: entry.handle,
            destination,
            count: None,
        })
        .await
        .map_err(|error| format!("step=move destination={destination:?} error={error}"))?;
    if result.outcome != expected {
        return Err(format!(
            "step=move destination={destination:?} observed={:?};expected={expected:?}",
            result.outcome
        )
        .into());
    }
    Ok(())
}

fn carried_rope(client: &DevClientSession, rope: NonZeroU32) -> TestResult<ItemEntry> {
    if quantity(client, rope)? != 1 {
        return Err("expected exactly one starter rope in the main backpack".into());
    }
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == rope)
        })
        .copied()
        .ok_or_else(|| "starter rope absent".into())
}

async fn drop_rope(
    client: &mut DevClientSession,
    field: &tree::ContainerView,
    at: WorldTilePosition,
    rope: NonZeroU32,
) -> TestResult<ItemEntry> {
    let before = view_revision(client)?;
    let entry = carried_rope(client, rope)?;
    client.take_events();
    move_rope(
        client,
        entry,
        ItemMoveDestination::Ground(at),
        ItemMoveOutcome::Moved,
    )
    .await?;
    wait_quantity(client, rope, 0).await?;
    let views = fresh_views(client, before).await?;
    let view = views
        .views
        .iter()
        .find(|view| view.view_id == field.view_id)
        .ok_or("browse view disappeared after ground drop")?;
    rope_on_tile(view, at, rope, 1)?.ok_or_else(|| "dropped rope absent".into())
}

async fn pick_up_rope(
    client: &mut DevClientSession,
    field: &tree::ContainerView,
    at: WorldTilePosition,
    rope: NonZeroU32,
    entry: ItemEntry,
) -> TestResult {
    if quantity(client, rope)? != 0 {
        return Err("step=pickup-prestate observed=rope-already-carried".into());
    }
    let before = view_revision(client)?;
    client.take_events();
    move_rope(
        client,
        entry,
        ItemMoveDestination::MainBackpack,
        ItemMoveOutcome::Moved,
    )
    .await?;
    wait_quantity(client, rope, 1).await?;
    let views = fresh_views(client, before).await?;
    let view = views
        .views
        .iter()
        .find(|view| view.view_id == field.view_id)
        .ok_or("browse view disappeared after pickup")?;
    rope_on_tile(view, at, rope, 0)?;
    Ok(())
}

// Store the untouched server domain payload. Session metadata stays outside the wire bytes.
fn capture_view(
    client: &mut DevClientSession,
    phase: &str,
    view_id: u8,
    rope: NonZeroU32,
) -> TestResult {
    use std::io::Write;

    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let run = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("capture run name")?;
    let id = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let sequence = client.last_server_sequence();
    let selected = client.selected_capabilities();
    let house = u8::from(selected.contains(&tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2));
    let appearances = u8::from(
        selected.contains(&oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_APPEARANCE_V1),
    );
    let snapshot = client
        .session_mut()
        .gated_snapshot(tree::STATE_DOMAIN_CONTAINER_VIEWS)
        .ok_or("capture container views absent")?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("i18b-{phase}.bin")))?;
    writeln!(
        file,
        "{run} {id} {sequence} {} {view_id} {rope} {house} {appearances}",
        snapshot.revision
    )?;
    file.write_all(&snapshot.payload)?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let rope = provisioning::item_reference(&provisioning::item_index()?, 3003)?;
    let mut client = connect_world(clients, 0xf2).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let position = client.world_spatial().actor_position;
    let at = WorldTilePosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };

    let field = browse_open(&mut client, at).await?;
    rope_on_tile(&field, at, rope, 0)?;
    // Global: https://tibia.fandom.com/wiki/Browse_Field (drag out only).
    // The physical tile remains the deposit target.
    let entry = carried_rope(&client, rope)?;
    move_rope(
        &mut client,
        entry,
        ItemMoveDestination::Container(field.container_handle, None),
        ItemMoveOutcome::NotSupported,
    )
    .await?;
    if carried_rope(&client, rope)? != entry
        || !client
            .session_mut()
            .container_views()
            .is_some_and(|views| views.views.iter().any(|view| view == &field))
    {
        return Err("step=locked-browse-refusal observed=state-changed".into());
    }
    capture_view(&mut client, "before-drop", field.view_id, rope)?;
    let dropped = drop_rope(&mut client, &field, at, rope).await?;
    capture_view(&mut client, "after-drop", field.view_id, rope)?;
    pick_up_rope(&mut client, &field, at, rope, dropped).await?;
    capture_view(&mut client, "after-pickup", field.view_id, rope)?;

    let before = view_revision(&mut client)?;
    let outcome = client
        .session_mut()
        .container_view(tree::ContainerViewIntent::Close {
            view_id: field.view_id,
        })
        .await?;
    if outcome != Some(tree::ContainerViewOutcome::Closed)
        || fresh_views(&mut client, before)
            .await?
            .views
            .iter()
            .any(|view| view.view_id == field.view_id)
    {
        return Err(format!("step=browse-close observed={outcome:?}").into());
    }
    let field = browse_open(&mut client, at).await?;
    rope_on_tile(&field, at, rope, 0)?;
    drop_rope(&mut client, &field, at, rope).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_world(clients, 0xf2).await?;
    let ground_session = client.join_snapshot().game_session_id;
    if ground_session == initial_session || quantity(&client, rope)? != 0 {
        return Err("step=ground-relog observed=unexpected-session-or-carried-rope".into());
    }
    let field = browse_open(&mut client, at).await?;
    let persisted = rope_on_tile(&field, at, rope, 1)?.ok_or("ground rope lost after relog")?;
    pick_up_rope(&mut client, &field, at, rope, persisted).await?;

    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0xf2).await?;
    let restored_session = client.join_snapshot().game_session_id;
    if restored_session == ground_session
        || restored_session == initial_session
        || quantity(&client, rope)? != 1
    {
        return Err("step=fresh-admission observed=unexpected-session-or-rope-count".into());
    }
    let field = browse_open(&mut client, at).await?;
    rope_on_tile(&field, at, rope, 0)?;
    logout(&mut client).await?;
    Ok("starter-rope;ground-drop=2;browse-pickup=2;virtual-destination-refused;ground-relog=backpack-0,tile-1;pickup-relog=backpack-1,tile-0".to_owned())
}
