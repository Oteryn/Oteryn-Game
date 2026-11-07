//! Genuine paid Royal continuation; every captured handle comes from its own native snapshot.
use super::*;
use sha2::{Digest, Sha256};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    item: [u8; 16],
) -> TestResult {
    assert!(admitted.continuity.selected_capabilities.contains(387));
    let root = super::super::raid_live::hare_production_activation::repository_root();
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )
    .map_err(debug)?;
    assert!(input.source_world.is_some());
    let independent =
        crate::content::qualify_selected_native_gameplay_room(admitted.world_id, &input)
            .map_err(debug)?;
    let public = &independent.compiled().client_artifact;
    let metadata = super::wrap_capture_metadata::metadata(owner, admitted, public).await?;
    if let Some(path) = std::env::var_os("OTERYN_SRV7_OPAQUE_WRAP_CLIENT_ARTIFACT") {
        std::fs::write(path, public)?;
    }
    let before: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(item.as_slice()).fetch_one(&h.pool).await?;
    let native = owner
        .active_generation
        .ok_or("capture active generation")?
        .native_gameplay()
        .ok_or("capture native content")?;
    let reference = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i31502", "definition-r1"))
        .ok_or("actual post-USE Golden definition")?;
    let mut phases = Vec::new();
    for marker in [5, 3] {
        let command = admitted.continuity.next_command_id;
        let (status, result, frames, client) = super::royal_peer::command_capture(
            owner,
            admitted,
            command,
            402,
            vec![],
            Some(reference),
            Some(marker),
        )
        .await?;
        assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
        let durable: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT to_jsonb(r) FROM game_house_furniture_wrap_receipts r JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric AND r.item_instance_id=encode($3,'hex')::uuid AND r.action=1 AND r.after_key='oteryn:item.tibia.i23398' AND i.last_transaction_id=r.transaction_id AND i.state_revision=(r.before_item->>'state_revision')::numeric+1",
        ).bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string())
            .bind(item.as_slice()).fetch_optional(&h.pool).await?;
        if marker == 5 {
            assert!(durable.is_none(), "advisory cannot create a wrap receipt");
            assert_eq!(
                oteryn_protocol_oteryn::house_wrap::decode_opaque_wrap_affordance(&result),
                Some(false)
            );
            let after: serde_json::Value = sqlx::query_scalar(
                "SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid",
            ).bind(item.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(after, before, "advisory cannot mutate the acquired item");
        } else {
            assert!(
                durable.is_some(),
                "native placed Wrap must have its exact immutable receipt"
            );
            assert!(matches!(
                oteryn_protocol_oteryn::house_wrap::Reply::decode(&result),
                Some(oteryn_protocol_oteryn::house_wrap::Reply::Committed(_))
            ));
        }
        phases.push(serde_json::json!({
            "marker":marker,"command_id":command,"command_type":402,
            "client_command_envelope_hex":hex(&client),
            "server_frames_hex":frames.iter().map(|frame|hex(frame)).collect::<Vec<_>>(),
            "result_hex":hex(&result),"durable_receipt":durable,
        }));
        if marker == 3 {
            let mut fresh_poff = 0;
            for frame in &frames {
                let envelope =
                    oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
                if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id
                        == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS
                    {
                        fresh_poff += oteryn_protocol_oteryn::presentation::decode_item_effect_batch(delta.payload)
                            .map_err(debug)?.iter().filter(|event| matches!(event.kind,
                                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {effect_id:3,..})).count();
                    }
                }
            }
            assert_eq!(
                fresh_poff, 1,
                "fresh Royal placed wrap publishes exact source POFF"
            );
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&client).map_err(debug)?;
            let original = envelope
                .client_command(admitted.continuity.connection_generation)
                .map_err(debug)?;
            let (replay_status, replay_result, replay_frames, replay_client) =
                super::royal_peer::command_capture(
                    owner,
                    admitted,
                    command,
                    402,
                    original.payload.to_vec(),
                    None,
                    None,
                )
                .await?;
            assert_eq!(
                replay_status,
                oteryn_protocol_oteryn::CommandStatus::DuplicateReplay
            );
            assert_eq!(replay_result, result);
            for frame in &replay_frames {
                let envelope =
                    oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
                if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id
                        == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS
                    {
                        assert!(
                            oteryn_protocol_oteryn::presentation::decode_item_effect_batch(
                                delta.payload
                            )
                            .map_err(debug)?
                            .is_empty(),
                            "replay cannot republish POFF"
                        );
                    }
                }
            }
            phases.push(serde_json::json!({
                "marker":3,"replay":true,"command_id":command,"command_type":402,
                "client_command_envelope_hex":hex(&replay_client),
                "server_frames_hex":replay_frames.iter().map(|frame|hex(frame)).collect::<Vec<_>>(),
                "result_hex":hex(&replay_result),
            }));
        }
    }
    let after: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after["definition_production_key"],
        "oteryn:item.tibia.i23398"
    );
    assert_eq!(before["item_instance_id"], after["item_instance_id"]);
    assert_eq!(
        before["minted_transaction_id"],
        after["minted_transaction_id"]
    );
    let capture = serde_json::json!({
        "schema":"OTERYN_ROYAL_OPAQUE_WRAP_NATIVE_CAPTURE/v1",
        "producer_test":"srv7_npc_king_one_billion_actual_inbox_house_golden_display_pipeline",
        "qualification":"Full genuine1b Royal source path; explicit source-region fixture staging is retained and is not a travel proof. serve_admitted epochs omit Accepted; no cold admission claim.",
        "source":metadata,"public_artifact_sha256":hex(&Sha256::digest(public)),
        "item_uuid_hex":hex(&item),"before":before,"after":after,"phases":phases,
    });
    std::fs::write(
        std::env::var_os("OTERYN_SRV7_OPAQUE_WRAP_CAPTURE").ok_or("actual capture output")?,
        serde_json::to_vec_pretty(&capture)?,
    )?;
    Ok(())
}
