//! P13 full-row journey: loyalty earning and its skill/magic projection.
//!
//! Concrete player scenario, derived from the P13 audit/matrix row and the
//! read owner in `gameplay_transport/loyalty_skills.rs`:
//! 1. Admit an ordinary node-boot character and send LOYALTY_SKILLS_QUERY;
//!    expect an accepted, decodable snapshot whose points, bracket, title,
//!    base skills and loyalty-adjusted skills agree.
//! 2. Send a malformed LOYALTY_SKILLS_QUERY and expect a rejected reply with
//!    no payload; then send another valid query to prove refusal is inert.
//! 3. Give the disposable Platform account a completed Premium interval and
//!    let the production Game stream puller import the real Platform export.
//! 4. Query again and require the new points, title, bonus and unchanged base skills.
//! 5. Log out normally, make a fresh admission, query again, and require the
//!    post-action points and every projected level to remain identical.
use super::*;
use crate::durability::platform_streams::PlatformStream;
use crate::premium::client::PremiumClientConfig;
use oteryn_dev_client::DevClientSession;
use oteryn_gameplay_social::platform_streams::{LOYALTY_IDENTITY_VAR, PlatformStreams, StreamPuller};
use oteryn_protocol_oteryn::loyalty_skills::{
    CAPABILITY_LOYALTY_SKILLS_V1, COMMAND_TYPE_LOYALTY_SKILLS_QUERY, LoyaltySkills,
};
use sha2::{Digest, Sha256};

// One spent Premium day per point. At 360 points Global grants 5% and Steward of Tibia.
const SPENT_PREMIUM_DAYS: i32 = 360;

fn verify_award(before: &LoyaltySkills, after: &LoyaltySkills) -> TestResult {
    if (after.points, after.bonus_percent, after.title.as_deref())
        != (u32::try_from(SPENT_PREMIUM_DAYS)?, 5, Some("Steward of Tibia"))
        || before.skills.map(|skill| skill.base) != after.skills.map(|skill| skill.base)
    {
        return Err(format!(
            "step=loyalty-award observed=before-{before:?}-after-{after:?} need=spent-premium-points-title-and-bonus-with-unchanged-base-skills"
        )
        .into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let (status, gap) =
        match verdict::indexed_timeout(Duration::from_secs(210), journey(clients)).await {
            Ok(Ok(())) => ("PASS", None),
            Ok(Err(error)) => ("FAIL", Some(error.to_string())),
            Err(_) => (
                "FAIL",
                Some("step=journey observed=timeout-210s need=responsive-server".into()),
            ),
        };
    if let Some(gap) = &gap {
        evidence(&format!("P13_GAP {gap}"));
    }
    let detail = gap.map_or_else(String::new, |gap| format!(" evidence={gap}"));
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

/// The enclosing node-boot harness owns both disposable databases and the
/// purpose-specific mTLS identity. Seed only Platform's earning input.
async fn platform_update(clients: &SeamClients<'_>) -> TestResult<PlatformStreams> {
    // connect_world uses the second packet Account, seeded as accountD.
    let account = &clients.accounts[1];
    let output = std::path::PathBuf::from(required("PORTAL_E2E_OUT")?);
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(
        output.with_file_name("portal-report.json"),
    )?)?;
    let database = report["database"]
        .as_str()
        .and_then(|name| name.strip_prefix("portal_e2e_"))
        .filter(|suffix| suffix.len() == 12 && suffix.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("step=platform-fixture need=disposable-portal-database")?;
    let mut admin = reqwest::Url::parse(&required("PORTAL_E2E_ADMIN_DATABASE_URL")?)
        .map_err(|_| "step=platform-fixture need=valid-admin-route")?;
    let origin = reqwest::Url::parse(
        report["baseUrls"]["internal"]
            .as_str()
            .ok_or("step=platform-fixture need=internal-origin")?,
    )?;
    if report["status"] != "ready"
        || report["outputDirectory"].as_str() != output.to_str()
        || report["seed"]["accountD"]["accountId"].as_str() != Some(account.as_str())
        || admin.scheme() != "postgresql"
        || admin.host_str() != Some("127.0.0.1")
        || admin.path() != "/postgres"
        || origin.scheme() != "https"
        || origin.host_str() != Some("127.0.0.1")
    {
        return Err("step=platform-fixture need=ready-local-portal-harness".into());
    }
    admin.set_path(&format!("/portal_e2e_{database}"));

    let certs = output.join("certs");
    let mut identity = std::fs::read(certs.join("clients/loyalty-stream.pem"))?;
    identity.extend(std::fs::read(certs.join("clients/loyalty-stream.key"))?);
    let puller = StreamPuller::new(
        &PremiumClientConfig {
            origin: origin.to_string(),
            identity_pem: identity,
            platform_ca_pem: std::fs::read(certs.join("ca.pem"))?,
        },
        PlatformStream::Loyalty,
        LOYALTY_IDENTITY_VAR,
    )?;

    let (_, runtime_url) = group_login(clients.url, "oteryn_game_runtime").await?;
    let root = DurabilityRoot::connect_test_runtime(&runtime_url)?;
    if !root.maintain_ready_once().await? {
        return Err("step=loyalty-ingestion need=ready-durability-root".into());
    }

    let mut platform = sqlx::PgConnection::connect(admin.as_str())
        .await
        .map_err(|_| "step=platform-fixture need=reachable-disposable-portal-database")?;
    let mut tx = platform.begin().await?;
    sqlx::query("INSERT INTO premium_authority (account_id) VALUES ($1::uuid) ON CONFLICT DO NOTHING")
        .bind(account)
        .execute(&mut *tx)
        .await?;
    let interval = sqlx::query(
        "WITH authority AS (\
             UPDATE premium_authority SET authority_revision = 1 \
             WHERE account_id = $1::uuid AND authority_revision = 0 RETURNING account_id\
         ), spent AS (\
             SELECT date_trunc('second', now()) - interval '1 hour' AS ended\
         ) \
         INSERT INTO premium_ledger (request_id, account_id, actor_id, event_type, duration_days, \
             reason_sha256, entitlement_id, lifecycle_revision, delta_seconds, effective_from, effective_until) \
         SELECT $2::uuid, account_id, account_id, 'GRANT', $3::integer, $4, $5::uuid, 1, \
             ($3::integer)::bigint * 86400, ended - $3::integer * interval '24 hours', ended \
         FROM authority CROSS JOIN spent",
    )
    .bind(account)
    .bind(uuid_text(&crate::node::operator_files::uuid_v7()?))
    .bind(SPENT_PREMIUM_DAYS)
    .bind(Sha256::digest(b"P13 spent Premium Time qualification").as_slice())
    .bind(uuid_text(&crate::node::operator_files::uuid_v7()?))
    .execute(&mut *tx)
    .await?;
    if interval.rows_affected() != 1 {
        return Err("step=platform-fixture need=fresh-premium-authority".into());
    }
    tx.commit().await?;
    platform.close().await?;

    Ok(PlatformStreams::spawn(&root, vec![puller], evidence))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
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
    if before.points != 0 {
        return Err(format!("step=baseline observed={before:?} need=fresh-loyalty-account").into());
    }

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

    let _pullers = platform_update(clients).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let after = loop {
        let current = snapshot(&mut client, "platform-update-query").await?;
        if current.points != before.points {
            verify_award(&before, &current)?;
            evidence("P13_CHECK platform-update PASS spent-premium-points-title-and-bonus");
            break current;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=loyalty-ingestion observed=unchanged-points need=platform-stream-update".into());
        }
        client.service_liveness(Duration::from_millis(500)).await?;
        client.take_events();
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
    verify_award(&before, &durable)?;
    evidence("P13_CHECK fresh-admission PASS projection-identical-after-logout");
    Ok(())
}
