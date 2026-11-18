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
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
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

use super::full_row_i18a::{
    browse_entry, browse_open, close_browse, drop_first, here, require_browse,
};
use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
use oteryn_protocol_oteryn::container_tree as tree;

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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xf2).await?;
    let at = here(&client);
    let dropped = drop_first(&mut client, None).await?;
    let definition = dropped.item_definition_ref;
    let count = dropped.count;
    let (outcome, view) = browse_open(&mut client, at).await?;
    let view = require_browse(at, outcome, view)?;
    let entry = browse_entry(&view.entries, definition, dropped.handle)
        .ok_or("step=browse observed=dropped-item-not-listed")?;
    take_entry(&mut client, &entry).await?;
    close_browse(&mut client, view.view_id).await?;
    let (outcome, after) = browse_open(&mut client, at).await?;
    let after = require_browse(at, outcome, after)?;
    if browse_entry(&after.entries, definition, dropped.handle).is_some() {
        return Err("step=browse-after-take observed=entry-still-listed".into());
    }
    close_browse(&mut client, after.view_id).await?;
    let dropped_again = drop_first(&mut client, Some(definition)).await?;
    let (outcome, again) = browse_open(&mut client, at).await?;
    let again = require_browse(at, outcome, again)?;
    let entry = browse_entry(&again.entries, definition, dropped_again.handle)
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

pub(super) fn verify_browse_close(
    outcome: tree::ContainerViewOutcome,
    still_open: bool,
) -> TestResult {
    if outcome != tree::ContainerViewOutcome::Closed || still_open {
        return Err(
            format!("step=close-browse observed={outcome:?};view-still-open={still_open}").into(),
        );
    }
    Ok(())
}

#[test]
fn browse_close_requires_closed_reply_and_removed_view() {
    assert!(verify_browse_close(tree::ContainerViewOutcome::Closed, false).is_ok());
    assert!(verify_browse_close(tree::ContainerViewOutcome::Closed, true).is_err());
    assert!(verify_browse_close(tree::ContainerViewOutcome::Stale, false).is_err());
}
