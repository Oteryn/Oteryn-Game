//! C08 replay owns the session-state consumer: `Session::apply_domain_delta` in
//! `domain_dispatch.rs` decodes/stores ACTOR_VITALS, and `Session::actor_vitals` in
//! `session_access.rs` exposes the client-visible HP/mana a poison DoT and `exana pox` change.
use super::*;

fn parse_expected(text: &str) -> Result<Vec<ActorVitals>, BoxError> {
    text.lines()
        .map(|line| {
            let values = line
                .split_whitespace()
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()?;
            let [health, max_health, mana, max_mana, soul, harmony, serene] = values.as_slice()
            else {
                return Err("invalid C08 expected vitals".into());
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
        })
        .collect()
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

/// Four vitals frames in server order: join, direct poison hit, delayed poison tick, dispel.
fn frames() -> Result<(Vec<Vec<u8>>, Vec<ActorVitals>), BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let mut payloads = Vec::new();
        for name in ["before", "direct", "delayed", "dispelled"] {
            payloads.push(std::fs::read(path.join(format!("c08-{name}.bin")))?);
        }
        return Ok((
            payloads,
            parse_expected(&std::fs::read_to_string(path.join("c08-expected.txt"))?)?,
        ));
    }
    let before = ActorVitals {
        health: 150,
        max_health: 150,
        mana: 55,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    };
    let direct = ActorVitals {
        health: 143,
        ..before
    };
    let delayed = ActorVitals {
        health: 141,
        ..direct
    };
    let dispelled = ActorVitals {
        mana: 40,
        ..delayed
    };
    let expected = vec![before, direct, delayed, dispelled];
    let payloads = expected
        .iter()
        .map(|vitals| {
            actor_spell::encode_actor_vitals(vitals)
                .map_err(|error| format!("C08 fixture: {error:?}").into())
        })
        .collect::<Result<Vec<_>, BoxError>>()?;
    Ok((payloads, expected))
}

#[test]
fn e2e_c08_poison_damage_and_dispel_replay() -> Result<(), BoxError> {
    let (payloads, expected) = frames()?;
    let [join, direct, delayed, dispelled] = payloads.as_slice() else {
        return Err("C08 needs four vitals frames".into());
    };
    let [e_join, e_direct, e_delayed, e_dispelled] = expected.as_slice() else {
        return Err("C08 needs four expected vitals".into());
    };
    assert!(e_direct.health < e_join.health && e_delayed.health < e_direct.health);
    assert_eq!(e_dispelled.health, e_delayed.health);
    assert!(e_dispelled.mana < e_delayed.mana);
    block_on(async {
        let join = join.clone();
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let joining = tokio::spawn(async move {
            send_join_with_vitals(&mut peer, &join).await?;
            Ok::<_, BoxError>(peer)
        });
        let mut session = Session::admit(client, admission()?).await?;
        let mut peer = joining.await??;
        assert_eq!(session.actor_vitals(), Some(e_join));
        for (offset, (payload, want)) in [
            (direct, e_direct),
            (delayed, e_delayed),
            (dispelled, e_dispelled),
        ]
        .into_iter()
        .enumerate()
        {
            let revision = 3 + u64::try_from(offset)?;
            write_frame(
                &mut peer,
                &encode_state_delta(
                    1,
                    41 + u64::try_from(offset)?,
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    revision,
                    revision + 1,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    payload,
                )?,
            )
            .await?;
            session.service_liveness(Duration::from_millis(20)).await?;
            assert_eq!(session.actor_vitals(), Some(want));
        }

        let retained = *session.actor_vitals().ok_or("dispelled vitals absent")?;
        write_frame(
            &mut peer,
            &encode_state_delta(
                1,
                44,
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                6,
                7,
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
