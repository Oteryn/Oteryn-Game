//! P13 full-row journey: loyalty earning and its skill/magic projection.
//!
//! Concrete player scenario, derived from the P13 audit/matrix row and the
//! read owner in `gameplay_transport/loyalty_skills.rs`:
//! 1. Admit an ordinary node-boot character and send LOYALTY_SKILLS_QUERY;
//!    expect an accepted, decodable snapshot whose points, bracket, title,
//!    base skills and loyalty-adjusted skills agree.
//! 2. Send a malformed LOYALTY_SKILLS_QUERY and expect a rejected reply with
//!    no payload; then send another valid query to prove refusal is inert.
//! 3. Seed consumed Premium time on the disposable Platform and watch the
//!    real loyalty stream change the admitted client's read. One point per
//!    consumed day is defined by Platform's docs/contracts/OTERYN_V2_LOYALTY_SNAPSHOT_CONTRACT.md:49.
//! 4. Log out normally, make a fresh admission, query again, and require the
//!    post-action points and every projected level to remain identical.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::loyalty_skills::{
    CAPABILITY_LOYALTY_SKILLS_V1, COMMAND_TYPE_LOYALTY_SKILLS_QUERY, LoyaltySkills,
};

const OWNER_GAP: &str = "step=loyalty-ingestion need=platform-consumed-premium-days source:crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:57";

const CONSUMED_PREMIUM_DAYS: u32 = 5;

fn loyalty_status(before: u32, after: u32) -> &'static str {
    match after.cmp(&before) {
        std::cmp::Ordering::Greater => "PASS",
        std::cmp::Ordering::Equal => "MISSING-SERVER",
        std::cmp::Ordering::Less => "FAIL",
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let (status, gap) =
        match verdict::indexed_timeout(Duration::from_secs(210), journey(clients)).await {
            Ok(Ok((before, after))) if before == after => (
                loyalty_status(before, after),
                Some(format!("{OWNER_GAP} observed=points-{before}-to-{after}")),
            ),
            Ok(Ok((before, after))) => (loyalty_status(before, after), None),
            Ok(Err(error)) => ("FAIL", Some(error.to_string())),
            Err(_) => (
                "FAIL",
                Some("step=journey observed=timeout-210s need=responsive-server".into()),
            ),
        };
    if let Some(gap) = &gap {
        evidence(&format!("P13_GAP {gap}"));
    }
    let detail = verdict_detail(status, gap);
    evidence(&format!(
        "e2e_all scenario=P13.full status={status} seconds={:.3}{detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn query(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
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
        return Err(
            format!("step={step} observed=status-{status:?} need=accepted-loyalty-reply").into(),
        );
    }
    let value = LoyaltySkills::decode(&payload).ok_or_else(|| {
        format!(
            "step={step} observed=undecodable-{}-byte-reply need=valid-loyalty-frame",
            payload.len()
        )
    })?;
    let expected = oteryn_gameplay_progression::loyalty::bonus_percent(value.points);
    if value.bonus_percent != expected
        || value.title.as_deref() != oteryn_gameplay_progression::loyalty::title(value.points)
        || value
            .skills
            .iter()
            .any(|skill| skill.with_loyalty < skill.base)
        || (value.bonus_percent == 0
            && value
                .skills
                .iter()
                .any(|skill| skill.with_loyalty != skill.base))
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<(u32, u32)> {
    let mut client = connect_world(clients, 0xd3).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_LOYALTY_SKILLS_V1)
    {
        return Err(
            "step=baseline observed=capability-not-offered need=loyalty-skills-capability".into(),
        );
    }
    let initial_session = client.join_snapshot().game_session_id;
    let before = snapshot(&mut client, "baseline-query").await?;

    let (status, payload) = query(&mut client, &[0]).await?;
    if status != CommandStatus::Rejected || !payload.is_empty() {
        return Err(format!(
            "step=malformed-refusal observed=status-{status:?}-payload-{} need=empty-rejection",
            payload.len()
        )
        .into());
    }
    if snapshot(&mut client, "post-refusal-query").await? != before {
        return Err("step=malformed-refusal observed=state-changed need=inert-refusal".into());
    }
    evidence("P13_CHECK malformed-refusal PASS rejected-empty-and-state-inert");

    let after = if std::env::var("NODE_BOOT_GAME_COMMERCE").as_deref() == Ok("1") {
        let work = required("NODE_BOOT_PLATFORM_GRANT_DIR")?;
        let account = clients.accounts[1].clone();
        let status = tokio::task::spawn_blocking(move || {
            std::process::Command::new("python3")
                .arg(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../tools/qualification/game_platform/composition.py"),
                )
                .args([
                    "loyalty",
                    &work,
                    &account,
                    &CONSUMED_PREMIUM_DAYS.to_string(),
                ])
                .output()
        })
        .await??
        .status;
        if !status.success() {
            return Err(format!(
                "step=seed-consumed-premium-days observed=platform-seed-exit-{status}"
            )
            .into());
        }
        let expected = before
            .points
            .checked_add(CONSUMED_PREMIUM_DAYS)
            .ok_or("loyalty points overflow")?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        verdict::Harness::global().bounded_wait(deadline, "loyalty-stream");
        loop {
            let current = snapshot(&mut client, "post-consumption-query").await?;
            if current.points == expected {
                break current;
            }
            if current.points != before.points || tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "step=loyalty-stream observed=points-{} expected={expected}",
                    current.points
                )
                .into());
            }
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
        }
    } else {
        let current = snapshot(&mut client, "stream-disabled-query").await?;
        if current != before {
            return Err(
                "step=loyalty-stream observed=unexpected-projection-change-with-commerce-disabled"
                    .into(),
            );
        }
        current
    };

    accepted_logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xd4).await?;
    if fresh.join_snapshot().game_session_id == initial_session {
        return Err("step=fresh-admission observed=reused-game-session need=new-admission".into());
    }
    let durable = snapshot(&mut fresh, "fresh-admission-query").await?;
    let cleanup = accepted_logout(&mut fresh).await;
    if durable != after {
        return Err(format!(
            "step=durability observed=before-{after:?}-after-{durable:?} need=identical-projection"
        )
        .into());
    }
    cleanup?;
    evidence("P13_CHECK fresh-admission PASS projection-identical-after-logout");
    if after.points < before.points {
        return Err(format!(
            "step=loyalty-earning observed=regressed-points-{}-to-{}",
            before.points, after.points
        )
        .into());
    }
    Ok((before.points, after.points))
}

fn verdict_detail(status: &str, gap: Option<String>) -> String {
    if status == "MISSING-SERVER" {
        return " evidence=source:crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:57".into();
    }
    gap.map_or_else(String::new, |gap| format!(" evidence={gap}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_server_evidence_is_only_a_source_coordinate() {
        assert_eq!(
            super::verdict_detail("MISSING-SERVER", Some("points unchanged".into())),
            " evidence=source:crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:57"
        );
    }

    #[test]
    fn unchanged_loyalty_is_a_missing_server_verdict() {
        assert_eq!(loyalty_status(0, 0), "MISSING-SERVER");
        assert_eq!(loyalty_status(2, 3), "PASS");
        assert_eq!(loyalty_status(3, 2), "FAIL");
    }

    use super::*;

    #[test]
    fn missing_server_verdict_identifies_the_platform_ingestion_gap() {
        assert_eq!(
            OWNER_GAP,
            "step=loyalty-ingestion need=platform-consumed-premium-days source:crates/gameplay/dispatch/src/gameplay_transport/loyalty_skills.rs:57"
        );
    }

    #[test]
    fn missing_server_evidence_is_a_report_compatible_source() {
        assert!(OWNER_GAP.contains(" source:"));
        assert!(
            OWNER_GAP
                .rsplit_once(':')
                .is_some_and(|(_, line)| line.parse::<u32>().is_ok())
        );
    }
}
