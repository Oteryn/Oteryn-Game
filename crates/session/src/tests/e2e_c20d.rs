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
        let (spatial_payload, vitals_payload, expected) = if let Some(path) =
            std::env::var_os("E2E_ALL_CAPTURE_DIR")
        {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("c20d-spatial.bin"))?,
                std::fs::read(path.join("c20d-vitals.bin"))?,
                std::fs::read_to_string(path.join("c20d-expected.txt"))?,
            )
        } else {
            let observation = WireSpatialObservation {
                content_generation: [0x73; 32],
                actor_position: ActorPosition {
                    x: 32_369,
                    y: 32_238,
                    floor: -7,
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
                "32369 32238 -7 150 150\n".into(),
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

        let baseline = encode_world_spatial(&WireSpatialObservation {
            content_generation: observation.content_generation,
            actor_position: at(0, 0),
        });
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let joining = tokio::spawn(async move {
            join_peer_with(
                &mut peer,
                &[13],
                &[],
                &[],
                (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, baseline),
                &[(
                    3,
                    3,
                    1,
                    actor_spell::encode_actor_vitals(&super::vitals(150, 55))
                        .map_err(|error| format!("vitals fixture: {error:?}"))?,
                )],
            )
            .await?;
            Ok::<_, BoxError>(peer)
        });
        let mut session = Session::admit(client, admission()?).await?;
        let _peer = joining.await??;
        let spatial = encode_state_delta(
            1,
            41,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial_payload,
        )?;
        session.apply_state_delta(41, decode_wire_envelope(&spatial)?.payload())?;
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
