//! Player native icons must not turn a valid appearance carrier into a rejected actor.
use super::*;
#[test]
fn srx3_native_creature_icons_real_session_retains_player_outfit_bindings() -> Result<(), BoxError>
{
    block_on(async {
        use oteryn_protocol_oteryn::world_spatial_entities as wire;
        let (appearance, mut entity, _) = appearance_fixture::snapshot()?;
        let EntityDetail::Actor {
            direction,
            appearance_ref,
            health_percent,
        } = entity.detail
        else {
            return Err("fixture actor".into());
        };
        entity.detail = EntityDetail::ActorWithIcons {
            direction,
            appearance_ref,
            health_percent,
            icons: wire::NativeCreatureIcons::new(&[
                wire::NativeCreatureIcon {
                    category: 0,
                    id: 2,
                    count: 0,
                },
                wire::NativeCreatureIcon {
                    category: 0,
                    id: 22,
                    count: 4,
                },
            ])
            .map_err(|error| format!("{error:?}"))?,
        };
        let fields = wire::EntityFields::selected(&[6, wire::CAPABILITY_NATIVE_CREATURE_ICONS_V1]);
        let snapshot = wire::WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: at(0, 0),
            own_identity: OWN,
            entities: vec![entity],
        };
        let bytes = wire::encode_world_spatial_entities_snapshot_with_fields(
            &snapshot,
            fields,
            &wire::EntityLights::new(),
        )
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            wire::decode_world_spatial_entities_snapshot_with_fields(&bytes, fields)
                .map_err(|error| format!("{error:?}"))?
                .0,
            snapshot
        );
        let domain = (
            21,
            1,
            1,
            appearance.encode().map_err(|error| format!("{error:?}"))?,
        );
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[6, 25, wire::CAPABILITY_NATIVE_CREATURE_ICONS_V1],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, bytes),
                &[domain],
            )
            .await?;
            let mut rest = Vec::new();
            server.read_to_end(&mut rest).await?;
            Ok::<_, BoxError>(())
        });
        let session = Session::admit(client, entity_admission()?)
            .await
            .map_err(|error| format!("native icon player admission: {error:?}"))?;
        let visible = session.visible_appearances();
        assert_eq!(visible[0].entity.detail, entity.detail);
        assert!(
            visible[0].look.is_some(),
            "native icons must preserve exact entity appearance binding"
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}
