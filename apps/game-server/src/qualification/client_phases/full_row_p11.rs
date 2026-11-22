//! P11: achievements query/notification and quest/event granters.
//!
//! Concrete player scenario derived from the P11 matrix/re-audit row and the routed owner:
//! 1. Admit the provisioned character and send account-achievement page 0; expect an accepted,
//!    internally consistent page and an achievement-notice watermark snapshot.
//! 2. Refuse malformed query bytes at the local codec boundary without submission, then
//!    re-query unchanged state. The typed core API cannot submit a malformed wire query.
//! 3. Check page boundaries and re-admit the same character to verify the query
//!    remains stable. A grant requires a reachable authored source and its notice.
use super::*;
use oteryn_dev_client::client_runtime::account_achievements::{
    AccountAchievementsResult, Response,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::account_achievements as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step deadline: achievement lifecycle timed out".into()),
    };
    let (status, evidence_text) = match result {
        Ok(()) => (
            "PASS",
            "query-notice-grant-refusal-relogin-validated".to_owned(),
        ),
        Err(error) => ("FAIL", sanitize(&error.to_string())),
    };
    evidence(&format!("P11_GAP status={status} observed={evidence_text}"));
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=P11.full status={status} seconds={:.3} evidence={evidence_text}",
        started.elapsed().as_secs_f64()
    ));
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:./=,".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

async fn page(client: &mut DevClientSession, number: u32) -> TestResult<AccountAchievementsResult> {
    match client.session_mut().account_achievements(number).await? {
        Response::Page(page)
            if page.page == number
                && page.fact_count >= u32::try_from(page.rows.len())?
                && page.total_points >= page.rows.iter().map(|row| row.points).sum() =>
        {
            Ok(page)
        }
        Response::Page(page) => Err(format!("step=query invalid server page: {page:?}").into()),
        Response::Refused => Err(format!("step=query page={number} observed=refused").into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc1).await?;
    let before = page(&mut client, 0).await?;
    let notice_snapshot = client
        .session_mut()
        .gated_snapshot(
            oteryn_protocol_oteryn::achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
        )
        .ok_or("step=query missing achievement-notice snapshot")?;
    oteryn_protocol_oteryn::achievement_notices::decode_achievement_notices_snapshot(
        &notice_snapshot.payload,
    )
    .map_err(|error| format!("step=query malformed notice snapshot: {error:?}"))?;

    client.take_events();
    let next_command = client.next_command_id();
    let sequence = client.last_server_sequence();
    if wire::decode_account_achievements_query(&[0x08]).is_ok()
        || client.next_command_id() != next_command
        || client.last_server_sequence() != sequence
    {
        return Err("step=local-codec-refusal observed=submitted-or-not-refused".into());
    }
    if client
        .take_events()
        .iter()
        .any(|event| matches!(event, SessionEvent::AchievementEarned(_)))
        || page(&mut client, 0).await? != before
    {
        return Err("step=local-codec-refusal observed=unexpected-achievement-delta".into());
    }
    evidence(
        "P11_CHECK step=malformed-query boundary=local-codec-refusal wire_submission=false command-and-sequence=unchanged reread=unchanged",
    );

    let beyond = before
        .fact_count
        .div_ceil(wire::ACCOUNT_ACHIEVEMENTS_PAGE_ROWS as u32)
        .max(1);
    let empty = page(&mut client, beyond).await?;
    if empty.has_more
        || !empty.rows.is_empty()
        || empty.fact_count != before.fact_count
        || empty.total_points != before.total_points
    {
        return Err(format!("step=pagination page={beyond} observed={empty:?}").into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc2).await?;
    if page(&mut fresh, 0).await? != before {
        return Err("step=relogin durable achievement page changed".into());
    }
    logout(&mut fresh).await?;
    Err("step=granter need=reachable-Yasir-committed-trade-or-other-authored-grant-and-AchievementEarned;source=crates/gameplay/durability/src/durability/npc_trade.rs:566;unproven=malformed-wire-query-refusal;source=crates/session/src/account_achievements.rs:18".into())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn evidence_sanitizer_produces_one_report_token() {
        assert_eq!(
            super::sanitize("step failed\n(reply)"),
            "step_failed__reply_"
        );
    }
}

#[cfg(test)]
#[test]
fn row_11_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p11.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P11_GAP status="))
    );
}
