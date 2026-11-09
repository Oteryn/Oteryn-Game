//! Real current-House USE and its same-socket blue effect; no synthesized packets.
use super::*;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};

pub(in crate::gameplay_transport) async fn assert_native_house_use(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("actual House item fence")?;
    let private = {
        let runtime = owner.runtime.lock().await;
        let state = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(debug)?;
        assert_eq!(state.scope, fence.runtime_scope);
        assert_eq!(state.generation, fence.scope_ownership_generation);
        assert_eq!(state.lease.generation(), fence.character_lease_generation);
        state.position
    };
    let mut supported = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    supported.push(23);
    supported.sort_unstable();
    supported.dedup();
    let actual_selection =
        crate::gameplay_transport::connection::fresh_current_actor_capability_selection(
            owner, &supported, admitted,
        )
        .await
        .ok_or("actual current House selection")?;
    assert_eq!(
        actual_selection, admitted.continuity.selected_capabilities,
        "the current House retains its immutable pre-handoff capability selection"
    );
    assert!(
        admitted.continuity.selected_capabilities.contains(23),
        "actual current House presentation23 must be selected before awaiting native egress"
    );
    let initial = admitted;
    let native = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .ok_or("native House body")?;
    let definition = native
        .item_index()
        .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i43501", "definition-r1"))
        .ok_or("actual native essence")?;
    let mut frames = Vec::new();
    let equip_drop = admitted
        .continuity
        .selected_capabilities
        .contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1);
    h.root.request_ready();
    assert!(h.root.maintain_ready_once().await.map_err(debug)?);
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: Some(fence),
    };
    let (_,_,scope,payload)=super::super::guild_wire::exchange_capture_effect(owner,&mut admitted,command.command_id,
        oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,|initial| {
            let mut handle=None;
            for raw in initial {let envelope=decode_wire_envelope(raw).map_err(debug)?;
                if envelope.message_type()!=MessageType::SnapshotChunk {continue;}
                let (_,domains)=decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                for domain in domains {if domain.domain_id!=oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {continue;}
                    let inventory=oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(domain.payload,equip_drop).map_err(debug)?;
                    for entry in inventory.entries {if entry.item_definition_ref==definition {
                        assert!(handle.replace(entry.handle).is_none(),"one actual carried essence handle");
                    }}
                }
            }
            encode_item_use_intent(&UseIntent {target:UseTarget::Item(handle.ok_or("actual House inventory essence handle")?),
                use_with:None,use_with_position:None,use_with_map_item:None}).map_err(debug)
        },13,&mut frames).await?;
    assert!(
        scope.is_none(),
        "the effect keeps the current named House transport"
    );
    assert_eq!(admitted.game_session_id, initial.game_session_id);
    let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT quantity::bigint FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid),(SELECT value FROM game_character_quest_tracks WHERE character_id=encode($2,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'),(SELECT count(*) FROM game_item_use_records WHERE cause_kind=4)")
        .bind(id(71).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (1, 1, 1));
    assert_eq!(
        owner
            .use_item(
                actor,
                command,
                crate::gameplay_transport::item_view::ItemKey::Instance(id(71))
            )
            .await
            .disposition,
        UseDisposition::Committed,
        "same actual handle binding replays without another burn or effect"
    );
    let effects = owner
        .observe_current_actor_movement_effects(actor, admitted.game_session_id, 0)
        .await;
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].2, 13);
    assert!(
        owner.observe_movement_effects(0).is_empty(),
        "no origin Channel leakage"
    );
    let mut seen = 0;
    for raw in &frames {
        let envelope = decode_wire_envelope(raw).map_err(debug)?;
        if envelope.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id != oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
        {
            continue;
        }
        for event in oteryn_protocol_oteryn::presentation::decode_presentation_batch(delta.payload)
            .map_err(debug)?
        {
            if let oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                position,
                effect_id: 13,
                ..
            } = event.kind
            {
                assert_eq!(
                    (position.x, position.y, position.floor),
                    (private.x, private.y, private.floor),
                    "decoded effect uses the actual current House wire frame"
                );
                seen += 1;
            }
        }
    }
    assert_eq!(
        seen, 1,
        "one real same-socket blue presentation after native USE"
    );
    if let Ok(path) = std::env::var("SRX3_HOUSE_ESSENCE_CAPTURE_PATH") {
        let hex = |raw: &[u8]| raw.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let boundary = frames
            .iter()
            .position(|raw| {
                decode_wire_envelope(raw)
                    .is_ok_and(|e| e.message_type() == MessageType::SnapshotCommit)
            })
            .ok_or("actual House initial snapshot boundary")?
            + 1;
        let value = serde_json::json!({"schema":"OTERYN_NATIVE_HOUSE_ESSENCE_REPLAY/v1",
            "character_id":hex(fence.character_id.as_bytes()),
            "admission":{"game_session_id":hex(initial.game_session_id.as_bytes()),"world_id":hex(initial.world_id.as_bytes()),
                "channel_id":hex(initial.channel_id.as_bytes()),"connection_generation":initial.continuity.connection_generation,
                "current_server_sequence":initial.continuity.server_sequence,"next_command_id":initial.continuity.next_command_id,
                "schema_revision":crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,
                "selected_capabilities":initial.continuity.selected_capabilities.as_slice()},
            "initial_frames":frames[..boundary].iter().map(|raw|hex(raw)).collect::<Vec<_>>(),
            "command":{"kind":oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,"payload":hex(&payload)},
            "subsequent_frames":frames[boundary..].iter().map(|raw|hex(raw)).collect::<Vec<_>>(),
            "position":[private.x,private.y,private.floor],"effect":13});
        std::fs::write(path, serde_json::to_vec_pretty(&value)?)?;
    }
    eprintln!(
        "SRX3_HOUSE_ESSENCE_NATIVE_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
