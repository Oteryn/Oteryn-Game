//! M02 replay covers the session state owner used by `DevClientSession::world_spatial`.
//! The production application path is `domain_dispatch.rs::Session::apply_state_delta`;
//! reads are exposed by `session_access.rs::Session::world_spatial` and
//! `tools/dev-client/src/lib.rs::DevClientSession::world_spatial`.

use super::*;
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, DELTA_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
    encode_world_spatial,
};

#[test]
fn replay_far_action_autowalk_position() -> Result<(), BoxError> {
    let (payload, expected) = if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let payload = std::fs::read(root.join("m02-world-spatial.bin"))?;
        let expected = std::fs::read_to_string(root.join("m02-expected.txt"))?;
        (payload, expected)
    } else {
        let position = ActorPosition {
            x: 101,
            y: 202,
            floor: 7,
        };
        (
            encode_world_spatial(
                &oteryn_protocol_oteryn::world_spatial::WorldSpatialObservation {
                    content_generation: CONTENT_GENERATION,
                    actor_position: position,
                },
            ),
            "101 202 7\n".into(),
        )
    };
    let coordinates = expected
        .split_whitespace()
        .map(str::parse::<i32>)
        .collect::<Result<Vec<_>, _>>()?;
    let [x, y, floor] = coordinates.as_slice() else {
        return Err("invalid M02 expected position".into());
    };
    let expected = ActorPosition {
        x: *x,
        y: *y,
        floor: i16::try_from(*floor)?,
    };
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
