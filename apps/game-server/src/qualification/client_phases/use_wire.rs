use super::*;

pub(super) async fn use_wire<'a>(
    clients: &SeamClients<'a>,
    exact: &TlsConnector,
    next_grant: &mut impl FnMut(&str, [u8; 16], u64) -> OwnedGrant<'a>,
) -> TestResult<()> {
    let &SeamClients {
        address,
        accounts,
        characters,
        world,
        descriptor,
        url,
        ..
    } = clients;
    evidence("stage=use_wire");
    // USE-WIRE-V1 (#162 5868482467, M2b): the released character[1] (now TERMINAL, above) is
    // admitted fresh (a 3rd durable admission — the receipt count never decreases, see
    // `committed_admissions`), then drives the full door interaction in one batch (see
    // `use_wire_frames` for the exact expected disposition of each command). The connection ends
    // the same way the "admission" stage's own `session` does (a command replay closes it): the
    // resulting durable GameSession becomes RECONNECTABLE and is then released after its own
    // grace, exactly like `session`/`concurrent[0]` above. This whole stage — admission through
    // full release — runs *before* character[0]'s own final re-admission below, and not
    // concurrently with it: character[0]'s plain-disconnect connection (below) survives only
    // because `seam_clients` returns, and `shutdown` cancels the listener, before its own missed-
    // liveness control-loss timer fires; letting this stage's ~160s release wait run afterward
    // (delaying that return) would give that timer time to fire too and terminally release
    // character[0] as well, breaking `seam_flow`'s final invariant (exactly the P1 this ordering
    // fixes: r4122215795's companion CI finding).
    let use_session = {
        use oteryn_protocol_oteryn::world_object::{
            COMMAND_TYPE_USE_INTENT, WorldObjectTarget, encode_use_intent,
        };
        let door_key = crate::content::accepted::DOOR_CELL.0;
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let step_type = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        let use_frame = |id: u64, placement: &str, expected_revision: u64| -> TestResult<Vec<u8>> {
            let payload = encode_use_intent(&WorldObjectTarget {
                placement: placement.as_bytes().to_vec(),
                expected_revision,
            })
            .map_err(|_| "encode use intent")?;
            Ok(client_command(id, use_type, &payload))
        };
        let step_frame = |id: u64, direction: StepDirection| {
            client_command(id, step_type, &encode_step_intent(direction))
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let generation = platform_generation(descriptor, &accounts[1]).await?;
            let again = next_grant(&accounts[1], characters[1], generation);
            let token = sign_grant(&again.borrowed(), now_seconds()?);
            // SPEED-1: the client paces its steps: each step after the first follows the
            // previous segment's results by a step duration.
            let snapshot = 1 + use_wire_frames(WorldId::decode(&world)?)?.len() - 15;
            let segment = |frames: Vec<Vec<u8>>| -> Vec<u8> {
                frames.iter().flat_map(|frame| framed(frame)).collect()
            };
            let mut first = framed(&bootstrap(1, 1, &characters[1], &token));
            first.extend(segment(vec![
                step_frame(1, StepDirection::East),
                use_frame(2, door_key, 0)?,
            ]));
            let segments = [
                (0, first),
                (
                    snapshot + 4,
                    segment(vec![
                        step_frame(3, StepDirection::North),
                        use_frame(4, door_key, 1)?,
                    ]),
                ),
                (
                    snapshot + 7,
                    segment(vec![
                        step_frame(5, StepDirection::South),
                        use_frame(6, door_key, 1)?,
                    ]),
                ),
                (
                    snapshot + 11,
                    segment(vec![
                        step_frame(7, StepDirection::North),
                        use_frame(8, door_key, 0)?,
                        use_frame(9, "oteryn:cell/unknown", 2)?,
                        // Replays the already-consumed CommandId 6.
                        use_frame(6, door_key, 1)?,
                    ]),
                ),
            ];
            let reply = exchange_must_close_paced(address, exact, &segments).await?;
            if let Some(session) = accepted_session(&reply) {
                let Reply::Frames(frames) = &reply else {
                    return Err("missing frames".into());
                };
                let expected = use_wire_frames(WorldId::decode(&world)?)?;
                if frames.get(1..) != Some(expected.as_slice()) {
                    return Err(format!("use-wire scenario diverged: {reply:?}").into());
                }
                break session;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("use-wire character was not admitted again: {reply:?}").into());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    };
    if committed_admissions(url).await? != 3 {
        return Err("use-wire admission did not commit exactly one GameSession".into());
    }
    evidence(
        "use_wire door_open=committed step_through=moved use_in_doorway=occupied step_out=moved door_close=committed step_blocked=blocked stale_revision=stale_state unknown_placement=nothing_to_use replayed_command_id=expired admissions=3",
    );

    // Release the use-wire actor the same way the "closed" transport above is released:
    // control loss (RECONNECTABLE, epoch 1) after the missed-liveness window, then terminal
    // release after its own grace. No resume is attempted for it. This completes in full, here,
    // before character[0]'s own final re-admission below is even attempted.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if let Some((1, epoch, grace)) = session_loss_row(url, use_session).await? {
            if epoch != "1" || !(58..=62).contains(&grace) {
                return Err(format!("use-wire loss epoch={epoch} grace={grace}").into());
            }
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("use-wire session never became reconnectable".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(100);
    while !matches!(session_loss_row(url, use_session).await?, Some((3, _, _))) {
        if tokio::time::Instant::now() >= deadline {
            return Err("use-wire session was not released after grace".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    evidence("use_wire released=terminal admissions=3");

    Ok(())
}
