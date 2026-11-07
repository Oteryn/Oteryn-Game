//! C01 replay owner: `snapshot_state::SnapshotState::from_join_snapshot` in
//! `crates/session/src/snapshot_state.rs` installs domain 10,
//! `Session::apply_state_delta` in `crates/session/src/domain_dispatch.rs` validates/applies its
//! deltas, and `Session::combat_state` in `session_access.rs` exposes the client-visible result.
use super::*;
use oteryn_protocol_oteryn::attack::{self, ActorCombatState, ChaseMode, FightMode, FightModes};

fn payloads() -> Result<(Vec<u8>, Vec<u8>, String), BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = std::path::Path::new(&directory);
        return Ok((
            std::fs::read(directory.join("c01-snapshot.bin"))?,
            std::fs::read(directory.join("c01-modes.bin"))?,
            std::fs::read_to_string(directory.join("c01-expected.txt"))?,
        ));
    }
    let initial = ActorCombatState {
        target: None,
        modes: FightModes::DEFAULT,
        in_fight: false,
    };
    let changed = ActorCombatState {
        target: None,
        modes: FightModes {
            fight_mode: FightMode::Offensive,
            chase: ChaseMode::Chase,
            secure: true,
        },
        in_fight: false,
    };
    Ok((
        attack::encode_actor_combat_state(&initial).map_err(|error| format!("{error:?}"))?,
        attack::encode_actor_combat_state(&changed).map_err(|error| format!("{error:?}"))?,
        visible(changed),
    ))
}

fn visible(state: ActorCombatState) -> String {
    format!(
        "{} {} {} {}\n",
        state.modes.fight_mode as u8,
        state.modes.chase as u8,
        u8::from(state.modes.secure),
        u8::from(state.in_fight)
    )
}

#[test]
fn e2e_c01_replays_server_combat_state_and_rejects_invalid_delta() -> Result<(), BoxError> {
    block_on(async {
        let (snapshot, delta, expected) = payloads()?;
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                CLIENT_SUPPORTED_CAPABILITIES,
                &[
                    CAPABILITY_WORLD_SPATIAL_ENTITIES,
                    attack::CAPABILITY_ATTACK_V1,
                ],
                &[],
                entity_snapshot(vec![]),
                &[(
                    attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    1,
                    attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
                    snapshot,
                )],
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    41,
                    attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    1,
                    2,
                    attack::DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
                    &delta,
                )?,
            )
            .await?;
            tokio::time::sleep(Duration::from_millis(150)).await;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, entity_admission()?).await?;
        session.service_liveness(Duration::from_millis(50)).await?;
        peer.await??;
        let applied = *session.combat_state().ok_or("combat state absent")?;
        assert_eq!(visible(applied), expected);

        let malformed = encode_state_delta(
            1,
            42,
            attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            2,
            3,
            attack::DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
            &[0xff],
        )?;
        assert!(session.apply_state_delta(42, &malformed).is_err());
        assert_eq!(session.combat_state(), Some(&applied));
        Ok(())
    })?
}
