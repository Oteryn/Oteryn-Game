use super::*;

pub(super) async fn resume_session(
    clients: &SeamClients<'_>,
    exact: &TlsConnector,
    session: [u8; 16],
) -> TestResult<()> {
    let &SeamClients {
        address,
        accounts,
        characters,
        world,
        descriptor,
        url,
        recovery,
        ..
    } = clients;
    evidence("stage=resume");
    // FND-04B §20: within grace the admission-stage player resumes the SAME GameSession with a
    // Platform recovery credential on a new TLS connection: generation 2 continues CommandId
    // and server_sequence, the actor is the same one at its committed position, and no
    // admission is created. That connection then ends on a gap again.
    let generation = platform_generation(descriptor, &accounts[0]).await?;
    let token = sign_recovery(
        recovery,
        &accounts[0],
        &characters[0],
        &world,
        generation,
        now_seconds()?,
        [0x5e; 32],
    );
    // ADMIT-CAP-1 (ADMIT0 §4, D1084): an authenticated recovery whose support omits the
    // required set is refused before PREPARE with exactly one RECOVERY_CAPABILITY_REQUIRED
    // (1118) frame. No claim is released and the credential is not consumed: the same
    // credential resumes the same GameSession below.
    let reply = exchange_must_close(
        address,
        exact,
        &framed(&envelope(
            3,
            0,
            &resume_payload_supporting(&session, token.as_bytes(), &[]),
        )),
    )
    .await?;
    if reply
        != Reply::Frames(vec![encode_protocol_error(
            FoundationProtocolError::RecoveryCapabilityRequired,
            0,
        )?])
        || committed_admissions(url).await? != 2
    {
        return Err(format!("recovery_capability_required: {reply:?}").into());
    }
    evidence("resume unsupported_recovery=refused_recovery_capability_required admissions=2");
    let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
    let mut raw = framed(&resume(&session, token.as_bytes()));
    raw.extend_from_slice(&framed(&client_command_at(
        2,
        7,
        step,
        &encode_step_intent(StepDirection::East),
    )));
    raw.extend_from_slice(&framed(&client_command_at(
        2,
        9,
        step,
        &encode_step_intent(StepDirection::East),
    )));
    let reply = exchange_must_close(address, exact, &raw).await?;
    let expected = resume_frames(
        WorldId::decode(&world)?,
        &session,
        ResumeFrames {
            generation: 2,
            sequence: 8,
            next: 7,
            revision: 3,
            x: 0,
            to: 1,
        },
    )?;
    if reply != Reply::Frames(expected) {
        return Err(format!("same-session resume diverged: {reply:?}").into());
    }
    if committed_admissions(url).await? != 2 {
        return Err("resume created an admission".into());
    }
    evidence(
        "resume same_session=resumed generation=2 next_command_id=7 server_sequence=8 position=0_0_0_rev3 step_east=moved_1_0_0_rev4 admissions=2",
    );
    // The consumed recovery credential cannot resume again.
    let replay = exchange(address, exact, &framed(&resume(&session, token.as_bytes()))).await?;
    if replay != Reply::Closed || committed_admissions(url).await? != 2 {
        return Err(format!("replayed recovery credential resumed: {replay:?}").into());
    }
    evidence("resume replayed_credential=refused");

    Ok(())
}

pub(super) async fn resumed_loss_and_expiry(
    clients: &SeamClients<'_>,
    exact: &TlsConnector,
    session: [u8; 16],
    concurrent_session: [u8; 16],
) -> TestResult<()> {
    let &SeamClients {
        address,
        accounts,
        characters,
        world,
        descriptor,
        url,
        recovery,
        ..
    } = clients;
    let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
    evidence("stage=resumed_loss");
    // FND-04B §§5–6, §20: the resumed connection ended on a gap, so its loss opens the next
    // epoch with its own 60 s grace, retaining the resumed epoch. Within that grace the
    // player resumes the SAME GameSession again: generation 3 continues CommandId and
    // server_sequence from generation 2, and the actor steps back west.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if let Some((1, epoch, grace)) = session_loss_row(url, session).await? {
            if epoch != "2" || !(58..=62).contains(&grace) {
                return Err(format!("resumed loss epoch={epoch} grace={grace}").into());
            }
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("resumed session never became reconnectable".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let generation = platform_generation(descriptor, &accounts[0]).await?;
    let token = sign_recovery(
        recovery,
        &accounts[0],
        &characters[0],
        &world,
        generation,
        now_seconds()?,
        [0x5f; 32],
    );
    let mut raw = framed(&resume(&session, token.as_bytes()));
    raw.extend_from_slice(&framed(&client_command_at(
        3,
        8,
        step,
        &encode_step_intent(StepDirection::West),
    )));
    raw.extend_from_slice(&framed(&client_command_at(
        3,
        10,
        step,
        &encode_step_intent(StepDirection::West),
    )));
    let reply = exchange_must_close(address, exact, &raw).await?;
    let expected = resume_frames(
        WorldId::decode(&world)?,
        &session,
        ResumeFrames {
            generation: 3,
            sequence: 10,
            next: 8,
            revision: 4,
            x: 1,
            to: 0,
        },
    )?;
    if reply != Reply::Frames(expected) {
        return Err(format!("second same-session resume diverged: {reply:?}").into());
    }
    if committed_admissions(url).await? != 2 {
        return Err("second resume created an admission".into());
    }
    evidence(
        "resumed_loss epoch=2 grace_s=60 same_session=resumed generation=3 next_command_id=8 server_sequence=10 position=1_0_0_rev4 step_west=moved_0_0_0_rev5 admissions=2",
    );

    evidence("stage=grace_expiry");
    // FND-04B §6: with no resumed control, each lost session is terminally released once its
    // original grace deadline passes; the released claims let the character enter again.
    // The twice-resumed session was lost a third time (epoch 3) and expires on its own grace.
    for (label, id) in [("closed", session), ("silent", concurrent_session)] {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(100);
        while !matches!(session_loss_row(url, id).await?, Some((3, _, _))) {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("{label} session was not released after grace").into());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    evidence("grace_expiry closed=terminal silent=terminal admissions=2");

    Ok(())
}
