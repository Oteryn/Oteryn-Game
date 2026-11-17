//! P13 full-row journey: loyalty earning and its skill/magic projection.
//!
//! Concrete player scenario, derived from the P13 audit/matrix row and the
//! read owner in `gameplay_transport/loyalty_skills.rs`:
//! 1. Admit an ordinary node-boot character and send LOYALTY_SKILLS_QUERY;
//!    expect an accepted, decodable snapshot whose points, bracket, title,
//!    base skills and loyalty-adjusted skills agree.
//! 2. Send a malformed LOYALTY_SKILLS_QUERY and expect a rejected reply with
//!    no payload; then send another valid query to prove refusal is inert.
//! 3. Credit a real native rat hunt and independently check the Platform-owned account total.
//! 4. Require an account accrual in that projection and the same value from the Game query.
//! 5. Log out and verify the updated projection after a fresh admission.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::loyalty_skills::{
    CAPABILITY_LOYALTY_SKILLS_V1, COMMAND_TYPE_LOYALTY_SKILLS_QUERY, LoyaltySkills,
};

const OWNER_GAP: &str =
    "source:crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:57";

enum Verdict {
    Pass,
    Fail(String),
    MissingAccrual,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let verdict = match verdict::indexed_timeout(Duration::from_secs(210), verdict::observe(journey(clients))).await {
        Ok(Ok(true)) => Verdict::Pass,
        Ok(Ok(false)) => Verdict::MissingAccrual,
        Ok(Err(error)) => Verdict::Fail(error.to_string()),
        Err(_) => Verdict::Fail("step=journey observed=timeout-210s need=responsive-server".into()),
    };
    let (status, evidence_suffix) = match verdict {
        Verdict::Pass => ("PASS", String::new()),
        Verdict::MissingAccrual => ("FAIL", format!(" evidence=needed-platform-loyalty-accrual;{OWNER_GAP}")),
        Verdict::Fail(error) => {
            let safe = error.split_whitespace().collect::<Vec<_>>().join("-");
            evidence(&format!("P13_GAP {error}"));
            ("FAIL", format!(" evidence={safe}"))
        }
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=P13.full status={status} seconds={:.3}{evidence_suffix}",
        started.elapsed().as_secs_f64()
    ));
}

async fn query(client: &mut DevClientSession, payload: &[u8]) -> TestResult<(CommandStatus, Vec<u8>)> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_LOYALTY_SKILLS_V1,
            COMMAND_TYPE_LOYALTY_SKILLS_QUERY,
            payload,
        )
        .await?;
    Ok((reply.status, reply.payload))
}

async fn snapshot(client: &mut DevClientSession, step: &str) -> TestResult<LoyaltySkills> {
    let (status, payload) = query(client, &[]).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step={step} observed=status-{status:?} need=accepted-loyalty-reply").into());
    }
    let value = LoyaltySkills::decode(&payload).ok_or_else(|| {
        format!("step={step} observed=undecodable-{}-byte-reply need=valid-loyalty-frame", payload.len())
    })?;
    let expected = oteryn_gameplay_progression::loyalty::bonus_percent(value.points);
    if value.bonus_percent != expected
        || value.title.as_deref() != oteryn_gameplay_progression::loyalty::title(value.points)
        || value.skills.iter().any(|skill| skill.with_loyalty < skill.base)
        || (value.bonus_percent == 0
            && value.skills.iter().any(|skill| skill.with_loyalty != skill.base))
    {
        return Err(format!("step={step} observed=inconsistent-projection-{value:?} need=points-bracket-and-levels-to-agree").into());
    }
    evidence(&format!(
        "P13_CHECK {step} PASS points={} bonus={} skills={:?}",
        value.points, value.bonus_percent, value.skills
    ));
    Ok(value)
}

async fn accepted_logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let observed = client.session_mut().logout().await?;
        if observed.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=logout observed={observed:?} need=accepted-logout").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
}

fn classify_loyalty(before: u32, after: u32, platform_before: u32, platform_after: u32) -> TestResult<bool> {
    if before != platform_before || after != platform_after || platform_after < platform_before {
        return Err(format!("step=loyalty-accrual observed=game:{before}->{after},account:{platform_before}->{platform_after}").into());
    }
    Ok(platform_after > platform_before)
}

async fn account_points(database: &mut sqlx::PgConnection, character: &[u8; 16]) -> TestResult<u32> {
    let points: i64 = sqlx::query_scalar(
        "SELECT coalesce(l.loyalty_points,0) FROM game_character_roots r
         LEFT JOIN game_account_loyalty_projections l USING(account_id)
         WHERE r.character_id=encode($1,'hex')::uuid",
    ).bind(character.as_slice()).fetch_one(database).await?;
    Ok(u32::try_from(points)?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<bool> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xd3).await?;
    if !client.selected_capabilities().contains(&CAPABILITY_LOYALTY_SKILLS_V1) {
        return Err("step=baseline observed=capability-not-offered need=loyalty-skills-capability".into());
    }
    let initial_session = client.join_snapshot().game_session_id;
    use sqlx::Connection;
    let character = &clients.characters[1];
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let account_before = account_points(&mut database, character).await?;
    let before = snapshot(&mut client, "baseline-query").await?;
    classify_loyalty(before.points, before.points, account_before, account_before)?;

    let (status, payload) = query(&mut client, &[0]).await?;
    if status != CommandStatus::Rejected || !payload.is_empty() {
        return Err(format!("step=malformed-refusal observed=status-{status:?}-payload-{} need=empty-rejection", payload.len()).into());
    }
    if snapshot(&mut client, "post-refusal-query").await? != before {
        return Err("step=malformed-refusal observed=state-changed need=inert-refusal".into());
    }
    evidence("P13_CHECK malformed-refusal PASS rejected-empty-and-state-inert");

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat")
        .await
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step=walk-to-training observed={error} need=reachable-rat"), error) })?;
    client_hunt::hunt_named(&mut client, "Rat")
        .await
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step=combat-training observed={error} need=attack-damage-corpse-and-loot-deltas"), error) })?;
    evidence("P13_CHECK combat-training PASS named-rat-attack-damage-corpse-use-loot-deltas");
    let until = tokio::time::Instant::now() + Duration::from_secs(10);
    let account_after = loop {
        let points = account_points(&mut database, character).await?;
        if points != account_before || tokio::time::Instant::now() >= until {
            break points;
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    };
    let after = snapshot(&mut client, "post-accrual-query").await?;
    let accrued = classify_loyalty(before.points, after.points, account_before, account_after)?;

    accepted_logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xd4).await?;
    if fresh.join_snapshot().game_session_id == initial_session {
        return Err("step=fresh-admission observed=reused-game-session need=new-admission".into());
    }
    let durable = snapshot(&mut fresh, "fresh-admission-query").await?;
    let cleanup = accepted_logout(&mut fresh).await;
    if durable != after {
        return Err(format!("step=durability observed=before-{after:?}-after-{durable:?} need=identical-projection").into());
    }
    cleanup?;
    evidence("P13_CHECK fresh-admission PASS projection-identical-after-logout");
    Ok(accrued)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loyalty_verdict_requires_a_real_account_accrual_and_detects_regression() -> TestResult {
        assert!(!classify_loyalty(10, 10, 10, 10)?);
        assert!(classify_loyalty(10, 10, 10, 12).is_err());
        assert!(classify_loyalty(10, 9, 10, 10).is_err());
        assert!(classify_loyalty(10, 11, 10, 10).is_err());
        assert!(classify_loyalty(10, 12, 10, 12)?);
        Ok(())
    }

    #[test]
    fn missing_server_evidence_is_a_report_compatible_source() {
        assert!(OWNER_GAP.starts_with("source:"));
        assert!(OWNER_GAP.rsplit_once(':').is_some_and(|(_, line)| line.parse::<u32>().is_ok()));
    }
}
