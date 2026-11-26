//! I18c: browse field, move up from a ground container.
//! Drops a fixture-supplied bag (2853) on the character's tile, browses the tile, opens it from the
//! browse view in place and expects the bag view's parent to be the browse view's container,
//! then sends `Up` and expects the browse view of that tile back. Pickup is verified after relog.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I18c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory snapshot absent")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == reference)
        .map(|item| u64::from(item.count.get()))
        .sum())
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

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    provisioning::item_reference(&index, id)
}

use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::browse_field as browse;
use oteryn_protocol_oteryn::container_tree as tree;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

fn view_of(selected: &[u32], payload: &[u8]) -> TestResult<tree::ContainerViews> {
    selected_container_views(selected, payload)
        .map_err(|error| format!("step=views-decode observed={error:?}").into())
}

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<(tree::ContainerViewOutcome, Option<tree::ContainerView>)> {
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
        .map_err(|error| format!("step=browse-reply observed={error:?}"))?;
    if outcome != tree::ContainerViewOutcome::Opened {
        return Ok((outcome, None));
    }
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    let view = view_of(client.selected_capabilities(), &delta.payload)?
        .views
        .into_iter()
        .find(|view| view.browse.is_some());
    Ok((outcome, view))
}

async fn view_intent(
    client: &mut DevClientSession,
    intent: tree::ContainerViewIntent,
) -> TestResult<(tree::ContainerViewOutcome, Vec<tree::ContainerView>)> {
    let payload = tree::encode_container_view_intent(&intent)
        .map_err(|error| format!("step=view-encode observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            tree::CAPABILITY_CONTAINER_TREE_V1,
            tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &payload,
        )
        .await?;
    let outcome = tree::decode_container_view_result(&reply.payload)
        .map_err(|error| format!("step=view-reply observed={error:?}"))?;
    let delta = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    Ok((
        outcome,
        view_of(client.selected_capabilities(), &delta.payload)?.views,
    ))
}

fn here(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: -at.floor,
    }
}

fn ground(client: &DevClientSession) -> WorldTilePosition {
    let at = client.world_spatial().actor_position;
    WorldTilePosition {
        x: at.x,
        y: at.y,
        floor: at.floor,
    }
}

async fn drop_bag(client: &mut DevClientSession, bag: std::num::NonZeroU32) -> TestResult {
    let item = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == bag)
        })
        .copied()
        .ok_or("step=drop observed=fixture-bag-absent")?;
    let at = ground(client);
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::Ground(at),
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=drop observed={:?};needed=Moved", moved.outcome).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantity(client, bag)? != 0 {
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("step=drop observed=no-inventory-debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

/// Moves a browse entry into the main backpack and waits for the inventory credit.
async fn take_entry(
    client: &mut DevClientSession,
    entry: &oteryn_protocol_oteryn::item_view::ItemEntry,
) -> TestResult<()> {
    let before = quantity(client, entry.item_definition_ref)?;
    client.take_events();
    let moved = client
        .move_item(&ItemMoveIntent {
            source: entry.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=take observed={:?};needed=Moved", moved.outcome).into());
    }
    wait_quantity(
        client,
        entry.item_definition_ref,
        before + u64::from(entry.count.get()),
    )
    .await?;
    Ok(())
}

fn has(
    view: &tree::ContainerView,
    definition: std::num::NonZeroU32,
) -> Option<oteryn_protocol_oteryn::item_view::ItemEntry> {
    view.entries
        .iter()
        .find(|entry| entry.item_definition_ref == definition)
        .copied()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_I18C_ACCOUNT").await?;
    let clients = identity.clients(clients);
    let bag = reference(2853)?;
    let mut client = connect_character(&clients, 0xf3, 0).await?;
    if quantity(&client, bag)? != 0 {
        return Err("step=prepare observed=bag-already-present".into());
    }
    character_prep::declare_seeded(
        "I18c.full",
        "bag-2853",
        "bag acquisition is outside browse-field navigation",
    );
    character_prep::grant_item(&client, 2853, 1).await?;
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&clients, 0xf3, 0).await?;
    if client.join_snapshot().game_session_id == session || quantity(&client, bag)? != 1 {
        return Err("step=prepare needed=fresh-session-with-exactly-one-bag".into());
    }
    let at = here(&client);
    drop_bag(&mut client, bag).await?;
    let (outcome, view) = browse_open(&mut client, at).await?;
    let browse_view =
        view.ok_or_else(|| format!("step=browse observed={outcome:?};needed=Opened-with-view"))?;
    if outcome != tree::ContainerViewOutcome::Opened
        || !browse_view.browse.is_some_and(|page| page.position == at)
    {
        return Err("step=browse needed=Opened-view-of-drop-tile".into());
    }
    let entry = has(&browse_view, bag).ok_or("step=browse observed=bag-not-listed")?;
    let (opened, views) = view_intent(
        &mut client,
        tree::ContainerViewIntent::Open {
            handle: entry.handle,
            replace_view: Some(browse_view.view_id),
        },
    )
    .await?;
    let inner = views
        .iter()
        .find(|view| view.container_handle == entry.handle)
        .ok_or_else(|| format!("step=open-bag observed={opened:?};needed=bag-view"))?;
    if opened != tree::ContainerViewOutcome::Opened
        || inner.view_id != browse_view.view_id
        || inner.parent_handle != Some(browse_view.container_handle)
    {
        return Err(format!(
            "step=open-bag observed={opened:?}-parent:{:?};needed=Opened-with-browse-parent",
            inner.parent_handle
        )
        .into());
    }
    let inner_id = inner.view_id;
    let (up, views) = view_intent(
        &mut client,
        tree::ContainerViewIntent::Up { view_id: inner_id },
    )
    .await?;
    let restored = views.iter().find(|view| {
        view.view_id == inner_id
            && view.container_handle == browse_view.container_handle
            && view.browse.is_some_and(|page| page.position == at)
    });
    if up != tree::ContainerViewOutcome::Opened || restored.is_none() {
        return Err(format!(
            "step=up observed={up:?}-views:{};needed=Opened-browse-view",
            views.len()
        )
        .into());
    }
    let restored_entry = restored
        .and_then(|view| has(view, bag))
        .filter(|restored| restored.handle == entry.handle && restored.count == entry.count)
        .ok_or("step=up observed=original-bag-not-listed-in-restored-view")?;
    take_entry(&mut client, &restored_entry).await?;
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&clients, 0xf3, 0).await?;
    if fresh.join_snapshot().game_session_id == session || quantity(&fresh, bag)? != 1 {
        return Err("step=relog needed=fresh-session-with-exactly-one-recovered-bag".into());
    }
    logout(&mut fresh).await?;
    Ok("up-returned-to-browse-view-of-the-tile;recovered-bag-persisted".to_owned())
}
