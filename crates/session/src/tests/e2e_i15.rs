//! SCENARIO_MATRIX I15 replay: Energy Ring damage is projected through the actor-vitals
//! payload. The owning client seam is `Session::apply_state_delta` in
//! `crates/session/src/domain_dispatch.rs`; it decodes and stores the payload, and
//! `Session::actor_vitals` in `crates/session/src/session_access.rs` exposes that state to
//! `tools/dev-client/src/lib.rs`. The client-ui and client-runtime feature directories were
//! also searched; neither owns this payload.

use super::*;

#[test]
fn e2e_i15_energy_ring_vitals_replay() -> Result<(), BoxError> {
    block_on(async {
        let (before_payload, after_payload, expected) =
            if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let path = std::path::Path::new(&path);
                (
                    std::fs::read(path.join("i15-before.bin"))?,
                    std::fs::read(path.join("i15-after.bin"))?,
                    std::fs::read_to_string(path.join("i15-expected.txt"))?,
                )
            } else {
                (
                    actor_spell::encode_actor_vitals(&vitals(150, 60))
                        .map_err(|error| format!("before fixture: {error:?}"))?,
                    actor_spell::encode_actor_vitals(&vitals(150, 50))
                        .map_err(|error| format!("after fixture: {error:?}"))?,
                    "150 50\n".into(),
                )
            };
        let expected = expected
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        let [expected_health, expected_mana] = expected.as_slice() else {
            return Err("invalid I15 expected vitals".into());
        };

        let (mut session, mut peer) = vitals_pair().await?;
        write_frame(
            &mut peer,
            &vitals_frame(1, 41, 3, 3, 4, 1, &before_payload)?,
        )
        .await?;
        session.service_liveness(Duration::from_millis(10)).await?;
        let before = *session.actor_vitals().ok_or("I15 before vitals absent")?;

        write_frame(&mut peer, &vitals_frame(1, 42, 3, 4, 5, 1, &after_payload)?).await?;
        session.service_liveness(Duration::from_millis(10)).await?;
        let after = *session.actor_vitals().ok_or("I15 after vitals absent")?;
        assert_eq!(
            (after.health, after.mana),
            (*expected_health, *expected_mana)
        );
        assert_eq!(after.health, before.health);
        assert!(before.mana > after.mana);
        assert_eq!((before.mana - after.mana) % 2, 0);

        write_frame(&mut peer, &vitals_frame(1, 43, 3, 5, 6, 1, &[255])?).await?;
        assert!(
            session
                .service_liveness(Duration::from_millis(10))
                .await
                .is_err()
        );
        assert_eq!(session.actor_vitals(), Some(&after));
        Ok::<_, BoxError>(())
    })?
}
