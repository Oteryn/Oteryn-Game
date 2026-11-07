#[test]
fn liveness_counts_only_the_current_probe_and_refuses_unsent_ids() {
    let mut liveness = Liveness::new(FAST);
    assert_eq!(
        liveness.ack(1),
        Err(FoundationProtocolError::InvalidWireIdentifier)
    );
    assert_eq!(liveness.tick(), Ok(1));
    assert_eq!(liveness.tick(), Ok(2));
    // A late ack of probe 1 restores nothing; probe 2 is still missed.
    assert_eq!(liveness.ack(1), Ok(()));
    assert_eq!(liveness.tick(), Ok(3));
    assert_eq!(liveness.ack(3), Ok(()));
    assert_eq!(liveness.ack(3), Ok(()));
    assert_eq!(
        liveness.ack(0),
        Err(FoundationProtocolError::InvalidWireIdentifier)
    );
    // After an answered probe the count restarts: three more misses are needed.
    assert_eq!(liveness.tick(), Ok(4));
    assert_eq!(liveness.tick(), Ok(5));
    assert_eq!(liveness.tick(), Ok(6));
    assert_eq!(liveness.tick(), Err(LivenessEnd::ControlLost));
    let mut exhausted = Liveness::new(FAST);
    exhausted.last_sent = u64::MAX;
    assert_eq!(exhausted.tick(), Err(LivenessEnd::ProbesExhausted));
}

#[test]
fn silent_admitted_client_loses_control_after_the_missed_limit() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        // The client keeps its transport open but never answers.
        let end = serve_admitted(&mut server, positioned()?, &authority, FAST).await;
        assert!(matches!(end, ConnectionEnd::AdmittedThenControlLost(_)));
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut expected = baseline();
        for probe_id in 1..=3 {
            expected.push(encode_liveness_probe(ADMITTED_GENERATION, probe_id)?);
        }
        assert_eq!(split_frames(&output)?, expected);
        Ok(())
    })
}

fn refused() -> Result<AdmittedSession, Box<dyn Error>> {
    Ok(AdmittedSession {
        first_entry: FirstEntryOutcome::RefusedUnavailable,
        ..positioned()?
    })
}

#[test]
fn a_silent_refused_session_with_an_actor_closes_after_the_bound() -> Result<(), Box<dyn Error>>
{
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        // The client keeps its transport open but never writes.
        let end = serve_admitted(&mut server, refused()?, &authority, FAST).await;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        assert!(output.is_empty());
        assert!(authority.steps.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn a_frame_before_the_bound_closes_a_refused_session_with_an_error()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        client.write_all(&framed(&ack(1, 1))).await?;
        let end = serve_admitted(&mut server, refused()?, &authority, FAST).await;
        assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, _)));
        Ok(())
    })
}

#[test]
fn a_refused_session_without_an_actor_is_not_bounded() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let admitted = AdmittedSession {
            runtime_actor: None,
            ..refused()?
        };
        let (mut server, _client) = tokio::io::duplex(1 << 16);
        let held = tokio::time::timeout(
            FAST.interval.saturating_mul(FAST.missed_limit * 5),
            serve_admitted(&mut server, admitted, &authority, FAST),
        )
        .await;
        assert!(held.is_err());
        Ok(())
    })
}

#[test]
fn answering_client_keeps_control_and_steps_between_probes() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let (mut server, client) = tokio::io::duplex(1 << 16);
        let (mut client_read, mut client_write) = tokio::io::split(client);
        let served = serve_admitted(&mut server, positioned()?, &authority, FAST);
        let client = async {
            let mut frames = super::super::tcp_tls::FrameReader::default();
            let mut probes = 0;
            while probes < 6 {
                let frame = frames.next(&mut client_read).await?;
                let envelope = decode_wire_envelope(&frame)?;
                if envelope.message_type() == MessageType::LivenessProbe {
                    probes += 1;
                    // Answer each probe as it arrives; a step between probes is served.
                    client_write.write_all(&framed(&ack(1, probes))).await?;
                    if probes == 3 {
                        client_write
                            .write_all(&framed(&command(
                                1,
                                1,
                                u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                                StepDirection::East,
                            )))
                            .await?;
                    }
                }
            }
            client_write.shutdown().await?;
            Ok::<_, Box<dyn Error>>(())
        };
        let (mut served, mut client) = (std::pin::pin!(served), std::pin::pin!(client));
        let (mut end, mut client_done) = (None, None);
        std::future::poll_fn(|context| {
            if end.is_none()
                && let std::task::Poll::Ready(value) = served.as_mut().poll(context)
            {
                end = Some(value);
            }
            if client_done.is_none()
                && let std::task::Poll::Ready(value) = client.as_mut().poll(context)
            {
                client_done = Some(value);
            }
            if end.is_some() && client_done.is_some() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
        let (Some(end), Some(client)) = (end, client_done) else {
            return Err("join incomplete".into());
        };
        client?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        assert_eq!(*authority.steps.borrow(), [StepDirection::East]);
        Ok(())
    })
}

#[test]
fn flooding_stale_acks_cannot_starve_the_cadence() -> Result<(), Box<dyn Error>> {
    use std::io::{Read, Write};
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        // A separate thread writes faster than the server reads, as a real flooding
        // client does: after probe 1 it repeats acks of probe 1 and never answers a later
        // probe.
        let flooder = std::thread::spawn(move || -> std::io::Result<()> {
            let mut socket = std::net::TcpStream::connect(address)?;
            let mut seen = Vec::new();
            let probe = framed(&encode_liveness_probe(ADMITTED_GENERATION, 1).map_err(
                |error| std::io::Error::new(std::io::ErrorKind::InvalidData, error),
            )?);
            let mut chunk = [0_u8; 4096];
            while !seen
                .windows(probe.len())
                .any(|window| window == probe.as_slice())
            {
                let read = socket.read(&mut chunk)?;
                if read == 0 {
                    return Ok(());
                }
                seen.extend_from_slice(&chunk[..read]);
            }
            let stale: Vec<u8> = (0..512).flat_map(|_| framed(&ack(1, 1))).collect();
            while socket.write_all(&stale).is_ok() {}
            Ok(())
        });
        let (stream, _) = listener.accept()?;
        stream.set_nonblocking(true)?;
        let mut stream = tokio::net::TcpStream::from_std(stream)?;
        let end = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            serve_admitted(&mut stream, positioned()?, &authority, FAST),
        )
        .await
        .map_err(|_| "the flood starved the liveness cadence")?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenControlLost(_)));
        drop(stream);
        let _ = flooder.join();
        Ok(())
    })
}

#[test]
fn ack_of_an_unsent_probe_closes_the_admitted_session() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = StepAuthority {
            steps: RefCell::new(Vec::new()),
            tutorial_notices: Default::default(),
        };
        let (end, frames) = drive_admitted(&authority, &[ack(1, 9)]).await?;
        let mut expected = baseline();
        expected.push(error_frame(
            FoundationProtocolError::InvalidWireIdentifier,
            1,
        ));
        assert_eq!(frames, expected);
        assert!(matches!(
            end,
            ConnectionEnd::AdmittedThenClosed(
                _,
                FoundationProtocolError::InvalidWireIdentifier
            )
        ));
        Ok(())
    })
}

#[test]
fn idle_liveness_matches_the_registered_provisional_rows() -> Result<(), Box<dyn Error>> {
    let registry: serde_json::Value =
        serde_json::from_str(oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON)?;
    let row = |id: &str| {
        registry["entries"]
            .as_array()
            .and_then(|entries| entries.iter().find(|entry| entry["id"] == id))
            .and_then(|entry| entry["hard_maximum"].as_u64())
    };
    assert_eq!(
        row("FND04B-LIVENESS-IDLE-PROBE-MS"),
        u64::try_from(IDLE_LIVENESS.interval.as_millis()).ok()
    );
    assert_eq!(
        row("FND04B-LIVENESS-IDLE-MISSED"),
        Some(u64::from(IDLE_LIVENESS.missed_limit))
    );
    assert_eq!(row("FND04B-LIVENESS-COMBAT-PROBE-MS"), Some(1_000));
    assert_eq!(row("FND04B-LIVENESS-COMBAT-MISSED"), Some(2));
    assert_eq!(
        row("FND04B-SAME-SESSION-GRACE-S"),
        u64::try_from(super::super::SAME_SESSION_GRACE_SECONDS).ok()
    );
    Ok(())
}
