//! Choose Forge operands exactly as a client can: shown inventory/container handles.

use super::super::fixture::debug;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use super::super::item_look_fixtures as item_look;

use super::*;

use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};

use oteryn_protocol_oteryn::{self as protocol, container_tree, item_view};

#[derive(Debug, Clone, Copy, Default)]
pub(in super::super) struct PairSelection {
    /// Open this displayed backpack entry before selecting its children.
    pub container: Option<usize>,
    pub nested_path: &'static [usize],
    /// Explicit displayed indices; otherwise select the newest seeded receiver/donor pair.
    pub indices: Option<(usize, usize)>,
    /// For wrong-item cases, select another actually displayed backpack item.
    pub second_from_backpack: Option<usize>,
    pub chest_items: Option<usize>,
}

pub(in super::super) async fn exchange_pair(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    mut request: Request,
    selection: PairSelection,
) -> TestResult<(Request, (CommandStatus, Value))> {
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exercise = async {
        let inventory = inventory(&mut client).await?;
        let mut command = current.continuity.next_command_id;
        let mut nested: Option<Vec<_>> = None;
        for index in selection
            .container
            .into_iter()
            .chain(selection.nested_path.iter().copied())
        {
            let entries = nested.as_deref().unwrap_or(inventory.entries.as_slice());
            let container = entries.get(index).ok_or("shown nested bag")?;
            nested =
                Some(open_wire_container(&mut client, current, command, container.handle).await?);
            command += 1;
        }
        let entries = nested.as_deref().unwrap_or(inventory.entries.as_slice());
        if entries.len() < 2 {
            return Err("two shown Forge inputs".into());
        }
        // Both native carried-container providers display newest placement first.
        // Fixtures append the receiver and then the donor, so their shown order
        // is donor, receiver. The operands are still actual client-visible handles.
        let (first, second) = selection.indices.unwrap_or((1, 0));
        request.first_item = Some(entries.get(first).ok_or("shown receiver")?.handle);
        request.second_item = Some(if let Some(index) = selection.second_from_backpack {
            inventory
                .entries
                .get(index)
                .ok_or("shown wrong source")?
                .handle
        } else {
            entries.get(second).ok_or("shown donor")?.handle
        });
        if request.operation == Operation::Transfer && selection.second_from_backpack.is_none() {
            use protocol::forge_preview as preview;
            let selection = preview::Selection::Pair {
                operation: request.operation,
                first: request.first_item.ok_or("receiver handle")?,
                second: request.second_item.ok_or("donor handle")?,
                boost: false,
                protect: false,
            };
            let reply = super::super::forge_preview_fixtures::read(
                &mut client,
                current.continuity.connection_generation,
                &mut command,
                1,
                selection,
            )
            .await?;
            let preview::Payload::Quote(quote) = reply.payload else {
                return Err("transfer pair quote".into());
            };
            assert_eq!(
                (quote.cost.gold, quote.cost.dust, quote.cost.cores),
                (20_000_000, 100, 1)
            );
            assert_eq!(quote.target_tier, 1);
        }
        let payload = request.encode().ok_or("canonical shown-handle request")?;
        let result = support::send_command(
            &mut client,
            current.continuity.connection_generation,
            command,
            protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
            &payload,
        )
        .await?;
        let chest = match selection.chest_items {
            Some(_) => {
                assert_eq!(result.0, CommandStatus::Accepted);
                let reply: Value = serde_json::from_slice(&result.1)?;
                assert_eq!(reply["succeeded"], true, "{reply}");
                let definition = chest_definition(owner)?;
                let updated = inventory_delta(&mut client).await?;
                assert!(updated.entries.iter().all(|item| {
                    Some(item.handle) != request.first_item
                        && Some(item.handle) != request.second_item
                }));
                let mut chests = updated
                    .entries
                    .iter()
                    .filter(|item| item.item_definition_ref == definition);
                let chest = chests
                    .next()
                    .ok_or("Forge refresh did not show the Exaltation Chest")?;
                assert!(chests.next().is_none());
                Some(chest.handle)
            }
            None => None,
        };
        let command_replay = support::send_command(
            &mut client,
            current.continuity.connection_generation,
            command,
            protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
            &payload,
        )
        .await?;
        // SESSION-INGRESS-1 marks remembered CommandIds as DuplicateReplay while
        // retaining the original payload, as for the existing container-sort wire.
        assert_eq!(command_replay.0, CommandStatus::DuplicateReplay);
        assert_eq!(
            command_replay.1, result.1,
            "the identical CommandId retains the exact original Forge payload"
        );
        let replay = support::send_command(
            &mut client,
            current.continuity.connection_generation,
            command + 1,
            protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
            &payload,
        )
        .await?;
        assert_eq!(
            replay, result,
            "same-connection replay survives source consumption and handle refresh"
        );
        if let Some(handle) = chest {
            let children = open_wire_container(&mut client, current, command + 2, handle).await?;
            assert_eq!(Some(children.len()), selection.chest_items);
            assert!(children.iter().all(|item| item.count.get() == 1));
        }
        client.shutdown().await?;
        let mut remaining = Vec::new();
        client.read_to_end(&mut remaining).await?;
        Ok::<_, Box<dyn std::error::Error>>((
            request,
            (result.0, serde_json::from_slice(&result.1)?),
        ))
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        support::together(serving, exercise),
    )
    .await?;
    let server_end = support::server_end_summary(&end);
    *admitted = match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => next,
        _ => {
            return Err(format!("Forge handle connection ended unexpectedly: {server_end}").into());
        }
    };
    result.map_err(|error| {
        format!(
            "Forge {:?} handle exchange from command {} failed: {error}; \
             awaited server outcome: {server_end}",
            request.operation, current.continuity.next_command_id,
        )
        .into()
    })
}

/// Both paid attempts use the original live connection and its issued handles.
pub(in super::super) async fn paid_fusion_sequence(
    h: &super::super::fixture::Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    mut requests: [Request; 2],
) -> TestResult<[(Request, Value); 2]> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exercise = async move {
        let snapshot = inventory(&mut client).await?;
        // The three newest entries are the success donor, failure donor and receiver.
        let [second_donor, first_donor, receiver, ..] = snapshot.entries.as_slice() else {
            return Err("expected three visible Fusion operands".into());
        };
        let mut replies = Vec::with_capacity(2);
        for (offset, (request, donor)) in requests
            .iter_mut()
            .zip([first_donor, second_donor])
            .enumerate()
        {
            let command = admitted.continuity.next_command_id + u64::try_from(offset)?;
            request.first_item = Some(receiver.handle);
            request.second_item = Some(donor.handle);
            let payload = request.encode().ok_or("canonical paid Fusion")?;
            let (status, bytes) = support::send_command(
                &mut client,
                admitted.continuity.connection_generation,
                command,
                protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
                &payload,
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted, "Fusion command {command}");
            let reply: Value = serde_json::from_slice(&bytes)?;
            assert_eq!(reply["applied"], true, "Fusion command {command}: {reply}");
            assert_eq!(reply["succeeded"], offset == 1);
            let updated = inventory_delta(&mut client).await?;
            assert!(updated.entries.iter().all(|item| item.handle != donor.handle));
            if offset == 0 {
                assert!(updated.entries.iter().any(|item| item.handle == receiver.handle));
                assert!(updated.entries.iter().any(|item| item.handle == second_donor.handle));
                // Advance the existing durable cooldown fixture after the first receipt.
                support::cooldown_elapsed(h).await?;
            } else {
                let definition = chest_definition(owner)?;
                let chest = updated
                    .entries
                    .iter()
                    .find(|item| item.item_definition_ref == definition)
                    .ok_or("successful Fusion chest missing from inventory")?;
                let children =
                    open_wire_container(&mut client, admitted, command + 1, chest.handle).await?;
                assert_eq!(children.len(), 1);
                assert_eq!(children[0].item_definition_ref, receiver.item_definition_ref);
                assert!(updated.entries.iter().all(|item| item.handle != receiver.handle));
            }
            replies.push((*request, reply));
        }
        client.shutdown().await?;
        let mut remaining = Vec::new();
        client.read_to_end(&mut remaining).await?;
        replies.try_into().map_err(|_| "expected two Fusion receipts".into())
    };
    let (end, replies) = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        support::together(serving, exercise),
    )
    .await?;
    if !matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)) {
        return Err(format!(
            "paid Fusion connection ended: {}",
            support::server_end_summary(&end)
        )
        .into());
    }
    replies
}

fn chest_definition(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<std::num::NonZeroU32> {
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| {
            index.definition_ref(
                "Item",
                super::super::item_forge_fusion_fixtures::CHEST,
                "definition-r1",
            )
        })
        .ok_or_else(|| "Exaltation Chest is absent from the active item index".into())
}

pub(in super::super) async fn extract_chest_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let current = *admitted;
    let definition = chest_definition(owner)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exercise = async {
        use protocol::item_look::{
            ItemLookIntent, ItemLookResult, ItemLookSelection, ItemLookTarget,
        };
        let initial = inventory(&mut client).await?;
        let chest = initial
            .entries
            .iter()
            .find(|entry| entry.item_definition_ref == definition)
            .ok_or("fresh inventory snapshot omitted Exaltation Chest")?;
        let command = current.continuity.next_command_id;
        let children = open_wire_container(&mut client, current, command, chest.handle).await?;
        assert_eq!(children.len(), 2);
        let receiver = children
            .last()
            .ok_or("oldest chest entry is the receiver")?;
        assert!(
            initial
                .entries
                .iter()
                .all(|entry| entry.item_definition_ref != receiver.item_definition_ref)
        );
        let look_selection = ItemLookSelection { extended: true };
        let look = protocol::item_look::encode_item_look_intent_for(
            ItemLookIntent {
                target: ItemLookTarget::Item(receiver.handle),
            },
            look_selection,
        )
        .map_err(debug)?;
        let (status, reply) = support::send_command(
            &mut client,
            current.continuity.connection_generation,
            command + 1,
            protocol::item_look::ITEM_LOOK_COMMAND_ID.ok_or("item look command")?,
            &look,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let ItemLookResult::Text(description) =
            protocol::item_look::decode_item_look_result_for(&reply, look_selection)
                .map_err(debug)?
        else {
            return Err("Look did not resolve the sword inside the Exaltation Chest".into());
        };
        assert!(description.contains("sword"), "{description}");
        let payload = item_view::encode_item_move_intent(&item_view::ItemMoveIntent {
            source: receiver.handle,
            destination: item_view::ItemMoveDestination::MainBackpack,
        })
        .map_err(debug)?;
        let (status, reply) = support::send_command(
            &mut client,
            current.continuity.connection_generation,
            command + 2,
            item_view::COMMAND_TYPE_ITEM_MOVE_INTENT,
            &payload,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            item_view::decode_item_move_result(&reply).map_err(debug)?,
            item_view::ItemMoveOutcome::Moved
        );
        let updated = inventory_delta(&mut client).await?;
        assert_eq!(
            updated
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == receiver.item_definition_ref)
                .count(),
            1
        );
        client.shutdown().await?;
        let mut remaining = Vec::new();
        client.read_to_end(&mut remaining).await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        support::together(serving, exercise),
    )
    .await?;
    *admitted = match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => next,
        _ => {
            return Err(format!(
                "chest item connection ended unexpectedly: {}",
                support::server_end_summary(&end)
            )
            .into());
        }
    };
    result
}

async fn inventory_delta(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<item_view::CharacterInventory> {
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::StateDelta {
            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                return item_view::decode_character_inventory_with_equip_drop(delta.payload, true)
                    .map_err(|error| debug(error).into());
            }
        }
    }
}

async fn open_wire_container(
    client: &mut tokio::io::DuplexStream,
    current: AdmittedSession,
    command: u64,
    handle: item_view::ItemHandle,
) -> TestResult<Vec<item_view::ItemEntry>> {
    let payload =
        container_tree::encode_container_view_intent(&container_tree::ContainerViewIntent::Open {
            handle,
            replace_view: None,
        })
        .map_err(debug)?;
    let (status, result) = support::send_command(
        client,
        current.continuity.connection_generation,
        command,
        container_tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
        &payload,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(
        container_tree::decode_container_view_result(&result).map_err(debug)?,
        container_tree::ContainerViewOutcome::Opened
    );
    open_entries(client, handle).await
}

async fn inventory(
    client: &mut tokio::io::DuplexStream,
) -> TestResult<item_view::CharacterInventory> {
    let mut inventory = None;
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::SnapshotChunk {
            for domain in protocol::decode_snapshot_chunk(envelope.payload())
                .map_err(debug)?
                .1
            {
                if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                    inventory = Some(
                        item_view::decode_character_inventory_with_equip_drop(domain.payload, true)
                            .map_err(debug)?,
                    );
                }
            }
        }
        if envelope.message_type() == protocol::MessageType::SnapshotCommit {
            return inventory
                .ok_or_else(|| "Forge client did not receive an inventory snapshot".into());
        }
    }
}

async fn open_entries(
    client: &mut tokio::io::DuplexStream,
    handle: item_view::ItemHandle,
) -> TestResult<Vec<item_view::ItemEntry>> {
    loop {
        let frame = item_look::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::StateDelta {
            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == container_tree::STATE_DOMAIN_CONTAINER_VIEWS {
                return container_tree::decode_container_views(delta.payload)
                    .map_err(debug)?
                    .views
                    .into_iter()
                    .find(|view| view.container_handle == handle)
                    .map(|view| view.entries)
                    .ok_or_else(|| "opened Forge input bag was not shown".into());
            }
        }
    }
}
