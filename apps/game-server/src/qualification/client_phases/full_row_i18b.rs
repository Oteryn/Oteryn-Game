//! I18b: browse field, move items out of and into the browse field.
//! Drops a backpack item on the character's tile, browses the tile and moves the listed entry
//! into the main backpack (Moved, backpack credit, the entry gone from a fresh browse), drops it
//! on the tile again and browses once more (the entry is listed again), and finally picks it up
//! from the view. The restored backpack count is re-read after logout and a fresh admission.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use std::collections::BTreeMap;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(240), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=I18B.full status={status} seconds={:.3} evidence={detail}",
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

use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::browse_field as browse;
use oteryn_protocol_oteryn::container_tree as tree;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

fn view_of(payload: &[u8]) -> TestResult<tree::ContainerViews> {
    tree::decode_container_views_for(payload, true)
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
    let view = view_of(&delta.payload)?
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
    Ok((outcome, view_of(&delta.payload)?.views))
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

/// Drops the first backpack entry on the actor's own tile and returns its definition and count.
async fn drop_first(
    client: &mut DevClientSession,
    prefer: Option<std::num::NonZeroU32>,
) -> TestResult<(std::num::NonZeroU32, std::num::NonZeroU32)> {
    let item = client
        .inventory()
        .and_then(|inventory| {
            prefer
                .and_then(|want| {
                    inventory
                        .entries
                        .iter()
                        .find(|item| item.item_definition_ref == want)
                })
                .or(inventory.entries.first())
        })
        .copied()
        .ok_or("step=drop observed=no-backpack-item;needed:player-acquired-item")?;
    let at = ground(client);
    let before = quantity(client, item.item_definition_ref)?;
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
    while quantity(client, item.item_definition_ref)? >= before {
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("step=drop observed=no-inventory-debit".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok((item.item_definition_ref, item.count))
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
    let mut client = connect_world(clients, 0xf2).await?;
    let at = here(&client);
    let (definition, count) = drop_first(&mut client, None).await?;
    let (outcome, view) = browse_open(&mut client, at).await?;
    let view =
        view.ok_or_else(|| format!("step=browse observed={outcome:?};needed=Opened-with-view"))?;
    let entry = has(&view, definition).ok_or("step=browse observed=dropped-item-not-listed")?;
    take_entry(&mut client, &entry).await?;
    let (_, after) = browse_open(&mut client, at).await?;
    if after
        .as_ref()
        .and_then(|view| has(view, definition))
        .is_some()
    {
        return Err("step=browse-after-take observed=entry-still-listed".into());
    }
    let (_, _) = view_intent(
        &mut client,
        tree::ContainerViewIntent::Close {
            view_id: view.view_id,
        },
    )
    .await?;
    let (definition_again, _) = drop_first(&mut client, Some(definition)).await?;
    let (_, again) = browse_open(&mut client, at).await?;
    let again = again.ok_or("step=browse-again observed=no-view")?;
    let entry = has(&again, definition_again)
        .ok_or("step=browse-again observed=re-dropped-item-not-listed")?;
    take_entry(&mut client, &entry).await?;
    let held = quantity(&client, definition)?;
    let mut client = {
        logout(&mut client).await?;
        connect_world(clients, 0xf2).await?
    };
    if quantity(&client, definition)? != held || held < u64::from(count.get()) {
        return Err("step=fresh-admission observed=backpack-count-changed".into());
    }
    logout(&mut client).await?;
    Ok("moved-out-and-in-of-browse-field-and-durable".to_owned())
}
