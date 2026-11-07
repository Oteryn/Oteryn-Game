//! M11b replay owner: `Session::apply_state_delta` in `crates/session/src/domain_dispatch.rs`
//! applies `WORLD_SPATIAL` and `ACTOR_VITALS`; `Session::world_spatial` and
//! `Session::actor_vitals` in `crates/session/src/session_access.rs` expose the result.
use super::*;
use crate::tests::{admission_with, block_on, join_peer_with};
use oteryn_protocol_oteryn::{actor_spell, encode_state_delta, world_spatial};

#[test]
fn e2e_m11b_magic_rope_replay() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (spatial_before, spatial_delta, vitals_before, vitals_delta, expected) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("m11b-spatial-before.bin"))?,
                std::fs::read(path.join("m11b-spatial-delta.bin"))?,
                std::fs::read(path.join("m11b-vitals-before.bin"))?,
                std::fs::read(path.join("m11b-vitals-delta.bin"))?,
                std::fs::read_to_string(path.join("m11b-expected.txt"))?,
            )
        } else {
            let generation = [0x11; 32];
            let spatial = |x, y, floor| {
                world_spatial::encode_world_spatial(&world_spatial::WorldSpatialObservation {
                    content_generation: generation,
                    actor_position: world_spatial::ActorPosition { x, y, floor },
                })
            };
            let vitals = |mana| {
                actor_spell::encode_actor_vitals(&actor_spell::ActorVitals {
                    health: 150,
                    max_health: 150,
                    mana,
                    max_mana: 100,
                    soul: 100,
                    harmony: 0,
                    serene: false,
                })
                .map_err(|error| format!("vitals fixture: {error:?}"))
            };
            (
                spatial(10, 20, 7),
                spatial(11, 20, 8),
                vitals(80)?,
                vitals(60)?,
                "11 20 8 150 60\n".into(),
            )
        };
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[],
                &[],
                &[],
                (
                    world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    spatial_before,
                ),
                &[(
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    3,
                    actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    vitals_before,
                )],
            )
            .await
        });
        let mut session = Session::admit(client, admission_with(&[])?).await?;
        peer.await??;
        let spatial_wire = encode_state_delta(
            1,
            41,
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            5,
            6,
            world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial_delta,
        )?;
        let vitals_wire = encode_state_delta(
            1,
            42,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &vitals_delta,
        )?;
        session.apply_state_delta(41, decode_wire_envelope(&spatial_wire)?.payload())?;
        session.apply_state_delta(42, decode_wire_envelope(&vitals_wire)?.payload())?;
        let position = session.world_spatial().actor_position;
        let vitals = session.actor_vitals().ok_or("missing replayed vitals")?;
        assert_eq!(
            format!(
                "{} {} {} {} {}\n",
                position.x, position.y, position.floor, vitals.health, vitals.mana
            ),
            expected
        );

        let before = (*session.world_spatial(), session.actor_vitals().cloned());
        let invalid = encode_state_delta(
            1,
            43,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            4,
            5,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&invalid)?.payload())
                .is_err()
        );
        assert_eq!(
            (*session.world_spatial(), session.actor_vitals().cloned()),
            before
        );
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })??;
    Ok(())
}
