//! SCENARIO_MATRIX C20d replay.
//!
//! The owning client seam is `crates/session/src/domain_dispatch.rs`:
//! `Session::apply_state_delta` applies WORLD_SPATIAL and ACTOR_VITALS payloads, while
//! `crates/session/src/session_access.rs` exposes them through `world_spatial()` and
//! `actor_vitals()` (the same accessors used by `tools/dev-client/src/lib.rs`).

use super::*;
use oteryn_protocol_oteryn::actor_spell::{self, ActorVitals};

#[test]
fn replays_temple_respawn_payloads_and_rejects_invalid_vitals() -> Result<(), BoxError> {
    block_on(async {
        // This unit fixture exercises malformed input. The live row owns original-frame replay.
        let (spatial_payload, vitals_payload, expected) = {
            let observation = WireSpatialObservation {
                content_generation: [0x22; 32],
                actor_position: ActorPosition {
                    x: 32_369,
                    y: 32_238,
                    floor: 7,
                },
            };
            let vitals = ActorVitals {
                health: 150,
                max_health: 150,
                mana: 55,
                max_mana: 55,
                soul: 0,
                harmony: 0,
                serene: false,
            };
            (
                encode_world_spatial(&observation),
                actor_spell::encode_actor_vitals(&vitals).map_err(|error| format!("{error:?}"))?,
                "32369 32238 7 150 150\n",
            )
        };
        let observation = world_spatial::decode_world_spatial(&spatial_payload)
            .map_err(|error| format!("{error:?}"))?;
        let vitals = actor_spell::decode_actor_vitals(&vitals_payload)
            .map_err(|error| format!("{error:?}"))?;
        let visible = format!(
            "{} {} {} {} {}\n",
            observation.actor_position.x,
            observation.actor_position.y,
            observation.actor_position.floor,
            vitals.health,
            vitals.max_health
        );
        assert_eq!(visible, expected);

        // A replay uses the captured World's generation for its synthetic admission.
        let (mut session, _peer) = vitals_pair_with_content(observation.content_generation).await?;
        let spatial = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial_payload,
        )?;
        let applied = session.apply_state_delta(41, decode_wire_envelope(&spatial)?.payload());
        assert!(
            matches!(applied, Ok(SessionEvent::WorldSpatial(_))),
            "{applied:?}"
        );
        let full = encode_state_delta(1, 42, 3, 3, 4, 1, &vitals_payload)?;
        session.apply_state_delta(42, decode_wire_envelope(&full)?.payload())?;
        assert_eq!(session.world_spatial(), &observation);
        assert_eq!(session.actor_vitals(), Some(&vitals));

        let before = *session.actor_vitals().ok_or("replayed vitals")?;
        let malformed = encode_state_delta(1, 43, 3, 4, 5, 1, &[8, 255])?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&malformed)?.payload())
                .is_err()
        );
        assert_eq!(session.actor_vitals(), Some(&before));
        Ok::<_, BoxError>(())
    })?
}

#[test]
fn original_respawn_frames_require_the_live_session_and_complete_sequence() -> Result<(), BoxError>
{
    block_on(async {
        use crate::qualification_replay::{capture_inbound, finish_capture, replay_respawn};
        let temple = ActorPosition {
            x: 12,
            y: 34,
            floor: 7,
        };
        let position = encode_world_spatial(&WireSpatialObservation {
            content_generation: [0x22; 32],
            actor_position: temple,
        });
        let vitals = |health| {
            actor_spell::encode_actor_vitals(&ActorVitals {
                health,
                max_health: 150,
                mana: 55,
                max_mana: 55,
                soul: 0,
                harmony: 0,
                serene: false,
            })
            .map_err(|error| format!("{error:?}"))
        };
        let damaged = encode_state_delta(1, 41, 3, 3, 4, 1, &vitals(60)?)?;
        let moved = encode_state_delta(1, 42, 1, 5, 6, 1, &position)?;
        let healed = encode_state_delta(1, 43, 3, 4, 5, 1, &vitals(150)?)?;
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let sender = tokio::spawn(async move {
            send_vitals_join_with_content(&mut peer, [0x22; 32]).await?;
            for frame in [&damaged, &moved, &healed] {
                write_frame(&mut peer, frame).await?;
            }
            Ok::<_, BoxError>(())
        });
        let (capture, id) = capture_inbound(async {
            let mut session = Session::admit(client, admission()?).await?;
            for _ in 0..3 {
                let _ = read_frame(&mut session.stream).await?;
            }
            Ok::<_, BoxError>((
                finish_capture().ok_or("missing recorded bytes")?,
                session.join_snapshot().game_session_id,
            ))
        })
        .await?;
        sender.await??;
        replay_respawn(capture.clone(), admission()?, id, 43, temple, 150)
            .await
            .map_err(|error| error.to_string())?;
        let other_session = GameSessionId::decode(&uuid_v7(9))?;
        assert!(
            replay_respawn(
                capture.clone(),
                admission()?,
                other_session,
                43,
                temple,
                150
            )
            .await
            .is_err()
        );
        assert!(
            replay_respawn(capture.clone(), admission()?, id, 44, temple, 150)
                .await
                .is_err()
        );
        assert!(
            replay_respawn(
                capture[..capture.len() - 1].to_vec(),
                admission()?,
                id,
                43,
                temple,
                150
            )
            .await
            .is_err()
        );
        Ok::<_, BoxError>(())
    })?
}
