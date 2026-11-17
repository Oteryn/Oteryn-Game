//! A fresh native kill's corpse through the actual USE dispatcher, including optional Item owners.
use super::{TestResult, fixture::debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::connection::{AdmittedSession, IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, capabilities::SelectedCapabilities};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, EntityFields, EntityKind, decode_world_spatial_entities_snapshot_with_fields,
};
use tokio::io::AsyncWriteExt;

#[test]
fn fresh_native_corpse_use_through_real_dispatch_postgres() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(
        "fresh-corpse-use",
        super::kill_reward_e2e::corpse_use_scenario,
    )
}

pub(super) async fn open(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    let selected = SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::ITEM_VIEW_OFFERED_CAPABILITIES,
        &[4, 6, 7, 14, 15],
    )
    .ok_or("corpse capabilities")?;
    assert!(selected.contains(4) && selected.contains(14) && selected.contains(15));
    admitted.continuity.selected_capabilities = selected;
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(owner.root.clone())
            .current_session(admitted.game_session_id)
            .await
            .map_err(debug)?;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = Box::pin(serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS)).await;
        drop(server);
        end
    };
    let driving = async {
        let mut handle = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(env.payload()).map_err(debug)?;
                for domain in domains {
                    if domain.domain_id == 1 {
                        let (snapshot, _) = decode_world_spatial_entities_snapshot_with_fields(
                            domain.payload,
                            EntityFields::selected(selected.as_slice()),
                        )
                        .map_err(debug)?;
                        for entity in snapshot.entities {
                            if entity.kind == EntityKind::Corpse {
                                if let EntityDetail::Object { item_handle, .. }
                                | EntityDetail::ObjectWithAppearance { item_handle, .. } =
                                    entity.detail
                                {
                                    handle = item_handle;
                                }
                            }
                        }
                    }
                }
            }
            if env.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let handle = handle.ok_or("fresh committed corpse must have a live handle")?;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_item_intent(handle);
        let command = oteryn_protocol_oteryn::encode_client_command(
            admitted.continuity.connection_generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: admitted.continuity.next_command_id,
                command_type: 2,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &command).await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let result =
                    oteryn_protocol_oteryn::decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(
                    result.status,
                    oteryn_protocol_oteryn::CommandStatus::Accepted
                );
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                        .map_err(debug)?,
                    oteryn_protocol_oteryn::world_object::UseDisposition::Committed
                );
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    result
}
