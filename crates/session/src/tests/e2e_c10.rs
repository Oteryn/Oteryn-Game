//! C10 replay owner: `Session::dispatch_delta` in `domain_dispatch.rs` decodes
//! actor-vitals payloads, updates `Session::actor_vitals` and `vitals_revision`,
//! while `Session::actor_vitals` in `session_access.rs` exposes the visible state.
use super::*;
use crate::tests::{BoxError, block_on};

#[test]
fn replays_soul_consumption_regeneration_and_saved_rest() -> Result<(), BoxError> {
    block_on(async {
        let (saved, debit, regenerated, expected) = if let Some(path) =
            std::env::var_os("E2E_ALL_CAPTURE_DIR")
        {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("c10-saved-rest.bin"))?,
                std::fs::read(path.join("c10-debit.bin"))?,
                std::fs::read(path.join("c10-regenerated.bin"))?,
                std::fs::read_to_string(path.join("c10-expected.txt"))?,
            )
        } else {
            let saved = ActorVitals {
                health: 150,
                max_health: 150,
                mana: 55,
                max_mana: 55,
                soul: 10,
                harmony: 0,
                serene: false,
            };
            let debit = ActorVitals { soul: 7, ..saved };
            let regenerated = ActorVitals { soul: 8, ..debit };
            (
                actor_spell::encode_actor_vitals(&saved).map_err(|error| format!("{error:?}"))?,
                actor_spell::encode_actor_vitals(&debit).map_err(|error| format!("{error:?}"))?,
                actor_spell::encode_actor_vitals(&regenerated)
                    .map_err(|error| format!("{error:?}"))?,
                "10 7 8\n".into(),
            )
        };
        let values = expected
            .split_whitespace()
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()?;
        let [saved_soul, debit_soul, regenerated_soul] = values.as_slice() else {
            return Err("invalid C10 expected values".into());
        };
        assert_eq!(
            actor_spell::decode_actor_vitals(&saved)
                .map_err(|error| format!("{error:?}"))?
                .soul,
            *saved_soul
        );

        let (mut session, mut peer) = tests::vitals_pair().await?;
        tests::send_vitals_delta(&mut peer, 41, 3, 4, &debit).await?;
        session.service_liveness(Duration::from_millis(10)).await?;
        assert_eq!(
            session.actor_vitals().ok_or("debit vitals")?.soul,
            *debit_soul
        );
        tests::send_vitals_delta(&mut peer, 42, 4, 5, &regenerated).await?;
        session.service_liveness(Duration::from_millis(10)).await?;
        assert_eq!(
            session.actor_vitals().ok_or("regenerated vitals")?.soul,
            *regenerated_soul
        );

        let retained = *session.actor_vitals().ok_or("retained vitals")?;
        tests::send_vitals_delta(&mut peer, 43, 5, 6, &[8, 255]).await?;
        assert!(
            session
                .service_liveness(Duration::from_millis(10))
                .await
                .is_err()
        );
        assert_eq!(session.actor_vitals(), Some(&retained));
        Ok::<_, BoxError>(())
    })?
}
