use super::*;

fn consumed_grant_refused(reply: &Reply) -> TestResult<bool> {
    Ok(*reply
        == Reply::Frames(vec![encode_protocol_error(
            FoundationProtocolError::AdmissionGrantReplayed,
            0,
        )?]))
}

#[test]
fn fresh_replay_requires_the_consumed_grant_refusal() -> TestResult {
    let reply = |error| {
        Ok::<_, Box<dyn std::error::Error>>(Reply::Frames(vec![encode_protocol_error(error, 0)?]))
    };
    assert!(consumed_grant_refused(&reply(
        FoundationProtocolError::AdmissionGrantReplayed
    )?)?);
    assert!(!consumed_grant_refused(&reply(
        FoundationProtocolError::AdmissionAccountCharacterConflict
    )?)?);
    Ok(())
}

fn silent_close_proven(probes: &[u64], elapsed: Duration, interval: Duration, misses: u32) -> bool {
    probes.iter().copied().eq(1..=u64::from(misses))
        && elapsed >= interval.saturating_mul(misses.saturating_add(1))
}

#[test]
fn silent_transport_requires_unanswered_server_probes_and_elapsed_cadence() {
    let cadence = Duration::from_secs(5);
    assert!(!silent_close_proven(
        &[1, 2],
        Duration::from_secs(20),
        cadence,
        3
    ));
    assert!(!silent_close_proven(
        &[1, 2, 3],
        Duration::from_secs(10),
        cadence,
        3
    ));
    assert!(!silent_close_proven(
        &[1, 2, 3],
        Duration::from_secs(15),
        cadence,
        3
    ));
    assert!(silent_close_proven(
        &[1, 2, 3],
        Duration::from_secs(20),
        cadence,
        3
    ));
}

pub(super) async fn admission(
    clients: &SeamClients<'_>,
    exact: &TlsConnector,
    grant: &OwnedGrant<'_>,
) -> TestResult<[u8; 16]> {
    let &SeamClients {
        address,
        characters,
        world,
        url,
        runtime,
        ..
    } = clients;
    evidence("stage=admission");
    // Positive: the real owners admit on evidence fetched by this attempt
    // (the pre-seeded S2 observations are older than five seconds). The
    // positioned actor receives its baseline snapshot, then the first-control
    // steps (#822): east moves, west returns, north (Blocked cell) and south
    // (outside the room) are blocked, a spell cast is rejected while casting is
    // gated, an unknown command type is rejected and a command-id gap closes the
    // connection.
    let admitted_token = sign_grant(&grant.borrowed(), now_seconds()?);
    // SPEED-1: the client paces its steps. West follows east's result and the blocked steps
    // follow west's, each after a step duration; blocked steps do not pace.
    let expected = first_control_frames(WorldId::decode(&world)?)?;
    let snapshot = 1 + expected.len() - 9;
    let mut segments = [
        (0, framed(&bootstrap(1, 1, &characters[0], &admitted_token))),
        (snapshot + 2, Vec::new()),
        (snapshot + 4, Vec::new()),
    ];
    let exura = crate::gameplay_transport::actor_spell::encode_spell_cast_intent(
        &crate::gameplay_transport::actor_spell::SpellCastIntent {
            spell: std::num::NonZeroU32::new(3).ok_or("spell index")?,
            target: crate::gameplay_transport::actor_spell::SpellTarget::None,
            aim_at_target: false,
        },
    );
    for (id, command_type, direction) in [
        (1, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection::East),
        (2, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection::West),
        (
            3,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            StepDirection::North,
        ),
        (
            4,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            StepDirection::South,
        ),
        (6, 0x7fff, StepDirection::East),
        (8, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection::East),
    ] {
        let raw = &mut segments[usize::try_from(id.min(3))? - 1].1;
        if id == 6 {
            raw.extend_from_slice(&framed(&client_command(
                5,
                u64::from(crate::gameplay_transport::actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT),
                &exura,
            )));
        }
        raw.extend_from_slice(&framed(&client_command(
            id,
            u64::from(command_type),
            &encode_step_intent(direction),
        )));
    }
    let reply = exchange_must_close_paced(address, exact, &segments).await?;
    let session =
        accepted_session(&reply).ok_or_else(|| format!("admission refused: {reply:?}"))?;
    let Reply::Frames(frames) = &reply else {
        return Err("missing frames".into());
    };
    if frames.get(1..) != Some(expected.as_slice()) {
        return Err(format!("first-control steps diverged: {reply:?}").into());
    }
    // ADMIT-CAP-1: the same grant the capability negative presented admits, so that refusal
    // consumed no nonce, and the session selects exactly the required set it declared.
    let selected = frames
        .first()
        .and_then(|frame| decode_wire_envelope(frame).ok())
        .and_then(|envelope| {
            oteryn_protocol_oteryn::decode_server_accepted(envelope.payload()).ok()
        })
        .map(|accepted| accepted.selected_capabilities);
    if selected.as_deref() != Some(SEAM_REQUIRED_CAPABILITIES) {
        return Err(format!("admission selected {selected:?}, not the required set").into());
    }
    if session[6] >> 4 != 7 || committed_admissions(url).await? != 1 {
        return Err("admission did not commit exactly one GameSession".into());
    }
    evidence(
        "admission=committed server_accepted=1 snapshot=baseline_0_0_0_rev1 step_east=moved_1_0_0_rev2 step_west=moved_0_0_0_rev3 step_north=blocked step_south=blocked spell_cast=rejected_gated unknown_command=rejected command_gap=closed_sequence_gap admissions=1",
    );

    // The fresh GameSession is durably committed and its first TLS socket has
    // ended. A new TLS connection's valid ClientResume must fail closed:
    // no ServerResumeAccepted, no replacement, no additional admission.
    let before = if let Some(runtime) = runtime {
        let counts = runtime.lock().await.player_slot_counts();
        if counts != (1, 0) {
            return Err(format!("fresh admission did not retain one actor: {counts:?}").into());
        }
        Some(counts)
    } else {
        None
    };
    let reply = exchange_must_close(
        address,
        exact,
        &framed(&resume(&session, b"unissued-reconnect-proof")),
    )
    .await?;
    let after = if let Some(runtime) = runtime {
        Some(runtime.lock().await.player_slot_counts())
    } else {
        None
    };
    if reply != Reply::Closed || committed_admissions(url).await? != 1 || after != before {
        return Err(
            format!("valid ClientResume acquired authority: {reply:?} actors={after:?}").into(),
        );
    }
    evidence(
        "resume valid_after_committed_socket_close=refused server_resume_accepted=0 admissions=1",
    );
    if after.is_some() {
        evidence("resume committed_players=1 pending_reservations=0");
    }

    Ok(session)
}

pub(super) async fn replay_released_grant(
    clients: &SeamClients<'_>,
    exact: &TlsConnector,
    grant: &OwnedGrant<'_>,
) -> TestResult {
    // The original claim has been terminally released. Renew only the validity
    // window/current security evidence; the consumed nonce and attempt stay fixed.
    let mut grant = grant.borrowed();
    grant.security_generation =
        platform_generation(clients.descriptor, &clients.accounts[0]).await?;
    let token = sign_grant(&grant, now_seconds()?);
    let reply = exchange_must_close(
        clients.address,
        exact,
        &framed(&bootstrap(1, 1, &clients.characters[0], &token)),
    )
    .await?;
    if !consumed_grant_refused(&reply)? || committed_admissions(clients.url).await? != 2 {
        return Err(format!(
            "released character did not reject the consumed grant nonce: {reply:?}"
        )
        .into());
    }
    evidence("replayed_grant=refused reason=consumed_nonce admissions=2");
    Ok(())
}

pub(super) async fn concurrent_and_control_loss<'a>(
    clients: &SeamClients<'a>,
    exact: &TlsConnector,
    next_grant: &mut impl FnMut(&str, [u8; 16], u64) -> OwnedGrant<'a>,
    session: [u8; 16],
) -> TestResult<[u8; 16]> {
    let &SeamClients {
        address,
        accounts,
        characters,
        descriptor,
        url,
        ..
    } = clients;
    // Concurrent use of one valid grant: exactly one GameSession. Each attempt refreshes
    // the account's owner evidence, which can leave the others' rounds stale; they retry
    // from fresh evidence and the grant replay key admits only one.
    let generation = platform_generation(descriptor, &accounts[1]).await?;
    let second = next_grant(&accounts[1], characters[1], generation);
    let token = sign_grant(&second.borrowed(), now_seconds()?);
    let raw = framed(&bootstrap(1, 1, &characters[1], &token));
    let ((first, second), third) = join(
        join(
            exchange(address, exact, &raw),
            exchange(address, exact, &raw),
        ),
        exchange(address, exact, &raw),
    )
    .await;
    let replies = [first?, second?, third?];
    let mut concurrent = Vec::new();
    let mut refusals = Vec::new();
    for (reply, elapsed) in &replies {
        if let Some(id) = accepted_session(reply) {
            let Reply::Frames(frames) = reply else {
                return Err("silent transport has no frames".into());
            };
            let mut probes = Vec::new();
            for frame in frames {
                let envelope = decode_wire_envelope(frame)?;
                match envelope.message_type() {
                    MessageType::LivenessProbe => probes.push(
                        oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())?,
                    ),
                    MessageType::ProtocolError => {
                        return Err("silent transport ended with a protocol error".into());
                    }
                    _ => {}
                }
            }
            let liveness = crate::gameplay_transport::IDLE_LIVENESS;
            if !silent_close_proven(
                &probes,
                elapsed.elapsed,
                liveness.interval,
                liveness.missed_limit,
            ) || elapsed.after_first_probe.is_none_or(|observed| {
                observed + Duration::from_millis(250)
                    < liveness.interval.saturating_mul(liveness.missed_limit)
            }) {
                return Err(format!("server close did not follow the unanswered-probe cadence: {probes:?} elapsed={elapsed:?}").into());
            }
            concurrent.push(id);
        } else {
            let mut reason = None;
            for error in [
                FoundationProtocolError::AdmissionGrantReplayed,
                FoundationProtocolError::AdmissionAccountCharacterConflict,
                FoundationProtocolError::AdmissionIncumbentProtected,
                FoundationProtocolError::AdmissionGrantRuntimeGenerationStale,
                FoundationProtocolError::AdmissionGrantSecurityEvidenceStale,
            ] {
                if *reply == Reply::Frames(vec![encode_protocol_error(error, 0)?]) {
                    reason = Some(error);
                    break;
                }
            }
            refusals.push(
                reason.ok_or_else(|| {
                    format!("unclassified concurrent admission refusal: {reply:?}")
                })?,
            );
        }
    }
    if concurrent.len() != 1 || refusals.len() != 2 || committed_admissions(url).await? != 2 {
        return Err(format!("concurrent same-grant admission diverged: {replies:?}").into());
    }
    evidence(&format!(
        "concurrent_same_grant attempts=3 accepted=1 admissions=2 refusals={refusals:?}"
    ));

    evidence("stage=control_loss");
    // DISCONNECT-PROTECTION-V1: the admission-stage transport closed on a command gap and the
    // concurrent-stage transport went silent. Neither answers liveness, so each durable
    // GameSession becomes RECONNECTABLE with loss epoch 1 and a 60 s same-session grace.
    for (label, id) in [("closed", session), ("silent", concurrent[0])] {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            if let Some((1, epoch, grace, _)) = session_loss_row(url, id, deadline).await? {
                if epoch != "1" || !(58..=62).contains(&grace) {
                    return Err(format!("{label} loss epoch={epoch} grace={grace}").into());
                }
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("{label} session never became reconnectable").into());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    if committed_admissions(url).await? != 2 {
        return Err("control loss changed the admission count".into());
    }
    evidence(
        "control_loss closed_transport=reconnectable silent_transport=reconnectable epoch=1 grace_s=60 admissions=2",
    );

    Ok(concurrent[0])
}
