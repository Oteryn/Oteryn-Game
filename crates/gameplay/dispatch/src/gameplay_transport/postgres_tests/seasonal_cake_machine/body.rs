//! Actual admitted Golem -> unchanged production appearance25 dictionary and map wire.
use super::*;
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
    golem: crate::foundation::ExactActorRef,
) -> TestResult {
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_monk_fence(session)
        .await
        .map_err(|_| "Golem fence")?
        .ok_or("Golem actual current session")?;
    deps.runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            session,
            crate::foundation::CharacterLease::new(
                oteryn_protocol_oteryn::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    assert!(owner.track_restored(
        session,
        deps.actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));
    let mut admitted = deps.admitted;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "Golem Item fence")?
            .ok_or("Golem current Item fence")?
            .item_fence(),
    );
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offered,
            &[4, 6, 18, 25],
        )
        .ok_or("Golem actual appearance25 capabilities")?;
    assert!(admitted.continuity.selected_capabilities.contains(25));
    assert!(!admitted.continuity.selected_capabilities.contains(104));
    let entity = oteryn_protocol_oteryn::appearance::EntityRef {
        identity: golem.placement_identity(),
        generation: golem.actor_local_generation(),
    };
    let native_probe = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("Golem actual native probe")?;
    let own = oteryn_protocol_oteryn::appearance::EntityRef {
        identity: deps.actor.placement_identity(),
        generation: deps.actor.actor_local_generation(),
    };
    let own_observation = owner
        .appearance_observation(deps.actor, session, vec![own])
        .await
        .is_some();
    {
        let runtime = deps.runtime.lock().await;
        let snapshot = runtime.companion_snapshot(golem).map_err(debug)?;
        eprintln!(
            "SRV7_GOLEM_BODY_PREFLIGHT own_observation={own_observation} context={} policy6393={} key={} state_item={:?} policy_item={:?} finite={} census_contains={}",
            owner.appearance_available(),
            native_probe.item_policy_for_source_id(6393).is_some(),
            snapshot.state.policy.definition_key,
            snapshot.state.object_look_type,
            snapshot.state.policy.object_look_type,
            crate::gameplay_transport::seasonal_appearance::creature(&runtime, golem),
            runtime
                .positioned_melee_census()
                .map_err(debug)?
                .iter()
                .any(|(actor, _, _)| *actor == golem)
        );
    }
    let observation = owner
        .appearance_observation(deps.actor, session, vec![entity])
        .await
        .ok_or("actual Golem Item6393 production appearance observation")?;
    let native = owner
        .active_generation
        .ok_or("Golem generation")?
        .native_gameplay()
        .ok_or("Golem native")?;
    let item_ref = native
        .item_index()
        .ok_or("Golem Item index")?
        .definition_ref("Item", "oteryn:item.tibia.i6393", "definition-r1")
        .ok_or("actual typed Golem6393 appearance ref")?
        .get();
    assert_eq!(observation.bindings.len(), 1);
    assert_eq!(observation.bindings[0].entity, entity);
    assert_eq!(
        observation.bindings[0].look,
        oteryn_protocol_oteryn::appearance::Look::Item {
            definition_ref: item_ref
        }
    );
    let visible = owner
        .observe_visible_entities(deps.actor)
        .await
        .ok_or("Golem visibility")?;
    let (mut recorder, domains) =
        SeasonalAppearanceCapture::begin(owner, deps.actor, &mut admitted, &visible)
            .await
            .ok_or("actual Golem appearance25 snapshot")?;
    let map = recorder
        .initial_map(owner, deps.actor, &mut admitted)
        .await
        .ok_or("actual shared Golem initial map")?;
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
    if let Some(path) = std::env::var_os("OTERYN_SRV7_GOLEM_NATIVE_CAPTURE") {
        let bundle = std::fs::read(
            std::env::var_os("OTERYN_SRV7_MACHINE_BUNDLE").ok_or("Golem actual Bundle path")?,
        )?;
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
            .read_actor_position(golem)
            .map_err(debug)?
            .position();
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema":"OTERYN_SEASONAL_GOLEM_NATIVE_CAPTURE/v1",
                "source":{"producer_test":"srv7_cake_golem_real_machine_runtime_extracts_and_removes_once",
                    "character_id_hex":hex(fence.character_id.as_bytes()),
                    "world_id_hex":hex(admitted.world_id.as_bytes()),
                    "client_content_generation":hex(&runtime.content_pin().client_artifact_digest()),
                    "item_6393_definition_ref":item_ref,
                "item_keys_sha256":hex(&Sha256::digest(&keys)),
                "bundle_sha256":hex(&Sha256::digest(&bundle)),
                "b3_sha256":"2798be76221d6dc00695258d46d1eb375a64e804a7a84265014e2af0c679b05d",
                "production_source_world_preserved":true,
                    "actor_identity_hex":hex(&golem.placement_identity()),
                    "actor_generation":golem.actor_local_generation(),
                    "source_runtime_cell":[source.x,source.y,source.floor],
                    "observer_runtime_cell":[viewer.x,viewer.y,viewer.floor]},
                "initial_frames_hex":hex(&initial),"initial_frames_sha256":hex(&Sha256::digest(&initial))
            }))?,
        )?;
    }
    eprintln!(
        "SRV7_GOLEM_NATIVE_BODY actual_item6393_ref={item_ref} appearance25=true forge104=false"
    );
    Ok(())
}
