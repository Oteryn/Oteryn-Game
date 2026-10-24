#[test]
fn domain_1_is_the_interest_set_with_capability_6_and_the_own_actor_without() -> TestResult {
    run(async {
        let result = encode_command_result(
            1,
            1,
            1,
            CommandStatus::Accepted,
            &encode_step_result(StepDisposition::Moved),
        )?;
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let creature = wire(EntityKind::Creature, [9; 16], at(102, 100));
        let mut expected =
            snapshot_frames(1, 1, 0, 1, &selected, vec![own(at(100, 100)), creature]);
        expected.push(result.clone());
        expected.push(delta_frame(
            2,
            1,
            &selected,
            &WorldSpatialEntitiesDelta {
                content_generation: CONTENT,
                actor_position: at(101, 100),
                enter: Vec::new(),
                update: vec![own(at(101, 100))],
                leave: Vec::new(),
            },
        ));
        assert_eq!(join_and_step(&selected).await?, expected);

        // Without capability 6: the v1 own-actor snapshot and delta, as before VIS-3.
        let at_start = encode_world_spatial(&WorldSpatialObservation {
            content_generation: CONTENT,
            actor_position: at(100, 100),
        });
        let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &at_start,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &[],
                },
            ],
        )?
        .to_vec();
        expected.push(result);
        expected.push(encode_state_delta(
            1,
            2,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            1,
            2,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &encode_world_spatial(&WorldSpatialObservation {
                content_generation: CONTENT,
                actor_position: at(101, 100),
            }),
        )?);
        assert_eq!(join_and_step(&[]).await?, expected);
        Ok(())
    })
}

#[test]
fn others_moving_send_a_delta_and_a_change_above_256_a_resync_on_resume() -> TestResult {
    run(async {
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
        let crowd: Vec<[u8; 16]> = (0..257_u16)
            .map(|n| {
                let mut identity = [0x40; 16];
                identity[14..].copy_from_slice(&n.to_be_bytes());
                identity
            })
            .collect();
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let admitted = session(&selected, SessionContinuity::FRESH);
        let serving = serve_admitted(&mut server, admitted, &channel, IDLE_LIVENESS);
        let observing = async {
            let mut snapshot = Vec::new();
            for _ in 0..3 {
                snapshot.push(read_one(&mut client).await?);
            }
            assert_eq!(
                snapshot,
                snapshot_frames(
                    1,
                    1,
                    0,
                    1,
                    &selected,
                    vec![
                        own(at(100, 100)),
                        wire(EntityKind::Creature, [9; 16], at(102, 100))
                    ],
                )
            );
            // The creature steps: one delta on the next refresh, sequence 1.
            channel.others.borrow_mut()[0] = creature([9; 16], at(103, 100), 1);
            assert_eq!(
                read_one(&mut client).await?,
                delta_frame(
                    1,
                    1,
                    &selected,
                    &WorldSpatialEntitiesDelta {
                        content_generation: CONTENT,
                        actor_position: at(100, 100),
                        enter: Vec::new(),
                        update: vec![wire(EntityKind::Creature, [9; 16], at(103, 100))],
                        leave: Vec::new(),
                    },
                )
            );
            // 200 creatures replace it: 201 changes, one delta (sequence 2, revision 3).
            let identities = |tag: u8, count: u16| -> Vec<[u8; 16]> {
                (0..count)
                    .map(|n| {
                        let mut identity = [tag; 16];
                        identity[14..].copy_from_slice(&n.to_be_bytes());
                        identity
                    })
                    .collect()
            };
            *channel.others.borrow_mut() = identities(0x30, 200)
                .into_iter()
                .map(|identity| creature(identity, at(99, 100), 0))
                .collect();
            let mut entered = WorldSpatialEntitiesDelta {
                content_generation: CONTENT,
                actor_position: at(100, 100),
                enter: identities(0x30, 200)
                    .into_iter()
                    .map(|identity| wire(EntityKind::Creature, identity, at(99, 100)))
                    .collect(),
                update: Vec::new(),
                leave: vec![EntityRef {
                    identity: [9; 16],
                    generation: 1,
                }],
            };
            entered.enter.sort_by_key(|entity| entity.entity.identity);
            assert_eq!(
                read_one(&mut client).await?,
                delta_frame(2, 2, &selected, &entered)
            );
            // 257 others replace those: 200 leave and the nearest 255 enter, more than 256
            // changes, so a resync: no mid-session snapshot (the admitted client accepts
            // none), the connection ends at the last delivered sequence and revision.
            *channel.others.borrow_mut() = crowd
                .iter()
                .map(|identity| creature(*identity, at(99, 100), 0))
                .collect();
            Ok::<_, Box<dyn Error>>(())
        };
        let (end, observed) = {
            let (mut serving, mut observing) =
                (std::pin::pin!(serving), std::pin::pin!(observing));
            let (mut end, mut observed) = (None, None);
            std::future::poll_fn(|context| {
                if end.is_none()
                    && let std::task::Poll::Ready(done) = serving.as_mut().poll(context)
                {
                    end = Some(done);
                }
                if observed.is_none()
                    && let std::task::Poll::Ready(done) = observing.as_mut().poll(context)
                {
                    observed = Some(done);
                }
                if end.is_some() && observed.is_some() {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
            (end, observed)
        };
        observed.ok_or("observed")??;
        let end = end.ok_or("ended")?;
        drop(server);
        assert!(
            read_one(&mut client).await.is_err(),
            "nothing after the last delta"
        );
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.spatial_revision, 3);
        assert_eq!(ended.continuity.server_sequence, 2);
        // The resume of that continuity is the resync: a snapshot of the own actor and the
        // 255 lowest identities (the degrade disposition) at sequence 2 and revision 3.
        let resumed = SessionContinuity {
            connection_generation: 2,
            ..ended.continuity
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        client.shutdown().await?;
        let end = serve_admitted(
            &mut server,
            session(&selected, resumed),
            &channel,
            IDLE_LIVENESS,
        )
        .await;
        drop(server);
        let mut frames = Vec::new();
        while let Ok(frame) = read_one(&mut client).await {
            frames.push(frame);
        }
        let mut nearest = vec![own(at(100, 100))];
        nearest.extend(
            crowd[..255]
                .iter()
                .map(|identity| wire(EntityKind::Creature, *identity, at(99, 100))),
        );
        assert_eq!(frames, snapshot_frames(2, 1, 2, 3, &selected, nearest));
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        Ok(())
    })
}

#[test]
fn a_resumed_connection_keeps_the_selection_and_gets_a_snapshot_of_the_interest_set()
-> TestResult {
    run(async {
        let selected = [CAPABILITY_WORLD_SPATIAL_ENTITIES];
        let channel = Channel::new(vec![creature([9; 16], at(102, 100), 0)]);
        let resumed = SessionContinuity {
            connection_generation: 2,
            next_command_id: 4,
            server_sequence: 6,
            spatial_revision: 7,
            ..SessionContinuity::FRESH
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        client.shutdown().await?;
        let end = serve_admitted(
            &mut server,
            session(&selected, resumed),
            &channel,
            IDLE_LIVENESS,
        )
        .await;
        drop(server);
        let mut frames = Vec::new();
        while let Ok(frame) = read_one(&mut client).await {
            frames.push(frame);
        }
        assert_eq!(
            frames,
            snapshot_frames(
                2,
                1,
                6,
                7,
                &selected,
                vec![
                    own(at(100, 100)),
                    wire(EntityKind::Creature, [9; 16], at(102, 100))
                ],
            )
        );
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.selected_capabilities.as_slice(), selected);
        Ok(())
    })
}

#[test]
fn a_session_with_capability_6_whose_channel_cannot_be_read_fails_closed() -> TestResult {
    struct Unreadable;
    impl FreshAdmissionAuthority for Unreadable {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Unavailable)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(WorldSpatialObservation {
                content_generation: CONTENT,
                actor_position: at(100, 100),
            })
        }

        async fn observe_visible_entities(
            &self,
            _actor: ExactActorRef,
        ) -> Option<ChannelEntities> {
            None
        }
    }
    run(async {
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        client.shutdown().await?;
        let end = serve_admitted(
            &mut server,
            session(
                &[CAPABILITY_WORLD_SPATIAL_ENTITIES],
                SessionContinuity::FRESH,
            ),
            &Unreadable,
            IDLE_LIVENESS,
        )
        .await;
        drop(server);
        assert!(read_one(&mut client).await.is_err());
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        Ok(())
    })
}
