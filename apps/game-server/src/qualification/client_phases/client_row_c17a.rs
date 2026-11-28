//! Client replay capture for SCENARIO_MATRIX C17a (Haste / Strong Haste).
//! The owning client seam is `ConditionStatusPanel::finish` in
//! `crates/client-ui/src/features/0276-condition-status/state.rs`; it decodes the real condition
//! query reply, while `panel_text`/`status_text` in that feature's `view.rs` exposes Haste.

use super::*;
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatSpeechMode};
use oteryn_protocol_oteryn::condition_status::{
    CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
};
use std::io::Write;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    match result {
        Ok(Ok(())) => evidence("SEAM_EVIDENCE e2e_all scenario=C17a.client status=CAPTURED"),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C17a.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("SEAM_EVIDENCE e2e_all scenario=C17a.client status=FAIL evidence=timeout")
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded(
        "C17a.client",
        "level-20-sorcerer,premium",
        "cast and capture both support spells within the row budget",
    );
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_Q_C17A_ACCOUNT").await?;
    let clients = &fixture.clients(clients);
    let mut client = connect_character(clients, 0xca, 0).await?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        "c17a-client".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    let mut client = connect_character(clients, 0xca, 0).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("C17a seeded build reused its original session".into());
    }
    let level = full_e2ef7::player_level(&mut client).await?;
    let vocation = full_e2ef5::current_vocation(&mut client).await?;
    let vitals = client.actor_vitals().ok_or("C17a caster vitals absent")?;
    if level != 20
        || vocation != crate::spell::Vocation::Sorcerer
        || vitals.mana != vitals.max_mana
        || vitals.mana != 450
        || vitals.health != vitals.max_health
        || vitals.health != 245
    {
        return Err(format!("C17a seeded build mismatch: {level} {vocation:?} {vitals:?}").into());
    }
    evidence(&format!(
        "C17A_SEEDED_BUILD level={level} vocation={vocation:?} vitals={vitals:?}"
    ));
    let book = native_spell_book()?;
    let haste_spell = book.spoken("utani hur").ok_or("C17a Haste absent")?.spell;
    if haste_spell.premium {
        return Err("C17a Global Haste must be available to free accounts".into());
    }
    let strong_spell = book
        .spoken("utani gran hur")
        .ok_or("C17a Strong Haste absent")?
        .spell;
    let world = full_e2ef7::qualification_world(clients)?;
    let start = tile(client.world_spatial().actor_position)?;
    let (outward, inward, neighbor) = [
        (StepDirection::North, StepDirection::South),
        (StepDirection::East, StepDirection::West),
        (StepDirection::South, StepDirection::North),
        (StepDirection::West, StepDirection::East),
    ]
    .into_iter()
    .find_map(|(outward, inward)| {
        let neighbor = destination(world, start, outward)?;
        (neighbor.floor == start.floor
            && world.ground_speed(neighbor) == world.ground_speed(start)
            && destination(world, neighbor, inward) == Some(start)
            && world
                .base()
                .tile(neighbor.x, neighbor.y, neighbor.floor)
                .is_some_and(|tile| tile.protection_zone()))
        .then_some((outward, inward, neighbor))
    })
    .ok_or("C17a no same-ground protected speed probe from the start")?;
    let ground = world.ground_speed(start);
    let base = oteryn_gameplay_characters::speed::player_base_speed(level)
        .ok_or("C17a invalid base speed")?;
    check_step(&mut client, outward, neighbor, base, ground).await?;
    let strong_index = client
        .session_mut()
        .spell_catalog()?
        .and_then(|catalog| {
            catalog
                .entries
                .into_iter()
                .find(|entry| entry.words == "utani gran hur")
        })
        .and_then(|entry| std::num::NonZeroU32::new(entry.cast_index))
        .ok_or("C17a Strong Haste index absent on free admission")?;
    assert_refusal(
        &mut client,
        strong_index,
        oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Rejected,
        false,
    )
    .await?;
    let _ = cast_and_query(&mut client, "utani hur", 390, 30_000).await?;
    check_step(&mut client, inward, start, base * 130 / 100, ground).await?;
    character_prep::grant_premium(&client).await?;
    require_current_premium(&mut client, "after_free_cast").await?;
    wait_support_cooldown(&mut client).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        if client
            .session_mut()
            .spell_catalog()?
            .is_some_and(|catalog| {
                ["utani hur", "utani gran hur"].iter().all(|words| {
                    catalog
                        .entries
                        .iter()
                        .any(|entry| entry.words == *words && entry.available)
                })
            })
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17a current Premium did not reach the live spell catalog".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    if !strong_spell.premium {
        return Err("C17a Global Strong Haste must require Premium".into());
    }
    full_e2ef5::prepare_spell_access(&mut client, strong_spell).await?;
    let (before, _) = query_conditions(&mut client).await?;
    if haste_remaining(&before)? > 28_000 {
        return Err("C17a free Haste cooldown did not elapse".into());
    }
    let (haste, haste_payload) = cast_and_query(&mut client, "utani hur", 330, 30_000).await?;
    let haste_sequence = client.last_server_sequence();
    assert_refusal(
        &mut client,
        strong_index,
        oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::CoolingDown,
        true,
    )
    .await?;
    let before_strong = wait_support_cooldown(&mut client).await?;
    if haste_remaining(&before_strong)? <= 22_000 {
        return Err("C17a pre-Strong Haste condition does not distinguish replacement".into());
    }
    let (strong, strong_payload) =
        cast_and_query(&mut client, "utani gran hur", 230, 22_000).await?;
    let strong_sequence = client.last_server_sequence();
    check_step(&mut client, outward, neighbor, base * 170 / 100, ground).await?;
    let cast_session = client.join_snapshot().game_session_id;
    let strong_remaining = haste_remaining(&strong)?;
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 0).await?;
    let mut client = connect_character(clients, 0xca, 0).await?;
    if client.join_snapshot().game_session_id == cast_session {
        return Err("C17a durability check reused the casting session".into());
    }
    let (retained, _) = query_conditions(&mut client).await?;
    if haste_remaining(&retained)? > strong_remaining
        || client.actor_vitals().map(|vitals| vitals.mana) != Some(230)
    {
        return Err("C17a fresh admission did not retain Strong Haste and its mana debit".into());
    }
    check_step(&mut client, inward, start, base * 170 / 100, ground).await?;
    logout(&mut client).await?;

    let path = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C17a capture directory missing")?;
    {
        let expected = format!("{}\n---\n{}\n", visible(&haste)?, visible(&strong)?);
        for (name, bytes) in [
            ("c17a-haste.bin", haste_payload),
            ("c17a-strong-haste.bin", strong_payload),
        ] {
            std::fs::File::create_new(Path::new(&path).join(name))?.write_all(&bytes)?;
        }
        std::fs::File::create_new(Path::new(&path).join("c17a-expected.txt"))?
            .write_all(expected.as_bytes())?;
    }
    evidence(&format!(
        "C17A_CAPTURE session={} haste_sequence={haste_sequence} strong_sequence={strong_sequence}",
        uuid_text(cast_session.as_bytes())
    ));
    Ok(())
}

async fn check_step(
    client: &mut oteryn_dev_client::DevClientSession,
    direction: StepDirection,
    destination: TilePos,
    speed: u32,
    ground: u16,
) -> TestResult {
    use oteryn_gameplay_characters::speed::StepSpeedTable;
    let expected = StepSpeedTable::embedded()
        .and_then(|table| table.step_duration(u16::try_from(speed).ok()?, ground))
        .ok_or("C17a expected pace unavailable")?;
    let frame = client.step(direction).await?;
    if frame.disposition != StepDisposition::Moved
        || tile(client.world_spatial().actor_position)? != destination
        || u128::from(frame.step_wait_ms) != expected.as_millis()
    {
        return Err(format!(
            "C17a movement mismatch expected_speed={speed} ground={ground} expected_ms={} actual_ms={} disposition={:?}",
            expected.as_millis(), frame.step_wait_ms, frame.disposition
        ).into());
    }
    evidence(&format!(
        "C17A_STEP speed={speed} ground={ground} frame={frame:?}"
    ));
    Ok(())
}

async fn assert_refusal(
    client: &mut oteryn_dev_client::DevClientSession,
    spell: std::num::NonZeroU32,
    expected: oteryn_protocol_oteryn::actor_spell::SpellCastDisposition,
    within_cooldown: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::actor_spell::SpellTarget;
    let (before, _) = query_conditions(client).await?;
    let vitals = client.actor_vitals().copied();
    let refusal = client.cast_spell(spell, SpellTarget::None, false).await?;
    let (after, _) = query_conditions(client).await?;
    let unchanged = if within_cooldown {
        let remaining = haste_remaining(&after)?;
        remaining > 28_000 && remaining <= haste_remaining(&before)?
    } else {
        before.conditions.is_empty() && after.conditions.is_empty()
    };
    if refusal.disposition != expected
        || refusal.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != vitals
        || !unchanged
    {
        return Err(
            format!("C17a expected {expected:?} without a state change: {refusal:?}").into(),
        );
    }
    evidence(&format!(
        "C17A_REFUSAL expected={expected:?} frame={refusal:?}"
    ));
    Ok(())
}

async fn require_current_premium(
    client: &mut oteryn_dev_client::DevClientSession,
    phase: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::{character_info, premium_overview};
    let reply = client
        .session_mut()
        .gated_command(
            character_info::CAPABILITY_CHARACTER_INFO_V1,
            character_info::COMMAND_TYPE_CHARACTER_INFO,
            &[premium_overview::OPERATION],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "C17a Premium query refused phase={phase}: {:?}",
            reply.status
        )
        .into());
    }
    let overview = premium_overview::Overview::decode(&reply.payload)
        .ok_or("C17a malformed Premium overview")?;
    evidence(&format!(
        "C17A_PREMIUM phase={phase} state={:?} valid_for_us={} authority_revision={}",
        overview.state, overview.valid_for_us, overview.authority_revision
    ));
    if overview.state != premium_overview::State::Active || overview.valid_for_us == 0 {
        return Err(format!("C17a Premium is not current phase={phase}").into());
    }
    Ok(())
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("C17a logout refused: {outcome:?}").into());
    }
    Ok(())
}

async fn cast_and_query(
    client: &mut oteryn_dev_client::DevClientSession,
    words: &str,
    expected_mana: u32,
    duration_ms: i64,
) -> TestResult<(ConditionStatus, Vec<u8>)> {
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        for event in client.take_events() {
            if matches!(
                event,
                oteryn_dev_client::SessionEvent::ServerMessage(_)
                    | oteryn_dev_client::SessionEvent::ActorCooldowns(_)
            ) {
                evidence(&format!(
                    "C17A_CAST_REFUSAL_EVENT words={words:?} {event:?}"
                ));
            }
        }
        return Err(format!(
            "C17a support cast refused: {words} {cast:?} vitals={:?}",
            client.actor_vitals()
        )
        .into());
    }
    let (status, payload) = query_conditions(client).await?;
    let remaining = haste_remaining(&status)?;
    if !(duration_ms - 2_000..=duration_ms).contains(&remaining)
        || client.actor_vitals().map(|vitals| vitals.mana) != Some(expected_mana)
    {
        return Err(format!(
            "C17a {words} outcome mismatch: remaining_ms={remaining} vitals={:?}",
            client.actor_vitals()
        )
        .into());
    }
    Ok((status, payload))
}

async fn query_conditions(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<(ConditionStatus, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err("C17a condition query refused".into());
    }
    let status = ConditionStatus::decode(&reply.payload).ok_or("C17a malformed condition reply")?;
    Ok((status, reply.payload))
}

fn haste_remaining(status: &ConditionStatus) -> TestResult<i64> {
    let mut haste = status
        .conditions
        .iter()
        .filter(|condition| condition.icon == 13);
    let condition = haste.next().ok_or("C17a Haste icon absent")?;
    if haste.next().is_some() || condition.sub_id != 0 || condition.remaining_ms <= 0 {
        return Err("C17a invalid Haste condition slot or duration".into());
    }
    Ok(condition.remaining_ms)
}

async fn wait_support_cooldown(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<ConditionStatus> {
    // The Haste condition and support cooldown start on the same server cast.
    // Use the server's countdown to prove two seconds elapsed. Local time only
    // bounds the observation, and never opens the cast gate.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let (status, _) = query_conditions(client).await?;
        if haste_remaining(&status)? <= 28_000 {
            return Ok(status);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17a support cooldown did not elapse on the server".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
}

fn visible(status: &ConditionStatus) -> TestResult<String> {
    let condition = status
        .conditions
        .iter()
        .find(|condition| condition.icon == 13)
        .ok_or("C17a Haste icon absent")?;
    let duration = if condition.remaining_ms == -1 {
        "Permanent".into()
    } else {
        format!(
            "{}s",
            condition.remaining_ms / 1_000 + i64::from(condition.remaining_ms % 1_000 != 0)
        )
    };
    Ok(format!("» Haste: {duration}\r\n"))
}
