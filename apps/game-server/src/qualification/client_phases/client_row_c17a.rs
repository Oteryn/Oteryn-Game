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
        Ok(Ok(())) => evidence("SEAM_EVIDENCE e2e_all scenario=C17a.client status=PASS"),
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
    character_prep::grant_premium(&client).await?;
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
        || vitals.mana < 160
        || vitals.health != vitals.max_health
    {
        return Err(format!("C17a seeded build mismatch: {level} {vocation:?} {vitals:?}").into());
    }
    evidence(&format!(
        "C17A_SEEDED_BUILD level={level} vocation={vocation:?} vitals={vitals:?}"
    ));
    let (haste, haste_payload) = cast_and_query(&mut client, "utani hur").await?;
    wait_support_cooldown(&mut client).await?;
    let (strong, strong_payload) = cast_and_query(&mut client, "utani gran hur").await?;

    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
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
    logout(&mut client).await?;
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
) -> TestResult<(ConditionStatus, Vec<u8>)> {
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!("C17a support cast refused: {words} {cast:?}").into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_CONDITION_STATUS_V1,
            COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C17a condition query refused after {words}").into());
    }
    let status = ConditionStatus::decode(&reply.payload).ok_or("C17a malformed condition reply")?;
    if !status
        .conditions
        .iter()
        .any(|condition| condition.icon == 13)
    {
        return Err(format!("C17a Haste icon absent after {words}").into());
    }
    Ok((status, reply.payload))
}

async fn wait_support_cooldown(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
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
