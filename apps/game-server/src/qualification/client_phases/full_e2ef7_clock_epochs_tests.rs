//! Offset attribution through the real TLS/session receive queue.
use super::*;
use oteryn_protocol_oteryn::world_clock::{
    DELTA_TYPE_WORLD_CLOCK_V1, SNAPSHOT_TYPE_WORLD_CLOCK_V1, STATE_DOMAIN_WORLD_CLOCK, WorldClock,
    encode_world_clock,
};
use tokio::sync::Notify;

fn reading(minute: u16) -> WorldClock {
    WorldClock {
        tibian_minute: minute,
        light_level: expected_light(u32::from(minute)),
        light_color: 215,
    }
}

#[test]
fn clock_epoch_matching_is_unique_monotonic_and_non_mutating() -> TestResult {
    let mut row = ClockRow::default();
    row.observe(reading(329), Duration::ZERO)?;
    let mut epochs = ClockEpochs::new(Duration::ZERO, Duration::ZERO);
    epochs.record(qualification_step())?;
    epochs.record(qualification_step() * 2)?;
    epochs.observe(&mut row, reading(360), Duration::from_millis(2_500))?;
    assert_eq!(epochs.observed, 1);
    assert!(epochs.delta_owed(reading(360)));
    epochs.observe(&mut row, reading(390), Duration::from_millis(2_500))?;
    assert_eq!(epochs.observed, 2);
    assert!(!epochs.delta_owed(reading(390)));
    assert_eq!(row.elapsed_minutes, 61);
    assert!(row.passed.contains("sunrise"));

    let before = (
        row.previous,
        row.elapsed_minutes,
        row.first_observation,
        row.passed.clone(),
    );
    // A prior epoch cannot be used after the current epoch was accepted.
    assert!(
        epochs
            .observe(&mut row, reading(360), Duration::from_secs(3))
            .is_err()
    );
    // Neither frozen time nor a guessed rate can manufacture a matching epoch.
    for (clock, received) in [
        (reading(390), Duration::from_secs(80)),
        (reading(600), Duration::from_secs(3)),
        (reading(391), Duration::ZERO),
        (
            WorldClock {
                light_color: 0,
                ..reading(391)
            },
            Duration::from_secs(5),
        ),
    ] {
        assert!(epochs.observe(&mut row, clock, received).is_err());
    }
    assert_eq!(epochs.observed, 2);
    assert_eq!(
        (
            row.previous,
            row.elapsed_minutes,
            row.first_observation,
            row.passed.clone(),
        ),
        before
    );
    assert!(epochs.record(qualification_step()).is_err());
    assert!(epochs.record(qualification_step() * 3).is_err());
    assert_eq!(epochs.len, 3);

    // The real epochs are 75s apart (> twice the 10s tolerance). If an
    // unexpected input makes attribution ambiguous, refuse it without evidence.
    let mut ambiguous = ClockEpochs::new(Duration::ZERO, Duration::ZERO);
    ambiguous.record(Duration::from_secs(5))?;
    let mut row = ClockRow::default();
    row.observe(reading(0), Duration::ZERO)?;
    let before = (row.previous, row.elapsed_minutes, row.passed.clone());
    let error = ambiguous
        .observe(&mut row, reading(2), Duration::ZERO)
        .expect_err("two independently valid candidates must not be guessed");
    assert!(error.to_string().contains("multiple offset epochs"));
    assert_eq!((row.previous, row.elapsed_minutes, row.passed), before);
    assert_eq!(ambiguous.observed, 0);
    Ok(())
}

#[test]
fn clock_epoch_rejection_keeps_cumulative_evidence_unchanged() -> TestResult {
    let mut row = ClockRow::default();
    row.observe(reading(0), Duration::ZERO)?;
    for minute in 1..=4_u16 {
        row.observe(reading(minute), Duration::from_secs(u64::from(minute) * 5))?;
    }
    let before = (row.previous, row.elapsed_minutes, row.passed.clone());
    assert!(row.observe(reading(5), Duration::from_secs(25)).is_err());
    assert_eq!((row.previous, row.elapsed_minutes, row.passed), before);
    Ok(())
}

#[test]
fn clock_epochs_keep_natural_time_and_fresh_admission_validation() -> TestResult {
    let origin = Duration::from_secs(37);
    let mut row = ClockRow::default();
    row.observe(reading(359), Duration::ZERO)?;
    let mut natural = ClockEpochs::new(origin, origin);
    for minute in 360..=400_u16 {
        natural.record(origin)?;
        natural.observe(
            &mut row,
            reading(minute),
            Duration::from_millis(u64::from(minute - 359) * 2_500),
        )?;
    }
    assert_eq!(natural.len, 1, "silent service turns do not consume epochs");
    assert!(row.passed.contains("sunrise"));

    let mut row = ClockRow::default();
    row.observe(reading(1_409), Duration::ZERO)?;
    let mut old = ClockEpochs::new(origin, origin);
    old.record(origin + qualification_step())?;
    old.record(origin + qualification_step() * 2)?;
    old.observe(&mut row, reading(0), Duration::from_millis(2_500))?;
    assert_eq!(old.observed, 1);
    assert!(!old.delta_owed(reading(0)));
    assert!(row.passed.contains("midnight-wrap"));

    // A fresh admission resets only the bounded ledger, not global rate or
    // cumulative evidence; it cannot import an old socket's epoch.
    let admitted_offset = origin + qualification_step() * 2;
    row.observe(reading(31), Duration::from_secs(155))?;
    let mut fresh = ClockEpochs::new(origin, admitted_offset);
    assert_eq!(fresh.len, 1);
    assert_eq!(row.elapsed_minutes, 62);
    assert!(
        fresh
            .observe(&mut row, reading(0), Duration::from_secs(5))
            .is_err()
    );
    fresh.record(admitted_offset + qualification_step())?;
    fresh.observe(&mut row, reading(61), Duration::from_secs(5))?;
    assert_eq!(fresh.observed, 1);
    assert_eq!(row.elapsed_minutes, 92);
    assert!(
        !row.complete(),
        "transport and the complete day remain required"
    );
    Ok(())
}

fn run_transport<F, Fut>(scenario: F) -> TestResult
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = TestResult>,
{
    let thread = std::thread::Builder::new()
        .name("clock-epoch-tls".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(async {
                    tokio::time::timeout(Duration::from_secs(20), scenario())
                        .await
                        .map_err(|_| "clock epoch TLS fixture deadline")?
                }))
            })();
            result.map_err(|error| error.to_string())
        })?;
    match thread.join() {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

async fn send(stream: &mut ServerStream, frame: &[u8]) -> TestResult {
    stream.write_all(&framed(frame)).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_clock(stream: &mut ServerStream, revision: u64, clock: WorldClock) -> TestResult {
    let payload = encode_world_clock(clock).map_err(|error| format!("clock payload: {error:?}"))?;
    send(
        stream,
        &native::encode_state_delta(
            1,
            revision,
            STATE_DOMAIN_WORLD_CLOCK,
            revision - 1,
            revision,
            DELTA_TYPE_WORLD_CLOCK_V1,
            &payload,
        )?,
    )
    .await
}

async fn probe(stream: &mut ServerStream, id: u64, sequence: u64) -> TestResult {
    send(stream, &native::encode_liveness_probe(1, id)?).await?;
    let bytes = light_read(stream).await?;
    let ack = native::decode_wire_envelope(&bytes)?.liveness_ack(1)?;
    assert_eq!(ack.probe_id, id);
    assert_eq!(ack.last_applied_server_sequence, Some(sequence));
    Ok(())
}

async fn wait_for_close(stream: &mut ServerStream) {
    let mut byte = [0];
    let _ = stream.read(&mut byte).await;
}

async fn connect_peer<F, Fut>(
    initial: WorldClock,
    marker: u8,
    script: F,
) -> TestResult<(DevClientSession, tokio::task::JoinHandle<TestResult>)>
where
    F: FnOnce(ServerStream) -> Fut + 'static,
    Fut: std::future::Future<Output = TestResult> + 'static,
{
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    config.alpn_protocols = vec![native::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::task::spawn_local(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        let bootstrap = light_read(&mut stream).await?;
        native::decode_wire_envelope(&bootstrap)?.client_bootstrap()?;
        send(
            &mut stream,
            &native::encode_server_accepted(&native::ServerAcceptedValue {
                game_session_id: native::GameSessionId::decode(&v7(marker, 1))?,
                world_id: native::WorldId::decode(&v7(2, 2))?,
                channel_id: native::ChannelId::decode(&v7(3, 3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: &[6, 19],
            })?,
        )
        .await?;
        let position = ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        let own = v7(4, 4);
        let entities = spatial::encode_world_spatial_entities_snapshot_with_fields(
            &spatial::WorldSpatialEntitiesSnapshot {
                content_generation: [0x11; 32],
                actor_position: position,
                own_identity: own,
                entities: vec![spatial::WorldSpatialEntity {
                    display_name: None,
                    kind: spatial::EntityKind::Player,
                    entity: spatial::EntityRef {
                        identity: own,
                        generation: 1,
                    },
                    position,
                    detail: spatial::EntityDetail::Actor {
                        direction: StepDirection::South,
                        appearance_ref: 1,
                        health_percent: 100,
                    },
                }],
            },
            spatial::EntityFields::selected(&[6, 19]),
            &spatial::EntityLights::new(),
        )
        .map_err(|error| format!("entities snapshot: {error:?}"))?;
        let overlay = native::world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("overlay snapshot: {error:?}"))?;
        let clock =
            encode_world_clock(initial).map_err(|error| format!("clock snapshot: {error:?}"))?;
        for frame in native::encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 0,
                    snapshot_type: spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    payload: &entities,
                },
                DomainSnapshot {
                    domain_id: native::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: native::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_CLOCK,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_CLOCK_V1,
                    payload: &clock,
                },
            ],
        )? {
            send(&mut stream, &frame).await?;
        }
        script(stream).await
    });
    let client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&v7(4, 4))?,
        admission_material: b"clock-epoch-fixture",
        client_build_id: "clock-epoch-tls-test",
        deadline: Duration::from_secs(5),
    })
    .await?;
    Ok((client, server))
}

#[test]
fn clock_offset_epochs_attribute_a_queued_plateau_tls_frame() -> TestResult {
    run_transport(|| queued_frame(1, 29, 60))
}

#[test]
fn clock_offset_epochs_wait_for_current_sunrise_after_a_queued_tls_frame() -> TestResult {
    run_transport(|| queued_frame(2, 329, 360))
}

#[test]
fn clock_offset_epochs_attribute_a_queued_midnight_tls_frame() -> TestResult {
    run_transport(|| queued_frame(3, 1_409, 0))
}

async fn queued_frame(marker: u8, initial: u16, delayed: u16) -> TestResult {
    // The natural transition follows a silent step but is already queued
    // before the second advance, independently of the client's new offset.
    let release = Arc::new(Notify::new());
    let queued = Arc::new(Notify::new());
    let current = Arc::new(Notify::new());
    let (release_peer, queued_peer, current_peer) =
        (release.clone(), queued.clone(), current.clone());
    let requires_current = initial == 329;
    let (mut client, peer) = connect_peer(reading(initial), marker, move |mut stream| async move {
        release_peer.notified().await;
        send_clock(&mut stream, 1, reading(delayed)).await?;
        queued_peer.notify_one();
        probe(&mut stream, 1, 1).await?;
        if requires_current {
            current_peer.notified().await;
            send_clock(&mut stream, 2, reading(390)).await?;
            probe(&mut stream, 2, 2).await?;
        }
        wait_for_close(&mut stream).await;
        Ok(())
    })
    .await?;
    let started = std::time::Instant::now();
    let original_session = client.join_snapshot().game_session_id;
    let mut row = ClockRow::default();
    row.observe(admitted_clock(&mut client)?, started.elapsed())?;
    let mut epochs = ClockEpochs::new(Duration::ZERO, Duration::ZERO);
    epochs.record(qualification_step())?;
    service_clock_advance(&mut client, &mut row, &mut epochs, started, true).await?;
    assert_eq!(row.previous.map(|sample| sample.0), Some(reading(initial)));
    assert_eq!(epochs.observed, 0);

    release.notify_one();
    queued.notified().await;
    // Flush on the server precedes the new epoch. The client has not yet
    // read that frame, just as at the boundary of service_liveness windows.
    epochs.record(qualification_step() * 2)?;
    {
        let wait = service_clock_advance(&mut client, &mut row, &mut epochs, started, true);
        tokio::pin!(wait);
        if requires_current {
            assert!(
                tokio::time::timeout(Duration::from_millis(1_200), &mut wait)
                    .await
                    .is_err(),
                "the old sunrise frame cannot acknowledge the newer advance"
            );
            current.notify_one();
        }
        wait.await?;
    }
    assert_eq!(epochs.observed, if requires_current { 2 } else { 1 });
    assert_eq!(row.elapsed_minutes, if requires_current { 61 } else { 31 });
    assert!(row.passed.contains("clock-delta"));
    assert!(!epochs.delta_owed(row.previous.ok_or("last clock")?.0));
    assert!(!row.complete());
    if initial == 1_409 {
        assert!(row.passed.contains("midnight-wrap"));
    }
    drop(client);
    peer.await??;

    if requires_current {
        // Fresh real TLS admission must continue the row's independent rate
        // check while discarding the previous connection's epoch ledger.
        let (mut fresh, peer) = connect_peer(reading(391), 4, |mut stream| async move {
            wait_for_close(&mut stream).await;
            Ok(())
        })
        .await?;
        assert_ne!(fresh.join_snapshot().game_session_id, original_session);
        let admitted = qualification_step() * 2;
        row.observe(admitted_clock(&mut fresh)?, started.elapsed() + admitted)?;
        let mut epochs = ClockEpochs::new(Duration::ZERO, admitted);
        assert!(
            epochs
                .observe(&mut row, reading(delayed), started.elapsed())
                .is_err()
        );
        assert_eq!(epochs.observed, 0);
        assert_eq!(row.elapsed_minutes, 62);
        drop(fresh);
        peer.await??;
    }
    Ok(())
}

#[test]
fn old_clock_tls_frames_neither_complete_nor_renew_the_new_epoch_deadline() -> TestResult {
    run_transport(old_frames_deadline)
}

async fn old_frames_deadline() -> TestResult {
    let release = Arc::new(Notify::new());
    let queued = Arc::new(Notify::new());
    let (release_peer, queued_peer) = (release.clone(), queued.clone());
    let (mut client, peer) = connect_peer(reading(329), 5, move |mut stream| async move {
        release_peer.notified().await;
        send_clock(&mut stream, 1, reading(360)).await?;
        queued_peer.notify_one();
        probe(&mut stream, 1, 1).await?;
        for revision in 2..=3 {
            tokio::time::sleep(Duration::from_secs(3)).await;
            send_clock(&mut stream, revision, reading(360)).await?;
            probe(&mut stream, revision, revision).await?;
        }
        wait_for_close(&mut stream).await;
        Ok(())
    })
    .await?;
    let started = std::time::Instant::now();
    let mut row = ClockRow::default();
    row.observe(admitted_clock(&mut client)?, started.elapsed())?;
    let mut epochs = ClockEpochs::new(Duration::ZERO, Duration::ZERO);
    epochs.record(qualification_step())?;
    service_clock_advance(&mut client, &mut row, &mut epochs, started, true).await?;
    release.notify_one();
    queued.notified().await;
    epochs.record(qualification_step() * 2)?;
    let wait_started = tokio::time::Instant::now();
    let error = tokio::time::timeout(
        Duration::from_secs(10),
        service_clock_advance(&mut client, &mut row, &mut epochs, started, true),
    )
    .await
    .map_err(|_| "old clock frames renewed the original 8s deadline")?
    .expect_err("old frames cannot satisfy a still-owed current-epoch delta");
    assert_eq!(
        error.to_string(),
        "world-clock advance did not produce a server delta"
    );
    assert!(wait_started.elapsed() >= Duration::from_secs(8));
    assert_eq!(epochs.observed, 1);
    assert!(epochs.delta_owed(reading(360)));
    assert_eq!(row.elapsed_minutes, 31);
    assert!(!row.complete());
    drop(client);
    peer.await??;
    Ok(())
}
