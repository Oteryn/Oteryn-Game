//! C05 — parameter spells (locate/find person and named targets).
//!
//! Concrete player scenario derived from the C05 audit/matrix row and its chat,
//! parameter-cast, Exiva-preference, and PostgreSQL owner tests:
//! 1. Prepare the dedicated caster at level 8, then admit it and the ordinary target.
//!    Obtain the target's served name.
//! 2. As the target, send the player Exiva-settings command to allow all locates;
//!    assert the accepted result frame and persisted settings-list delta/reply.
//! 3. As the caster, say `exiva "<target>"`; assert an accepted chat result, the
//!    private Game locate reply naming the target, and the exact 20-mana delta.
//! 4. As the target, send the player settings command denying all locates; assert
//!    its accepted result, then cast again and assert the protected refusal reply
//!    and no mana/state cost.
//! 5. Log the target out, freshly admit it with a different game session, list the
//!    durable setting, and re-cast to prove denial and no-cost refusal survived.
use super::full_row_c01::admission_error;
use super::*;
use oteryn_dev_client::{
    ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession, SessionEvent,
};
use oteryn_protocol_oteryn::exiva;

const ACTIONS: &[&str] = &["allow", "locate", "deny", "refusal", "fresh-admission"];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("C05", "level-8-exiva-build", "learn and cast Exiva");
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
        result.as_ref().err().map_or_else(
            || "observed-all-required-actions".to_owned(),
            |error| super::full_row_c04::evidence_token(&error.to_string()),
        ),
    ));
}

async fn setting(client: &mut DevClientSession, payload: &[u8]) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(
            exiva::CAPABILITY_EXIVA_SETTINGS_V1,
            exiva::COMMAND_TYPE_EXIVA_SETTINGS,
            payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "settings step refused: status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into());
    }
    Ok(reply.payload)
}

pub(super) fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client
        .world_entities()
        .ok_or("name step missing entity snapshot")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .and_then(|entry| entry.display_name)
        .ok_or("name step missing own display name")?
        .as_str()
        .map(str::to_owned)
        .map_err(|error| format!("name step invalid UTF-8: {error:?}").into())
}

pub(super) async fn cast(
    client: &mut DevClientSession,
    target: &str,
) -> TestResult<(oteryn_dev_client::ChatOutcome, Vec<String>)> {
    client.take_events();
    let reply = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: format!("exiva \"{target}\""),
        })
        .await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut messages = Vec::new();
    loop {
        for event in client.take_events() {
            if let SessionEvent::ChatLine(delta) = event
                && let ChatLine::Private { speaker_name, text } = delta.value
                && speaker_name == "Game"
            {
                messages.push(text);
            }
        }
        if !messages.is_empty() || tokio::time::Instant::now() >= deadline {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok((reply, messages))
}

pub(super) async fn connect_caster(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    let mut initial = connect_character(clients, 0xc5, 0).await?;
    let old_session = initial.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(old_session.as_bytes()),
        "--profile",
        "c05-exiva".into(),
    )
    .await?;
    let logout = initial.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C05 caster preparation logout refused: {:?}", logout.result).into());
    }
    drop(initial);
    let mut caster = connect_character(clients, 0xc5, 0)
        .await
        .map_err(|error| admission_error("caster-preparation", error))?;
    if caster.join_snapshot().game_session_id == old_session {
        return Err("C05 caster preparation reused its original session".into());
    }
    if full_e2ef7::player_level(&mut caster).await? != 8 {
        return Err("Exiva caster did not admit at level 8".into());
    }
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        8,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("caster preparation has no source vitals")?;
    if !caster.actor_vitals().is_some_and(|vitals| {
        vitals.max_health == max_health && vitals.max_mana == max_mana && vitals.mana >= 20
    }) {
        return Err(format!(
            "caster preparation did not load level-8 sorcerer vitals: {:?}",
            caster.actor_vitals()
        )
        .into());
    }
    Ok(caster)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_EXIVA_ACCOUNT_ID").await?;
    let caster_clients = identity.clients(clients);
    let target_identity =
        character_prep::dedicated_character(clients, "NODE_BOOT_C05_TARGET_ACCOUNT").await?;
    let target_clients = target_identity.clients(clients);
    let mut caster = connect_caster(&caster_clients).await?;
    let mut target = connect_character(&target_clients, 0xc6, 0)
        .await
        .map_err(|error| admission_error("target-admission", error))?;
    let target_name = own_name(&target)?;
    let initial = exiva::PlayerList::decode(&setting(&mut target, &[5]).await?)
        .ok_or("initial settings list invalid")?;
    if initial.modes != 30 || !initial.players.is_empty() {
        return Err(
            format!("initial Exiva settings differ from expected defaults: {initial:?}").into(),
        );
    }
    let caster_position = caster.world_spatial().actor_position;
    let target_position = target.world_spatial().actor_position;
    if caster_position.floor != target_position.floor
        || caster_position
            .x
            .abs_diff(target_position.x)
            .max(caster_position.y.abs_diff(target_position.y))
            >= 5
    {
        return Err(format!(
            "C05 expected adjacent spawn positions, got {caster_position:?} and {target_position:?}"
        )
        .into());
    }
    if setting(&mut target, &[0, 1]).await? != [1] {
        return Err("allow step missing committed [1] reply".into());
    }
    let list = exiva::PlayerList::decode(&setting(&mut target, &[5]).await?)
        .ok_or("allow step invalid settings list")?;
    if list.modes != 1 || !list.players.is_empty() {
        return Err(format!("allow step observed modes={}", list.modes).into());
    }
    evidence(&format!(
        "C05_CHECK {} PASS accepted-settings-reply",
        ACTIONS[0]
    ));

    let catalog = caster
        .session_mut()
        .gated_snapshot(
            oteryn_protocol_oteryn::actor_spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG,
        )
        .ok_or("step=locate-eligibility observed=no-served-spell-catalog")?;
    let entries = oteryn_protocol_oteryn::actor_spell_catalog::decode_catalog(&catalog.payload)
        .map_err(|error| format!("step=locate-eligibility observed=invalid-catalog:{error:?}"))?;
    if let Some(gap) = locate_eligibility(&entries) {
        return Err(gap.into());
    }
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("locate step missing vitals")?;
    let (reply, messages) = cast(&mut caster, &target_name).await?;
    let after = caster
        .actor_vitals()
        .copied()
        .ok_or("locate step missing post-cast vitals")?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != ChatDisposition::Ok
        || before.mana.checked_sub(20) != Some(after.mana)
        || messages != [format!("{target_name} is standing next to you.")]
    {
        return Err(format!("locate step command={} observed status={:?} disposition={:?} mana={}->{} replies={messages:?}", reply.command_id, reply.status, reply.disposition, before.mana, after.mana).into());
    }
    evidence(&format!(
        "C05_CHECK {} PASS locate-reply-and-mana-delta",
        ACTIONS[1]
    ));

    if setting(&mut target, &[0, 0]).await? != [1] {
        return Err("deny step missing committed [1] reply".into());
    }
    let denied = exiva::PlayerList::decode(&setting(&mut target, &[5]).await?)
        .ok_or("deny step invalid settings list")?;
    if denied.modes != 0 || !denied.players.is_empty() {
        return Err(format!("deny step observed {denied:?}").into());
    }
    evidence(&format!(
        "C05_CHECK {} PASS observed-deny-settings",
        ACTIONS[2]
    ));
    // Exiva starts its spell and support cooldowns on success. A refusal
    // during that interval would not exercise the target's privacy gate.
    let ready_at = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < ready_at {
        caster.service_liveness(Duration::from_millis(25)).await?;
        target.service_liveness(Duration::from_millis(25)).await?;
    }
    let before_refusal = caster
        .actor_vitals()
        .copied()
        .ok_or("refusal step missing vitals")?;
    let (refused, messages) = cast(&mut caster, &target_name).await?;
    let after_refusal = caster
        .actor_vitals()
        .copied()
        .ok_or("refusal step missing post-cast vitals")?;
    let protected =
        "The character you are trying to find with Exiva is currently protected from your spell.";
    if refused.status != CommandStatus::Rejected
        || refused.disposition != ChatDisposition::Rejected
        || before_refusal != after_refusal
        || messages != [protected]
    {
        return Err(format!("refusal step observed status={:?} disposition={:?} replies={messages:?} vitals_changed={}", refused.status, refused.disposition, before_refusal != after_refusal).into());
    }
    evidence(&format!("C05_CHECK {} PASS protected-no-cost", ACTIONS[3]));

    let old_session = target.join_snapshot().game_session_id;
    if target.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("fresh-admission step logout refused".into());
    }
    drop(target);
    let mut target = connect_character(&target_clients, 0xc7, 0)
        .await
        .map_err(|error| admission_error("target-readmission", error))?;
    if target.join_snapshot().game_session_id == old_session {
        return Err("fresh-admission step reused game session".into());
    }
    let list = exiva::PlayerList::decode(&setting(&mut target, &[5]).await?)
        .ok_or("fresh-admission step invalid settings list")?;
    if list != denied {
        return Err(format!("fresh-admission step observed modes={}", list.modes).into());
    }
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("fresh refusal missing vitals")?;
    let (refused, messages) = cast(&mut caster, &target_name).await?;
    if refused.status != CommandStatus::Rejected
        || refused.disposition != ChatDisposition::Rejected
        || caster.actor_vitals() != Some(&before)
        || messages != [protected]
    {
        return Err(format!(
            "fresh-admission refusal observed disposition={:?} replies={messages:?}",
            refused.disposition
        )
        .into());
    }
    evidence(&format!("C05_CHECK {} PASS durable-denial", ACTIONS[4]));
    for client in [&mut caster, &mut target] {
        let logout = client.session_mut().logout().await?;
        if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("C05 final logout refused: {:?}", logout.result).into());
        }
    }
    Ok(())
}

fn locate_eligibility(
    entries: &[oteryn_protocol_oteryn::actor_spell_catalog::SpellCatalogEntry],
) -> Option<String> {
    match entries.iter().find(|entry| entry.words.eq_ignore_ascii_case("exiva")) {
        Some(entry) if entry.available => None,
        Some(entry) => Some(format!("step=locate-eligibility observed=exiva-unavailable required-level={} learned={} need=prepared-exiva-eligibility", entry.level, entry.learned)),
        None => Some("step=locate-eligibility observed=exiva-absent-from-vocation-catalogue need=served-exiva-for-admitted-vocation".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_exiva_is_distinct_from_a_cast_refusal() {
        assert_eq!(
            locate_eligibility(&[]).as_deref(),
            Some(
                "step=locate-eligibility observed=exiva-absent-from-vocation-catalogue need=served-exiva-for-admitted-vocation"
            )
        );
    }
}
