//! Choose Forge operands exactly as a client can: shown inventory/container handles.
use super::super::fixture::debug;
use super::super::item_look;
use super::*;
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use oteryn_protocol_oteryn::{self as protocol, container_tree, item_view};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Copy, Default)]
pub(in super::super) struct PairSelection {
    /// Open this displayed backpack entry before selecting its children.
    pub container: Option<usize>,
    /// Explicit displayed indices; otherwise select the newest seeded receiver/donor pair.
    pub indices: Option<(usize, usize)>,
    /// For wrong-item cases, select another actually displayed backpack item.
    pub second_from_backpack: Option<usize>,
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
        let nested = if let Some(index) = selection.container {
            let container = inventory.entries.get(index).ok_or("shown nested bag")?;
            let payload = container_tree::encode_container_view_intent(
                &container_tree::ContainerViewIntent::Open {
                    handle: container.handle,
                    replace_view: None,
                },
            )
            .map_err(debug)?;
            let (status, result) = support::send_command(
                &mut client,
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
            command += 1;
            Some(open_entries(&mut client, container.handle).await?)
        } else {
            None
        };
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
        let payload = request.encode().ok_or("canonical shown-handle request")?;
        let result = support::send_command(
            &mut client,
            current.continuity.connection_generation,
            command,
            protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
            &payload,
        )
        .await?;
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
