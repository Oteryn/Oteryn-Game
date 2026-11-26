//! Unit coverage for applying spatial deltas. The live M02 replay is in Game qualification.

use super::*;
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, DELTA_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
    encode_world_spatial,
};

#[test]
fn spatial_delta_applies_and_malformed_update_leaves_position_unchanged() -> Result<(), BoxError> {
    let expected = ActorPosition {
        x: 101,
        y: 202,
        floor: 7,
    };
    let payload = encode_world_spatial(
        &oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation {
            content_generation: CONTENT_GENERATION,
            actor_position: expected,
        },
    );
    let decoded = oteryn_protocol_oteryn::world_spatial::decode_world_spatial(&payload)
        .map_err(|error| format!("M02 spatial payload: {error:?}"))?;
    let initial = encode_world_spatial(
        &oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation {
            content_generation: decoded.content_generation,
            actor_position: ActorPosition {
                x: expected.x.saturating_sub(3),
                ..expected
            },
        },
    );
    block_on(async move {
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(&mut server, &[13], &[], &[], (1, initial), &[]).await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    5,
                    6,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &payload,
                )?,
            )
            .await?;
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        session.service_liveness(Duration::from_millis(10)).await?;
        assert_eq!(session.world_spatial().actor_position, expected);
        let before = *session.world_spatial();
        assert!(session.apply_state_delta(42, &[0xff]).is_err());
        assert_eq!(*session.world_spatial(), before);
        drop(session);
        peer.await??;
        Ok::<(), BoxError>(())
    })?
}
