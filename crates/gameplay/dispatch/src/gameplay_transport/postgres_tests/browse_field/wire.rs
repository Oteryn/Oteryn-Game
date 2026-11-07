use super::super::fixture::{Harness, debug, id, join_two};
use super::super::item_look::read_frame;
use super::{Case, FIELD, TestResult};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, LivenessPolicy, serve_admitted,
};
use oteryn_protocol_oteryn::{self as protocol, CommandStatus, MessageType};
use protocol::browse_field::{BrowseFieldIntent, COMMAND_TYPE_BROWSE_FIELD_INTENT, encode_browse_field_intent};
use protocol::container_tree::{
    COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewIntent, ContainerViewOutcome, ContainerViews,
    STATE_DOMAIN_CONTAINER_VIEWS, decode_container_view_result, decode_container_views_for,
    encode_container_view_intent,
};
use protocol::item_view::{
    CharacterInventory, ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome,
    ItemMoveSelection, WorldTilePosition, decode_character_inventory_with_equip_drop,
    decode_item_move_result_with_equip_drop, encode_item_move_intent_for,
};
use protocol::world_spatial::{StepDirection, StepDisposition};
use tokio::io::{AsyncWriteExt, DuplexStream};

const SELECTION: ItemMoveSelection = ItemMoveSelection {
    equip_drop: true,
    container_tree: true,
    world_interaction: false,
};
// The deadline bounds the scenario. Liveness is tested elsewhere and must not
// disconnect a test client while it checks committed PostgreSQL rows.
const LIVENESS: LivenessPolicy = LivenessPolicy {
    interval: std::time::Duration::from_secs(600),
    missed_limit: 3,
};

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    harness: &Harness,
    case: Case,
) -> TestResult<Option<Vec<u8>>> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = serve_admitted(&mut server, admitted, owner, LIVENESS);
    let client_task = async move {
        let inventory = snapshot(&mut client).await?;
        let replay = match case {
            Case::Backpack | Case::Nested => {
                Some(transfers(&mut client, &inventory, harness, case).await?)
            }
            Case::Navigation => {
                navigation(&mut client, &inventory, harness).await?;
                None
            }
            Case::Paging => {
                paging(&mut client, harness).await?;
                None
            }
            Case::Autowalk => {
                autowalk(&mut client, harness).await?;
                None
            }
        };
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(replay)
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(60), join_two(serving, client_task),
    ).await?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)), "{end:?}");
    result
}

async fn transfers(
    client: &mut DuplexStream,
    inventory: &CharacterInventory,
    harness: &Harness,
    case: Case,
) -> TestResult<Vec<u8>> {
    let coin = inventory.entries.iter().find(|entry| entry.count.get() == 7)
        .ok_or("carried coin")?.handle;
    let bag = inventory.entries.iter().find(|entry| entry.count.get() == 1)
        .ok_or("carried bag")?.handle;
    browse(client, 1, BrowseFieldIntent::Open { position: FIELD }).await?;
    let opened = views(client).await?;
    let field = opened.views.first().ok_or("open field")?;
    assert!(field.entries.is_empty());
    assert_eq!(field.browse.ok_or("browse metadata")?.total, 0);
    // I18b uses the existing Ground destination. The donor browse window itself
    // is locked, so dropping onto its virtual container must refuse atomically.
    let payload = move_payload(coin, ItemMoveDestination::Ground(FIELD))?;
    move_command(client, 2, &payload, ItemMoveOutcome::Moved).await?;
    let dropped = views(client).await?;
    assert_eq!(custody(harness).await?, (1, 0, 0, 7));
    assert_eq!(receipts(harness).await?, 1);
    let field = dropped.views.first().ok_or("refreshed field")?;
    assert_eq!(field.browse.ok_or("browse metadata")?.total, 1);
    let source = field.entries.first().ok_or("advertised Ground coin")?.handle;
    assert_eq!(field.entries.len(), 1);
    let destination = if case == Case::Nested {
        ItemMoveDestination::Container(bag)
    } else {
        ItemMoveDestination::MainBackpack
    };
    move_command(client, 3, &move_payload(source, destination)?, ItemMoveOutcome::Moved).await?;
    let picked_up = views(client).await?;
    assert!(picked_up.views[0].entries.is_empty());
    assert_eq!(picked_up.views[0].browse.ok_or("browse metadata")?.total, 0);
    assert_eq!(receipts(harness).await?, 2);
    assert_eq!(custody(harness).await?, if case == Case::Nested { (0, 0, 1, 7) } else { (0, 1, 0, 7) });
    let before = custody(harness).await?;
    let bag_before = bag_custody(harness).await?;
    assert_eq!(bag_before, (0, 1, 0, 1));
    // The same durable owner retains its nonempty-container-drop refusal.
    if case == Case::Nested {
        move_command(client, 4, &move_payload(bag, ItemMoveDestination::Ground(FIELD))?, ItemMoveOutcome::NotSupported).await?;
        assert_eq!(bag_custody(harness).await?, bag_before);
        assert_eq!(custody(harness).await?, before);
        assert_eq!(receipts(harness).await?, 2);
    }
    // Crystal Container::queryAdd refuses its locked browse destination.
    let field = picked_up.views.first().ok_or("field after pickup")?;
    let command_id = if case == Case::Nested { 5 } else { 4 };
    move_command(client, command_id, &move_payload(bag, ItemMoveDestination::Container(field.container_handle))?, ItemMoveOutcome::NotSupported).await?;
    assert_eq!(bag_custody(harness).await?, bag_before);
    assert_eq!(custody(harness).await?, before);
    assert_eq!(receipts(harness).await?, 2);
    Ok(payload)
}

async fn navigation(client: &mut DuplexStream, inventory: &CharacterInventory, harness: &Harness) -> TestResult {
    let bag = inventory.entries.iter().find(|entry| entry.count.get() == 1)
        .ok_or("carried bag")?.handle;
    browse(client, 1, BrowseFieldIntent::Open { position: FIELD }).await?;
    views(client).await?;
    move_command(client, 2, &move_payload(bag, ItemMoveDestination::Ground(FIELD))?, ItemMoveOutcome::Moved).await?;
    let dropped = views(client).await?;
    let field = dropped.views.first().ok_or("browse view")?;
    let bag = field.entries.first().ok_or("Ground bag")?.handle;
    let field_id = field.view_id;
    let field_handle = field.container_handle;
    container(client, 3, ContainerViewIntent::Open { handle: bag, replace_view: None }, ContainerViewOutcome::Opened).await?;
    let ordinary = views(client).await?;
    assert_eq!(ordinary.views.len(), 2);
    let bag_view = ordinary.views.iter().find(|view| view.container_handle == bag)
        .ok_or("opened Ground bag view")?;
    assert!(bag_view.browse.is_none());
    let bag_view_id = bag_view.view_id;
    assert_ne!(bag_view_id, field_id);
    let retained = ordinary.views.iter().find(|view| view.view_id == field_id)
        .ok_or("original browse view retained")?;
    assert_eq!(retained.container_handle, field_handle);
    assert_eq!(retained.browse.ok_or("original browse metadata")?.position, FIELD);
    container(client, 4, ContainerViewIntent::Up { view_id: bag_view_id }, ContainerViewOutcome::Opened).await?;
    let parent = views(client).await?;
    // Crystal player.cpp:1208-1226 and game.cpp:4481-4503 permit the same
    // browse container in two view ids; Up preserves the original window.
    assert_eq!(parent.views.len(), 2);
    let mut expected_ids = [field_id, bag_view_id];
    expected_ids.sort_unstable();
    assert_eq!(parent.views.iter().map(|view| view.view_id).collect::<Vec<_>>(), expected_ids);
    for view in &parent.views {
        assert_eq!(view.container_handle, field_handle);
        let page = view.browse.ok_or("Up reached tile")?;
        assert_eq!(page.position, FIELD);
        assert_eq!(page.index, 0);
        assert_eq!(page.total, 1);
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].handle, bag);
    }
    // From x=6, x=7 and x=8 remain in range; x=9 closes both field windows.
    for command_id in 5..=7 {
        step(client, command_id, StepDirection::East).await?;
    }
    assert!(views(client).await?.views.is_empty());
    assert_eq!(receipts(harness).await?, 1, "navigation creates no durable browse state");
    Ok(())
}

async fn paging(client: &mut DuplexStream, harness: &Harness) -> TestResult {
    browse(client, 1, BrowseFieldIntent::Open { position: FIELD }).await?;
    let first = views(client).await?;
    let view = first.views.first().ok_or("first page")?;
    assert_eq!(view.capacity, 30);
    assert_eq!(view.entries.len(), 30);
    assert_eq!(view.browse.ok_or("page metadata")?.total, 31);
    let first_handles: Vec<_> = view.entries.iter().map(|entry| entry.handle).collect();
    browse(client, 2, BrowseFieldIntent::Seek { view_id: view.view_id, index: 30 }).await?;
    let second = views(client).await?;
    let page = second.views.first().ok_or("second page")?;
    assert_eq!(page.entries.len(), 1);
    assert!(!first_handles.contains(&page.entries[0].handle));
    assert_eq!(page.browse.ok_or("page metadata")?.index, 30);
    assert_eq!(page.browse.ok_or("page metadata")?.total, 31);
    assert_eq!(receipts(harness).await?, 0, "paging never persists a virtual container");
    Ok(())
}

async fn autowalk(client: &mut DuplexStream, harness: &Harness) -> TestResult {
    let far = WorldTilePosition { x: 9, ..FIELD };
    browse(client, 1, BrowseFieldIntent::Open { position: far }).await?;
    let opened = views(client).await?;
    assert_eq!(opened.views[0].browse.ok_or("walked field")?.position, far);
    for (command_id, position) in [
        (2, WorldTilePosition { floor: -8, ..far }),
        (3, WorldTilePosition { x: 1, ..FIELD }),
    ] {
        let payload = encode_browse_field_intent(&BrowseFieldIntent::Open { position }).map_err(debug)?;
        send(client, command_id, COMMAND_TYPE_BROWSE_FIELD_INTENT, &payload).await?;
        assert_eq!(decode_container_view_result(&result(client, command_id).await?).map_err(debug)?, ContainerViewOutcome::TooFar);
    }
    assert_eq!(receipts(harness).await?, 0);
    Ok(())
}

pub(super) async fn replay(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    harness: &Harness,
    payload: &[u8],
) -> TestResult {
    let before = custody(harness).await?;
    let receipt_count = receipts(harness).await?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = serve_admitted(&mut server, admitted, owner, LIVENESS);
    let client_task = async move {
        let inventory = snapshot(&mut client).await?;
        assert!(inventory.main_backpack.ok_or("restarted inventory")?.handle.get() > 1000);
        container(&mut client, 1, ContainerViewIntent::Close { view_id: 0 }, ContainerViewOutcome::Stale).await?;
        move_command(&mut client, 2, payload, ItemMoveOutcome::Moved).await?;
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(std::time::Duration::from_secs(60), join_two(serving, client_task)).await?;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)), "{end:?}");
    assert_eq!(custody(harness).await?, before);
    assert_eq!(receipts(harness).await?, receipt_count);
    Ok(())
}

async fn snapshot(client: &mut DuplexStream) -> TestResult<CharacterInventory> {
    let mut inventory = None;
    loop {
        let bytes = read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            for domain in protocol::decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                if domain.domain_id == 9 {
                    inventory = Some(decode_character_inventory_with_equip_drop(domain.payload, true).map_err(debug)?);
                } else if domain.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                    assert!(decode_container_views_for(domain.payload, true).map_err(debug)?.views.is_empty());
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            return inventory.ok_or_else(|| "inventory snapshot absent".into());
        }
    }
}

async fn send(client: &mut DuplexStream, command_id: u64, command_type: u32, payload: &[u8]) -> TestResult {
    let frame = protocol::encode_client_command(1, &protocol::ClientCommandValue { command_id, command_type, payload }).map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn result(client: &mut DuplexStream, command_id: u64) -> TestResult<Vec<u8>> {
    loop {
        let bytes = read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(result.status, CommandStatus::Accepted);
            return Ok(result.payload.to_vec());
        }
    }
}

async fn views(client: &mut DuplexStream) -> TestResult<ContainerViews> {
    loop {
        let bytes = read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                return decode_container_views_for(delta.payload, true).map_err(debug);
            }
        }
    }
}

async fn browse(client: &mut DuplexStream, command_id: u64, intent: BrowseFieldIntent) -> TestResult {
    send(client, command_id, COMMAND_TYPE_BROWSE_FIELD_INTENT, &encode_browse_field_intent(&intent).map_err(debug)?).await?;
    assert_eq!(decode_container_view_result(&result(client, command_id).await?).map_err(debug)?, ContainerViewOutcome::Opened);
    Ok(())
}

async fn container(client: &mut DuplexStream, command_id: u64, intent: ContainerViewIntent, expected: ContainerViewOutcome) -> TestResult {
    send(client, command_id, COMMAND_TYPE_CONTAINER_VIEW_INTENT, &encode_container_view_intent(&intent).map_err(debug)?).await?;
    assert_eq!(decode_container_view_result(&result(client, command_id).await?).map_err(debug)?, expected);
    Ok(())
}

fn move_payload(source: ItemHandle, destination: ItemMoveDestination) -> TestResult<Vec<u8>> {
    encode_item_move_intent_for(&ItemMoveIntent { source, destination }, SELECTION).map_err(debug)
}

async fn move_command(client: &mut DuplexStream, command_id: u64, payload: &[u8], expected: ItemMoveOutcome) -> TestResult {
    send(client, command_id, protocol::item_view::COMMAND_TYPE_ITEM_MOVE_INTENT, payload).await?;
    assert_eq!(decode_item_move_result_with_equip_drop(&result(client, command_id).await?, true).map_err(debug)?, expected);
    Ok(())
}

async fn step(client: &mut DuplexStream, command_id: u64, direction: StepDirection) -> TestResult {
    send(client, command_id, 1, &protocol::world_spatial::encode_step_intent(direction)).await?;
    assert_eq!(protocol::world_spatial::decode_step_result(&result(client, command_id).await?).map_err(debug)?, StepDisposition::Moved);
    Ok(())
}

async fn receipts(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts").fetch_one(&harness.pool).await?)
}

async fn custody(harness: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid AND parent_item_instance_id=encode($2,'hex')::uuid), quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1")
        .bind(id(71).as_slice()).bind(id(72).as_slice()).fetch_one(&harness.pool).await?)
}

async fn bag_custody(harness: &Harness) -> TestResult<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid), (SELECT count(*) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid), quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1")
        .bind(id(72).as_slice()).fetch_one(&harness.pool).await?)
}
