// Included in the World journey so route planning and admission keep their existing owners.
// These are bounded scenarios, not claims of complete coverage of M01 or S18.
async fn character_core(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode};
    use oteryn_protocol_oteryn::target_move;

    let journal_started = std::time::Instant::now();
    let journal = query_quest_journal(&mut client, true).await?;
    let journal_seconds = journal_started.elapsed().as_secs_f64();
    let started = std::time::Instant::now();
    let initial = client.world_spatial().actor_position;
    let old_session = client.join_snapshot().game_session_id;
    for direction in [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ] {
        // TARGET_MOVE has no exported encoder; its Turn branch embeds the STEP codec.
        let mut payload = Vec::new();
        bytes_field(&mut payload, 3, &encode_step_intent(direction));
        let result = client
            .session_mut()
            .gated_command(
                target_move::CAPABILITY_TARGET_MOVE_V1,
                target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
                &payload,
            )
            .await?;
        if result.status != CommandStatus::Accepted || result.payload != [8, 1] {
            return Err(format!("core turn {direction:?}: {result:?}").into());
        }
        if client.world_spatial().actor_position != initial {
            return Err("turn moved the player".into());
        }
    }
    evidence(&format!(
        "e2e_all scenario=M01.turn-result status=PASS seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));

    for (mode, text) in [
        (ChatSpeechMode::Say, "E2E ALL say"),
        (ChatSpeechMode::Whisper, "E2E ALL whisper"),
    ] {
        let started = std::time::Instant::now();
        let outcome = client
            .chat(&ChatIntent::Say {
                mode,
                text: text.into(),
            })
            .await?;
        if outcome.status != CommandStatus::Accepted || outcome.disposition != ChatDisposition::Ok {
            return Err(format!("core speech {mode:?}: {outcome:?}").into());
        }
        // The echo is published by the server's periodic chat refresh, not by the command
        // result. Leave the same load-bearing observation window as the neighbouring combat
        // journey while continuing to require the exact echo below.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            if client.chat_log().is_some_and(|log| {
                log.lines().any(|line| {
                    matches!(line,
                        ChatLine::Local { mode: seen_mode, text: seen_text, position, .. }
                        if *seen_mode == mode && seen_text == text && *position == initial
                    )
                })
            }) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("core speech {mode:?} missing from client chat model").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
        let scenario = match mode {
            ChatSpeechMode::Say => "S18.say-echo",
            ChatSpeechMode::Whisper => "S18.whisper-echo",
            ChatSpeechMode::Yell | ChatSpeechMode::MonsterSay => unreachable!(),
        };
        evidence(&format!(
            "e2e_all scenario={scenario} status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        ));
    }

    let started = std::time::Instant::now();
    // Move away from the configured spawn: a relogin that silently resets to spawn must fail.
    let direction = [
        StepDirection::North,
        StepDirection::East,
        StepDirection::South,
        StepDirection::West,
    ]
    .into_iter()
    .find(|direction| destination(world, TOWN, *direction).is_some_and(|at| at != TOWN))
    .ok_or("core has no legal step away from temple")?;
    walk(&mut client, world, &[direction]).await?;
    let saved = client.world_spatial().actor_position;
    if saved == initial {
        return Err("core step did not leave the configured spawn".into());
    }
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if busy_timed_out {
        return Err(format!(
            "core logout refused after Busy retries: {logout}; attempt count: {attempts}"
        )
        .into());
    }
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("core logout refused: {logout}").into());
    }
    drop(client);
    client = connect_world(clients, 0x76).await?;
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != saved
    {
        return Err("core fresh admission did not load the saved non-spawn position".into());
    }
    let journal_started = std::time::Instant::now();
    if query_quest_journal(&mut client, false).await? != journal {
        return Err(
            "quest journal list changed across fresh admission without a quest action".into(),
        );
    }
    evidence(&format!(
        "e2e_all scenario=W15a.list-relogin status=PASS seconds={:.3}",
        journal_seconds + journal_started.elapsed().as_secs_f64()
    ));
    evidence(&format!(
        "e2e_all scenario=M01.step-relogin status=PASS seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    let back = route(world, tile(saved)?, |at| at == TOWN).ok_or("core return route")?;
    walk(&mut client, world, &back).await?;
    Ok(client)
}
