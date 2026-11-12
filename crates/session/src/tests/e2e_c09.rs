//! C09 replay owns the session-state consumer: `Session::apply_domain_delta` in
//! `domain_dispatch.rs` decodes/stores ACTOR_VITALS, and `Session::actor_vitals` in
//! `session_access.rs` exposes the client-visible HP/mana used by `DevClientSession`.
use super::*;

fn parse_expected(text: &str) -> Result<ActorVitals, BoxError> {
    let values = text
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let [health, max_health, mana, max_mana, soul, harmony, serene] = values.as_slice() else {
        return Err("invalid C09 expected vitals".into());
    };
    Ok(ActorVitals {
        health: *health,
        max_health: *max_health,
        mana: *mana,
        max_mana: *max_mana,
        soul: *soul,
        harmony: *harmony,
        serene: *serene != 0,
    })
}

async fn send_join_with_vitals(stream: &mut DuplexStream, payload: &[u8]) -> Result<(), BoxError> {
    read_frame(&mut *stream).await?;
    write_frame(
        &mut *stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let overlay = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial(0),
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            DomainSnapshot {
                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 3,
                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload,
            },
        ],
    )? {
        write_frame(&mut *stream, &frame).await?;
    }
    Ok(())
}

#[test]
fn e2e_c09_food_regeneration_replay() -> Result<(), BoxError> {
    let (before_payload, regenerated_payload, expected) =
        if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                std::fs::read(path.join("c09-before.bin"))?,
                std::fs::read(path.join("c09-regenerated.bin"))?,
                parse_expected(&std::fs::read_to_string(path.join("c09-expected.txt"))?)?,
            )
        } else {
            let before = ActorVitals {
                health: 91,
                max_health: 150,
                mana: 41,
                max_mana: 55,
                soul: 100,
                harmony: 0,
                serene: false,
            };
            let regenerated = ActorVitals {
                health: 92,
                mana: 42,
                ..before
            };
            (
                actor_spell::encode_actor_vitals(&before)
                    .map_err(|error| format!("before fixture: {error:?}"))?,
                actor_spell::encode_actor_vitals(&regenerated)
                    .map_err(|error| format!("regenerated fixture: {error:?}"))?,
                regenerated,
            )
        };
    block_on(async {
        let before = actor_spell::decode_actor_vitals(&before_payload)
            .map_err(|error| format!("before payload: {error:?}"))?;
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let joining = tokio::spawn(async move {
            send_join_with_vitals(&mut peer, &before_payload).await?;
            Ok::<_, BoxError>(peer)
        });
        let mut session = Session::admit(client, admission()?).await?;
        let mut peer = joining.await??;
        assert_eq!(session.actor_vitals(), Some(&before));
        write_frame(
            &mut peer,
            &encode_state_delta(
                1,
                41,
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                3,
                4,
                actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                &regenerated_payload,
            )?,
        )
        .await?;
        session.service_liveness(Duration::from_millis(20)).await?;
        assert_eq!(session.actor_vitals(), Some(&expected));

        let retained = *session.actor_vitals().ok_or("regenerated vitals absent")?;
        write_frame(
            &mut peer,
            &encode_state_delta(
                1,
                42,
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                4,
                5,
                actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                &[8, 255],
            )?,
        )
        .await?;
        assert!(
            session
                .service_liveness(Duration::from_millis(20))
                .await
                .is_err()
        );
        assert_eq!(session.actor_vitals(), Some(&retained));
        Ok::<_, BoxError>(())
    })?
}
