//! Actual admitted source Messenger -> unchanged production appearance25 dictionary and map wire.
use super::*;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::connection::{
    FreshAdmissionAuthority, seasonal_appearance_capture::SeasonalAppearanceCapture,
};
use sha2::{Digest, Sha256};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) async fn capture(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &AdmittedSession,
    entity: oteryn_protocol_oteryn::appearance::EntityRef,
) -> TestResult {
    let mut admitted = *admitted;
    let session = admitted.game_session_id;
    let fence = owner
        .current_monk_fence(session)
        .await
        .map_err(|_| "Messenger fence")?
        .ok_or("Messenger actual session")?;
    let observation = owner
        .appearance_observation(deps.actor, session, vec![entity])
        .await
        .ok_or("actual Messenger source Outfit159 production appearance observation")?;
    assert_eq!(observation.bindings.len(), 1);
    assert_eq!(observation.bindings[0].entity, entity);
    assert_eq!(
        observation.bindings[0].look,
        oteryn_protocol_oteryn::appearance::Look::Outfit {
            look_type: 159,
            colours: [39, 99, 60, 117],
            addons: 0
        }
    );
    let visible = owner
        .observe_visible_entities(deps.actor)
        .await
        .ok_or("Messenger visibility")?;
    let (mut recorder, domains) =
        SeasonalAppearanceCapture::begin(owner, deps.actor, &mut admitted, &visible)
            .await
            .ok_or("actual Messenger appearance25 snapshot")?;
    let map = recorder
        .initial_map(owner, deps.actor, &mut admitted)
        .await
        .ok_or("actual shared Messenger initial map")?;
    let accepted =
        crate::foundation::encode_server_accepted(&crate::foundation::ServerAcceptedValue {
            game_session_id: session,
            world_id: admitted.world_id,
            channel_id: admitted.channel_id,
            connection_generation: admitted.continuity.connection_generation,
            current_server_sequence: admitted.continuity.server_sequence,
            next_command_id: admitted.continuity.next_command_id,
            schema_revision: crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,
            selected_capabilities: admitted.continuity.selected_capabilities.as_slice(),
        })
        .map_err(debug)?;
    let mut borrowed = domains
        .iter()
        .map(|d| oteryn_protocol_oteryn::DomainSnapshot {
            domain_id: d.domain_id,
            revision: d.revision,
            snapshot_type: d.snapshot_type,
            payload: &d.payload,
        })
        .collect::<Vec<_>>();
    borrowed.push(oteryn_protocol_oteryn::DomainSnapshot {
        domain_id: map.domain_id,
        revision: map.revision,
        snapshot_type: map.snapshot_type,
        payload: &map.payload,
    });
    let snapshots = oteryn_protocol_oteryn::encode_single_chunk_snapshot(
        admitted.continuity.connection_generation,
        1,
        admitted.continuity.server_sequence,
        &borrowed,
    )
    .map_err(debug)?;
    let mut initial = Vec::new();
    crate::gameplay_transport::tcp_tls::write_frame(&mut initial, &accepted).await?;
    for frame in snapshots {
        crate::gameplay_transport::tcp_tls::write_frame(&mut initial, &frame).await?;
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_MESSENGER_NATIVE_CAPTURE") {
        let keys = std::fs::read(
            super::super::raid_live::hare_production_activation::repository_root()
                .join("tools/content-schema/native-gameplay/item-keys.json"),
        )?;
        let runtime = deps.runtime.lock().await;
        let viewer = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let source = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|entry| {
                entry.actor.placement_identity() == entity.identity
                    && entry.generation == entity.generation
            })
            .ok_or("actual Messenger position")?
            .position;
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema":"OTERYN_SEASONAL_NPC_NATIVE_CAPTURE/v1",
                "source":{"producer_test":"srv7_messenger_of_santa_native_hi_trade_thirteen_current_goods_buy_and_replay",
                    "character_id_hex":hex(fence.character_id.as_bytes()),
                    "world_id_hex":hex(admitted.world_id.as_bytes()),
                    "client_content_generation":hex(&runtime.content_pin().client_artifact_digest()),
                    "npc_key":"oteryn:npc.messenger_of_santa",
                "source_look_type":159,"source_colours":[39,99,60,117],
                "item_keys_sha256":hex(&Sha256::digest(&keys)),
                "production_source_world_preserved":true,
                    "actor_identity_hex":hex(&entity.identity),
                    "actor_generation":entity.generation,
                    "source_runtime_cell":[source.x,source.y,source.floor], "map_fixture":"existing grizzly caller Bundle; production source placement separately qualified by seasonal_population compiler",
                    "observer_runtime_cell":[viewer.x,viewer.y,viewer.floor]},
                "initial_frames_hex":hex(&initial),"initial_frames_sha256":hex(&Sha256::digest(&initial))
            }))?,
        )?;
    }
    eprintln!(
        "SRV7_MESSENGER_NATIVE_BODY outfit159 palette39_99_60_117 appearance25=true forge104=false"
    );
    Ok(())
}
