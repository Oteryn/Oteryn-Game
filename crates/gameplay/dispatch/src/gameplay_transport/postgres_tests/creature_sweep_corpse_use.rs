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

fn corpse_reply_complete<T: Eq>(
    accepted: bool,
    target: &T,
    opened: Option<(&T, &std::collections::BTreeMap<u32, u64>)>,
    expected: &std::collections::BTreeMap<u32, u64>,
) -> bool {
    accepted && opened.is_some_and(|(handle, contents)| handle == target && contents == expected)
}

#[test]
fn a_use_acknowledgement_does_not_prove_the_container_or_loot() {
    use std::collections::BTreeMap;
    let expected = BTreeMap::from([(1, 3)]);
    let empty = BTreeMap::new();
    assert!(!corpse_reply_complete(true, &1u8, None, &expected));
    assert!(!corpse_reply_complete(
        true,
        &1,
        Some((&2, &expected)),
        &expected
    ));
    assert!(!corpse_reply_complete(
        true,
        &1,
        Some((&1, &empty)),
        &expected
    ));
    assert!(corpse_reply_complete(
        true,
        &1,
        Some((&1, &expected)),
        &expected
    ));
}

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
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[4, 6, 7, 12, 14, 15],
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
    let corpse = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .next()
        .ok_or("committed corpse")?;
    let contents = owner
        .root
        .read_corpse_contents(owner.character, corpse.item.item_instance_id)
        .await
        .map_err(debug)?
        .ok_or("live corpse contents")?;
    let index = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("pinned Item index")?;
    let mut expected = std::collections::BTreeMap::<u32, u64>::new();
    for entry in contents {
        let definition = entry.item.definition;
        let reference = index
            .definition_ref(
                &definition.family,
                &definition.production_key,
                &definition.revision_ref,
            )
            .ok_or("exact loot definition")?;
        *expected.entry(reference.get()).or_default() += u64::from(entry.item.quantity);
    }
    assert!(
        !expected.is_empty(),
        "the known positive native loot plan is part of this regression"
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = Box::pin(serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS)).await;
        drop(server);
        end
    };
    let driving = async move {
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
                            if entity.kind == EntityKind::Corpse
                                && let EntityDetail::Object { item_handle, .. }
                                | EntityDetail::ObjectWithAppearance { item_handle, .. } =
                                    entity.detail
                            {
                                handle = item_handle;
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
        let mut accepted = false;
        let mut opened = None;
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
                accepted = true;
            }
            if env.message_type() == MessageType::StateDelta {
                let delta =
                    oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id == 11 {
                    let container =
                        oteryn_protocol_oteryn::item_view::decode_open_container(delta.payload)
                            .map_err(debug)?;
                    let mut items = std::collections::BTreeMap::<u32, u64>::new();
                    for entry in container.entries {
                        *items.entry(entry.item_definition_ref.get()).or_default() +=
                            u64::from(entry.count.get());
                    }
                    opened = container.container_handle.map(|handle| (handle, items));
                }
            }
            if corpse_reply_complete(
                accepted,
                &handle,
                opened.as_ref().map(|(handle, items)| (handle, items)),
                &expected,
            ) {
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    result?;
    assert!(
        matches!(
            end,
            crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_)
        ),
        "the real dispatcher completed without a protocol refusal"
    );
    Ok(())
}
