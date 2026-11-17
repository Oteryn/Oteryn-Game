//! C05 — parameter spells (locate/find person and named targets).
//!
//! Concrete player scenario derived from the C05 audit/matrix row and its chat,
//! parameter-cast, Exiva-preference, and PostgreSQL owner tests:
//! 1. Admit two ordinary node-boot characters and obtain the target's served name.
//! 2. As the target, send the player Exiva-settings command to allow all locates;
//!    assert the accepted result frame and persisted settings-list delta/reply.
//! 3. As the caster, say `exiva "<target>"`; assert an accepted chat result, the
//!    private Game locate reply naming the target, and the exact 20-mana delta.
//! 4. As the target, send the player settings command denying all locates; assert
//!    its accepted result, then cast again and assert the protected refusal reply
//!    and no mana/state cost.
//! 5. Log the target out, freshly admit it with a different game session, list the
//!    durable setting, and re-cast to prove denial and no-cost refusal survived.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::exiva;

const ACTIONS: &[&str] = &["allow", "locate", "deny", "refusal", "fresh-admission"];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step deadline: observed timeout while awaiting C05 server reply".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("C05_GAP {error}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C05.full status={} seconds={:.3} evidence={}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        if result.is_ok() { "observed-all-required-actions" } else { "observed-runtime-failure-see-C05_GAP" },
    ));
}

async fn setting(client: &mut DevClientSession, payload: &[u8]) -> TestResult<Vec<u8>> {
    let reply = client.session_mut().gated_command(
        exiva::CAPABILITY_EXIVA_SETTINGS_V1,
        exiva::COMMAND_TYPE_EXIVA_SETTINGS,
        payload,
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("settings step refused: status={:?} payload={:?}", reply.status, reply.payload).into());
    }
    Ok(reply.payload)
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client.world_entities().ok_or("name step missing entity snapshot")?;
    entities.iter().find(|entry| entry.entity.identity == *entities.own_identity())
        .and_then(|entry| entry.display_name)
        .ok_or("name step missing own display name")?
        .as_str().map(str::to_owned).map_err(|error| format!("name step invalid UTF-8: {error:?}").into())
}

async fn cast(client: &mut DevClientSession, target: &str) -> TestResult<(oteryn_dev_client::ChatOutcome, Vec<String>)> {
    client.take_events();
    let reply = client.chat(&ChatIntent::Say {
        mode: ChatSpeechMode::Say,
        text: format!("exiva \"{target}\""),
    }).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut messages = Vec::new();
    loop {
        for event in client.take_events() {
            if let SessionEvent::ChatLine(delta) = event
                && let ChatLine::Private { speaker_name, text } = delta.value
                && speaker_name == "Game"
            { messages.push(text); }
        }
        if !messages.is_empty() || tokio::time::Instant::now() >= deadline { break; }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok((reply, messages))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut caster = connect_character(clients, 0xc5, 0).await?;
    let mut target = connect_character(clients, 0xc6, 1).await?;
    let target_name = own_name(&target)?;
    if setting(&mut target, &[0, 1]).await? != [1] { return Err("allow step missing committed [1] reply".into()); }
    let list = exiva::PlayerList::decode(&setting(&mut target, &[5]).await?).ok_or("allow step invalid settings list")?;
    if list.modes != 1 { return Err(format!("allow step observed modes={}", list.modes).into()); }
    evidence(&format!("C05_CHECK {} PASS accepted-settings-reply", ACTIONS[0]));

    let before = caster.actor_vitals().copied().ok_or("locate step missing vitals")?;
    let (reply, messages) = cast(&mut caster, &target_name).await?;
    let after = caster.actor_vitals().copied().ok_or("locate step missing post-cast vitals")?;
    if reply.status != CommandStatus::Accepted || reply.disposition != ChatDisposition::Ok
        || before.mana.checked_sub(20) != Some(after.mana)
        || !messages.iter().any(|line| line.starts_with(&format!("{target_name} ")))
    { return Err(format!("locate step observed status={:?} disposition={:?} mana={}->{} replies={messages:?}", reply.status, reply.disposition, before.mana, after.mana).into()); }
    evidence(&format!("C05_CHECK {} PASS locate-reply-and-mana-delta", ACTIONS[1]));

    if setting(&mut target, &[0, 0]).await? != [1] { return Err("deny step missing committed [1] reply".into()); }
    evidence(&format!("C05_CHECK {} PASS accepted-settings-reply", ACTIONS[2]));
    let before_refusal = caster.actor_vitals().copied().ok_or("refusal step missing vitals")?;
    let (refused, messages) = cast(&mut caster, &target_name).await?;
    let after_refusal = caster.actor_vitals().copied().ok_or("refusal step missing post-cast vitals")?;
    let protected = "The character you are trying to find with Exiva is currently protected from your spell.";
    if refused.disposition != ChatDisposition::Rejected || before_refusal != after_refusal || !messages.iter().any(|line| line == protected) {
        return Err(format!("refusal step observed status={:?} disposition={:?} replies={messages:?} vitals_changed={}", refused.status, refused.disposition, before_refusal != after_refusal).into());
    }
    evidence(&format!("C05_CHECK {} PASS protected-no-cost", ACTIONS[3]));

    let old_session = target.join_snapshot().game_session_id;
    if target.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) { return Err("fresh-admission step logout refused".into()); }
    drop(target);
    let mut target = connect_character(clients, 0xc7, 1).await?;
    if target.join_snapshot().game_session_id == old_session { return Err("fresh-admission step reused game session".into()); }
    let list = exiva::PlayerList::decode(&setting(&mut target, &[5]).await?).ok_or("fresh-admission step invalid settings list")?;
    if list.modes != 0 { return Err(format!("fresh-admission step observed modes={}", list.modes).into()); }
    let before = caster.actor_vitals().copied().ok_or("fresh refusal missing vitals")?;
    let (refused, messages) = cast(&mut caster, &target_name).await?;
    if refused.disposition != ChatDisposition::Rejected || caster.actor_vitals() != Some(&before) || !messages.iter().any(|line| line == protected) {
        return Err(format!("fresh-admission refusal observed disposition={:?} replies={messages:?}", refused.disposition).into());
    }
    evidence(&format!("C05_CHECK {} PASS durable-denial", ACTIONS[4]));
    let _ = caster.session_mut().logout().await;
    let _ = target.session_mut().logout().await;
    Ok(())
}
