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

type ViewsDelta = oteryn_dev_client::native_session::AppliedDelta<tree::ContainerViews>;

#[derive(Default)]
struct TransferEvidence {
    views: Option<ViewsDelta>,
    inventory: bool,
}

impl TransferEvidence {
    fn observe(
        &mut self,
        event: SessionEvent,
        before: u64,
        after_sequence: u64,
        expected: Option<(NonZeroU32, u64)>,
    ) -> TestResult {
        match event {
            SessionEvent::GatedSnapshotReplaced(replacement)
                if matches!(
                    replacement.value.domain_id,
                    tree::STATE_DOMAIN_CONTAINER_VIEWS
                        | oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                ) =>
            {
                return Err("replacement snapshot cannot prove I18b delta".into());
            }
            SessionEvent::ContainerViews(delta) => {
                if delta.base_revision != before
                    || delta.new_revision <= before
                    || delta.server_sequence <= after_sequence
                    || self.views.is_some()
                {
                    return Err(format!("unexpected I18b container views delta {delta:?}").into());
                }
                self.views = Some(delta);
            }
            SessionEvent::Inventory(delta) if delta.server_sequence > after_sequence => {
                if let Some((rope, count)) = expected {
                    let observed: u64 = delta
                        .value
                        .entries
                        .iter()
                        .filter(|entry| entry.item_definition_ref == rope)
                        .map(|entry| u64::from(entry.count.get()))
                        .sum();
                    if observed != count {
                        return Err(format!(
                            "I18b inventory delta expected {count} rope, observed {observed}"
                        )
                        .into());
                    }
                    self.inventory = true;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

async fn fresh_views(
    client: &mut DevClientSession,
    before: u64,
    after_sequence: u64,
    expected: Option<(NonZeroU32, u64)>,
) -> TestResult<ViewsDelta> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut observed = TransferEvidence::default();
    loop {
        for event in client.take_events() {
            observed.observe(event, before, after_sequence, expected)?;
        }
        if (expected.is_none() || observed.inventory)
            && let Some(delta) = observed.views.take()
        {
            if view_revision(client)? != delta.new_revision
                || client.session_mut().container_views() != Some(&delta.value)
                || expected.is_some_and(|(rope, count)| quantity(client, rope).ok() != Some(count))
            {
                return Err("I18b cached state diverged from the observed deltas".into());
            }
            return Ok(delta);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("required I18b container views or inventory delta absent".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

fn action_boundary(client: &mut DevClientSession) -> TestResult<(u64, u64)> {
    client.take_events();
    Ok((view_revision(client)?, client.last_server_sequence()))
}

async fn browse_open(
    client: &mut DevClientSession,
    at: WorldTilePosition,
) -> TestResult<(tree::ContainerView, ViewsDelta)> {
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
    let (before, sequence) = action_boundary(client)?;
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
    let delta = fresh_views(client, before, sequence, None).await?;
    let view = delta
        .value
        .views
        .iter()
        .find(|view| view.browse.is_some_and(|page| page.position == at))
        .cloned()
        .ok_or("step=browse-open observed=own-tile-view-absent")?;
    Ok((view, delta))
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
) -> TestResult<(ItemEntry, ViewsDelta)> {
    let (before, sequence) = action_boundary(client)?;
    let entry = carried_rope(client, rope)?;
    move_rope(
        client,
        entry,
        ItemMoveDestination::Ground(at),
        ItemMoveOutcome::Moved,
    )
    .await?;
    let delta = fresh_views(client, before, sequence, Some((rope, 0))).await?;
    let view = delta
        .value
        .views
        .iter()
        .find(|view| view.view_id == field.view_id)
        .ok_or("browse view disappeared after ground drop")?;
    let item = rope_on_tile(view, at, rope, 1)?.ok_or("dropped rope absent")?;
    Ok((item, delta))
}

async fn pick_up_rope(
    client: &mut DevClientSession,
    field: &tree::ContainerView,
    at: WorldTilePosition,
    rope: NonZeroU32,
    entry: ItemEntry,
) -> TestResult<ViewsDelta> {
    if quantity(client, rope)? != 0 {
        return Err("step=pickup-prestate observed=rope-already-carried".into());
    }
    let (before, sequence) = action_boundary(client)?;
    move_rope(
        client,
        entry,
        ItemMoveDestination::MainBackpack,
        ItemMoveOutcome::Moved,
    )
    .await?;
    let delta = fresh_views(client, before, sequence, Some((rope, 1))).await?;
    let view = delta
        .value
        .views
        .iter()
        .find(|view| view.view_id == field.view_id)
        .ok_or("browse view disappeared after pickup")?;
    rope_on_tile(view, at, rope, 0)?;
    Ok(delta)
}

// Store the untouched server domain payload. Session metadata stays outside the wire bytes.
fn capture_view(
    client: &mut DevClientSession,
    phase: &str,
    delta: &ViewsDelta,
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
    let sequence = delta.server_sequence;
    let selected = client.selected_capabilities();
    let house = u8::from(selected.contains(&tree::CAPABILITY_HOUSE_CONTAINER_CAPACITY_V2));
    let appearances = u8::from(
        selected.contains(&oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_APPEARANCE_V1),
    );
    let snapshot = client
        .session_mut()
        .gated_snapshot(tree::STATE_DOMAIN_CONTAINER_VIEWS)
        .ok_or("capture container views absent")?;
    if snapshot.revision != delta.new_revision {
        return Err("capture payload revision diverged from I18b delta".into());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("i18b-{phase}.bin")))?;
    writeln!(
        file,
        "{run} {id} {sequence} {} {} {view_id} {rope} {house} {appearances}",
        delta.base_revision, delta.new_revision
    )?;
    file.write_all(&snapshot.payload)?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct RefusalState {
    inventory: oteryn_dev_client::native_session::CharacterInventory,
    views: tree::ContainerViews,
}

impl RefusalState {
    fn read(client: &mut DevClientSession) -> TestResult<Self> {
        Ok(Self {
            inventory: client
                .inventory()
                .cloned()
                .ok_or("refusal inventory absent")?,
            views: client
                .session_mut()
                .container_views()
                .cloned()
                .ok_or("refusal views absent")?,
        })
    }

    fn verify(&self, after: &Self, events: Vec<SessionEvent>) -> TestResult {
        if self != after
            || events.into_iter().any(|event| match event {
                SessionEvent::Inventory(delta) => delta.value != self.inventory,
                SessionEvent::ContainerViews(delta) => delta.value != self.views,
                SessionEvent::GatedSnapshotReplaced(delta) => matches!(
                    delta.value.domain_id,
                    tree::STATE_DOMAIN_CONTAINER_VIEWS
                        | oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
                ),
                _ => false,
            })
        {
            return Err("step=locked-browse-refusal observed=state-changed".into());
        }
        Ok(())
    }
}

async fn refuse_field_move(
    client: &mut DevClientSession,
    entry: ItemEntry,
    destination: ItemMoveDestination,
) -> TestResult {
    use oteryn_protocol_oteryn::item_look as look;

    let before = RefusalState::read(client)?;
    client.take_events();
    move_rope(client, entry, destination, ItemMoveOutcome::NotSupported).await?;
    // A subsequent read-only command consumes queued pushes from the refused move.
    let payload = look::encode_item_look_intent(look::ItemLookIntent {
        target: look::ItemLookTarget::Item(entry.handle),
    })
    .map_err(|error| format!("refusal Look request: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            look::ITEM_LOOK_CAPABILITY_ID.ok_or("Look capability absent")?,
            look::ITEM_LOOK_COMMAND_ID.ok_or("Look command absent")?,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || !matches!(
            look::decode_item_look_result(&reply.payload),
            Ok(look::ItemLookResult::Text(_))
        )
    {
        return Err(format!("refusal Look barrier failed: {:?}", reply.status).into());
    }
    before.verify(&RefusalState::read(client)?, client.take_events())
}

fn reorder_destination(
    view: &tree::ContainerView,
    rope: ItemEntry,
    other: ItemEntry,
) -> TestResult<ItemMoveDestination> {
    let source = view
        .entries
        .iter()
        .position(|entry| *entry == rope)
        .ok_or("reorder source absent")?;
    let target = view
        .entries
        .iter()
        .position(|entry| *entry == other)
        .ok_or("occupied reorder target absent")?;
    if source == target {
        return Err("reorder destination is the source index".into());
    }
    Ok(ItemMoveDestination::Container(
        view.container_handle,
        Some(u32::try_from(target)?),
    ))
}

fn capture_drag_inventory(client: &DevClientSession, delta: &ViewsDelta) -> TestResult {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        use std::io::Write;
        let inventory = client.inventory().ok_or("capture inventory absent")?;
        let bytes = oteryn_protocol_oteryn::item_view::encode_character_inventory_with_appearances(
            inventory, true, true,
        )
        .map_err(|error| format!("capture inventory encoding: {error:?}"))?;
        let root = Path::new(&root);
        let run = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("capture run name")?;
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("i18b-drag-inventory.bin"))?;
        writeln!(file, "{run} {session} {}", delta.server_sequence)?;
        file.write_all(&bytes)?;
    }
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

    let (field, opened) = browse_open(&mut client, at).await?;
    rope_on_tile(&field, at, rope, 0)?;
    capture_view(&mut client, "before-drop", &opened, field.view_id, rope)?;
    // Global: https://tibia.fandom.com/wiki/Browse_Field (drag out only).
    // The physical tile remains the deposit target.
    let entry = carried_rope(&client, rope)?;
    refuse_field_move(
        &mut client,
        entry,
        ItemMoveDestination::Container(field.container_handle, None),
    )
    .await?;
    let (dropped, drop_delta) = drop_rope(&mut client, &field, at, rope).await?;
    capture_view(&mut client, "after-drop", &drop_delta, field.view_id, rope)?;
    capture_drag_inventory(&client, &drop_delta)?;
    let pickup_delta = pick_up_rope(&mut client, &field, at, rope, dropped).await?;
    capture_view(
        &mut client,
        "after-pickup",
        &pickup_delta,
        field.view_id,
        rope,
    )?;

    let (before, sequence) = action_boundary(&mut client)?;
    let outcome = client
        .session_mut()
        .container_view(tree::ContainerViewIntent::Close {
            view_id: field.view_id,
        })
        .await?;
    if outcome != Some(tree::ContainerViewOutcome::Closed)
        || fresh_views(&mut client, before, sequence, None)
            .await?
            .value
            .views
            .iter()
            .any(|view| view.view_id == field.view_id)
    {
        return Err(format!("step=browse-close observed={outcome:?}").into());
    }
    let (field, _) = browse_open(&mut client, at).await?;
    rope_on_tile(&field, at, rope, 0)?;
    let (_, rope_on_ground) = drop_rope(&mut client, &field, at, rope).await?;
    let shovel = provisioning::item_reference(&provisioning::item_index()?, 3457)?;
    let one_item = rope_on_ground
        .value
        .views
        .iter()
        .find(|view| view.view_id == field.view_id)
        .ok_or("pre-reorder field absent")?;
    rope_on_tile(one_item, at, shovel, 0)?;
    let (shovel_on_tile, two_items) = drop_rope(&mut client, &field, at, shovel).await?;
    let populated = two_items
        .value
        .views
        .iter()
        .find(|view| view.view_id == field.view_id)
        .ok_or("reorder field absent")?;
    if !client.selected_capabilities().contains(&510) {
        return Err("I18b reorder requires explicit slot capability".into());
    }
    let current_rope = rope_on_tile(populated, at, rope, 1)?.ok_or("reorder rope absent")?;
    let destination = reorder_destination(populated, current_rope, shovel_on_tile)?;
    refuse_field_move(&mut client, current_rope, destination).await?;
    pick_up_rope(&mut client, populated, at, shovel, shovel_on_tile).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_world(clients, 0xf2).await?;
    let ground_session = client.join_snapshot().game_session_id;
    if ground_session == initial_session || quantity(&client, rope)? != 0 {
        return Err("step=ground-relog observed=unexpected-session-or-carried-rope".into());
    }
    let (field, _) = browse_open(&mut client, at).await?;
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
    let (field, _) = browse_open(&mut client, at).await?;
    rope_on_tile(&field, at, rope, 0)?;
    logout(&mut client).await?;
    Ok("starter-rope;ground-drop=2;browse-pickup=2;virtual-deposit-refused;occupied-slot-reorder-refused;refusals-preserve-full-inventory-and-views;ground-relog=backpack-0,tile-1;pickup-relog=backpack-1,tile-0".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_dev_client::native_session::{AppliedDelta, GatedSnapshot};

    #[test]
    fn i18b_refusal_rejects_collateral_inventory_and_view_changes() -> TestResult {
        let rope = ItemEntry {
            appearance_id: 0,
            handle: 1.try_into()?,
            item_definition_ref: 1.try_into()?,
            count: 1.try_into()?,
            sub_type: 0,
        };
        let other = ItemEntry {
            handle: 2.try_into()?,
            item_definition_ref: 2.try_into()?,
            ..rope
        };
        let view = tree::ContainerView {
            view_id: 1,
            container_handle: 3.try_into()?,
            parent_handle: None,
            capacity: 30,
            entries: vec![rope, other],
            browse: None,
        };
        let before = RefusalState {
            inventory: oteryn_dev_client::native_session::CharacterInventory {
                entries: vec![rope, other],
                ..Default::default()
            },
            views: tree::ContainerViews {
                views: vec![view.clone()],
            },
        };
        assert_eq!(
            reorder_destination(&view, rope, other)?,
            ItemMoveDestination::Container(view.container_handle, Some(1))
        );
        assert!(reorder_destination(&view, rope, rope).is_err());
        before.verify(&before, Vec::new())?;
        let mut changed_inventory = before.inventory.clone();
        changed_inventory.entries.remove(1);
        let restored = SessionEvent::Inventory(AppliedDelta {
            server_sequence: 20,
            base_revision: 1,
            new_revision: 2,
            value: changed_inventory,
        });
        assert!(before.verify(&before, vec![restored]).is_err());
        let mut changed_views = before.views.clone();
        changed_views.views[0].entries.swap(0, 1);
        let after = RefusalState {
            inventory: before.inventory.clone(),
            views: changed_views,
        };
        assert!(before.verify(&after, Vec::new()).is_err());
        Ok(())
    }

    #[test]
    fn i18b_replacement_with_matching_typed_projection_is_rejected() -> TestResult {
        let typed = SessionEvent::ContainerViews(AppliedDelta {
            server_sequence: 21,
            base_revision: 4,
            new_revision: 5,
            value: tree::ContainerViews::default(),
        });
        let replacement = SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 21,
            base_revision: 4,
            new_revision: 5,
            value: GatedSnapshot {
                domain_id: tree::STATE_DOMAIN_CONTAINER_VIEWS,
                snapshot_type: 1,
                revision: 5,
                payload: Vec::new(),
            },
        });
        let mut real = TransferEvidence::default();
        real.observe(typed.clone(), 4, 20, None)?;
        assert_eq!(real.views.ok_or("real delta missing")?.server_sequence, 21);
        let mut replaced = TransferEvidence::default();
        let result = [replacement, typed]
            .into_iter()
            .try_for_each(|event| replaced.observe(event, 4, 20, None));
        assert!(result.is_err());
        assert!(replaced.views.is_none());
        Ok(())
    }
}
