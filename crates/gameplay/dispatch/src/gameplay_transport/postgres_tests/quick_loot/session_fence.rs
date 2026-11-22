use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::{self as protocol, item_view, world_spatial_entities};
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn quick_loot_resolves_visible_corpse_and_recipient_under_session_fence() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-visible", || {
        scenario_with_pouch(false, Some(FenceCase::Visible))
    })
}

#[test]
fn quick_loot_refuses_stale_handles_without_durable_writes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-stale", || {
        scenario_with_pouch(false, Some(FenceCase::Stale))
    })
}

#[derive(Clone, Copy)]
pub(super) enum FenceCase {
    Visible,
    Stale,
}

#[derive(Default)]
pub(super) struct ShownHandles {
    pub(super) bag: Option<[u8; 16]>,
    pub(super) corpse: Option<[u8; 16]>,
}

async fn read_handles(
    client: &mut DuplexStream,
    corpse_identity: [u8; 16],
) -> TestResult<ShownHandles> {
    use world_spatial_entities::EntityDetail;
    let mut shown = ShownHandles::default();
    loop {
        let frame = super::super::item_look_fixtures::read_frame(client).await?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            protocol::MessageType::SnapshotCommit => return Ok(shown),
            protocol::MessageType::SnapshotChunk => {}
            _ => continue,
        }
        for domain in protocol::decode_snapshot_chunk(envelope.payload())
            .map_err(debug)?
            .1
        {
            if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                let inventory =
                    item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                shown.bag = inventory
                    .entries
                    .first()
                    .map(|item| handle_bytes(item.handle));
            } else if domain.domain_id
                == protocol::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
                && domain.snapshot_type
                    == world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2
            {
                let snapshot = world_spatial_entities::decode_world_spatial_entities_snapshot_with_item_handles(domain.payload)
                    .map_err(debug)?;
                shown.corpse = snapshot.entities.iter().find_map(|entry| {
                    if entry.entity.identity != corpse_identity {
                        return None;
                    }
                    match entry.detail {
                        EntityDetail::Object { item_handle, .. }
                        | EntityDetail::ObjectWithAppearance { item_handle, .. } => {
                            item_handle.map(handle_bytes)
                        }
                        _ => None,
                    }
                });
            }
        }
    }
}

fn handle_bytes(handle: item_view::ItemHandle) -> [u8; 16] {
    u128::from(handle.get()).to_be_bytes()
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    corpse_identity: [u8; 16],
    requests: impl FnOnce(&ShownHandles) -> TestResult<Vec<QuickLootRequest>>,
) -> TestResult<(ShownHandles, Vec<(CommandStatus, QuickLootResult)>)> {
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move {
        match serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await {
            ConnectionEnd::AdmittedThenDisconnected(next) => Ok(next),
            end => Err(format!("quick-loot wire session ended: {end:?}").into()),
        }
    };
    let exercise = async move {
        let shown = read_handles(&mut client, corpse_identity).await?;
        let mut replies = Vec::new();
        for (offset, request) in requests(&shown)?.into_iter().enumerate() {
            let command_id = current.continuity.next_command_id + u64::try_from(offset)?;
            let payload = encode_quick_loot_request(&request).map_err(debug)?;
            let frame = protocol::encode_client_command(
                current.continuity.connection_generation,
                &protocol::ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_QUICK_LOOT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let frame = super::super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::CommandResult {
                    let reply =
                        protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command_id);
                    replies.push((
                        reply.status,
                        decode_quick_loot_result(reply.payload).map_err(debug)?,
                    ));
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>((shown, replies))
    };
    let (next, result) = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        tokio::try_join!(serving, exercise)
    })
    .await??;
    *admitted = next;
    Ok(result)
}

impl FenceCase {
    pub(super) async fn check(
        self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        pool: &sqlx::PgPool,
        corpse_identity: [u8; 16],
        corpse_id: [u8; 16],
        items: &[[u8; 16]],
    ) -> TestResult {
        let assign = |handle| QuickLootRequest {
            recipient: Some(QuickLootRecipient::Container),
            recipient_id: Some(handle),
            ..request(QuickLootAction::Assign)
        };
        let click = |handle| QuickLootRequest {
            corpse: Some(handle),
            ..request(QuickLootAction::Click)
        };
        match self {
            Self::Visible => {
                let (_, replies) = exchange(owner, admitted, corpse_identity, |shown| {
                    Ok(vec![
                        assign(shown.bag.ok_or("shown bag")?),
                        click(shown.corpse.ok_or("shown corpse")?),
                    ])
                })
                .await?;
                assert_eq!(replies[0].0, CommandStatus::Accepted);
                assert_eq!(replies[1].0, CommandStatus::Accepted);
                assert_eq!(
                    replies[1].1.items,
                    items
                        .iter()
                        .map(|&item| QuickLootItemResult {
                            item,
                            accepted: true
                        })
                        .collect::<Vec<_>>()
                );
                let quantities: Vec<i64> = sqlx::query_scalar("SELECT i.quantity FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY i.quantity")
                    .bind(id(71).as_slice()).fetch_all(pool).await?;
                assert_eq!(quantities, [20, 100]);
            }
            Self::Stale => {
                let (previous, _) =
                    exchange(owner, admitted, corpse_identity, |_| Ok(Vec::new())).await?;
                let corpse = previous.corpse.ok_or("previously shown corpse")?;
                let bag = previous.bag.ok_or("previously shown bag")?;
                let (_, replies) = exchange(owner, admitted, corpse_identity, |shown| {
                    assert_ne!(shown.corpse.ok_or("current corpse handle")?, corpse);
                    assert_ne!(shown.bag.ok_or("current bag handle")?, bag);
                    Ok(vec![
                        click(corpse),
                        assign(bag),
                        click(corpse_id),
                        assign(id(71)),
                    ])
                })
                .await?;
                let reads: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quick_loot_receipts), (SELECT count(*) FROM game_character_quick_loot_settings), (SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
                    .bind(corpse_id.as_slice()).fetch_one(pool).await?;
                assert_eq!(reads, (0, 0, i64::try_from(items.len())?));
                assert_eq!(
                    replies,
                    vec![
                        (
                            CommandStatus::Rejected,
                            QuickLootResult { items: Vec::new() }
                        );
                        4
                    ]
                );
            }
        }
        Ok(())
    }
}
