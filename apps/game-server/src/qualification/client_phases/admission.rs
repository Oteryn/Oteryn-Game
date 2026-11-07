use super::*;

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

    // Replay of the consumed grant on a fresh connection.
    let reply = exchange(
        address,
        exact,
        &framed(&bootstrap(1, 1, &characters[0], &admitted_token)),
    )
    .await?;
    // N8: the committed session already holds the character, so the fresh verifier refuses the
    // replay as an account/character conflict before the durable replay key is reached.
    let replayed = Reply::Frames(vec![encode_protocol_error(
        FoundationProtocolError::AdmissionAccountCharacterConflict,
        0,
    )?]);
    if reply != replayed || committed_admissions(url).await? != 1 {
        return Err(format!("replayed grant admitted: {reply:?}").into());
    }
    evidence("replayed_grant=refused admissions=1");

    Ok(session)
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
    let concurrent: Vec<[u8; 16]> = [first?, second?, third?]
        .iter()
        .filter_map(accepted_session)
        .collect();
    let accepted = concurrent.len();
    if accepted != 1 || committed_admissions(url).await? != 2 {
        return Err(format!("concurrent same-grant admission accepted {accepted}").into());
    }
    evidence("concurrent_same_grant attempts=3 accepted=1 admissions=2");

    evidence("stage=control_loss");
    // DISCONNECT-PROTECTION-V1: the admission-stage transport closed on a command gap and the
    // concurrent-stage transport went silent. Neither answers liveness, so each durable
    // GameSession becomes RECONNECTABLE with loss epoch 1 and a 60 s same-session grace.
    for (label, id) in [("closed", session), ("silent", concurrent[0])] {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        loop {
            if let Some((1, epoch, grace)) = session_loss_row(url, id).await? {
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
