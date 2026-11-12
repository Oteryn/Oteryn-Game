//! P11: achievements query/notification and quest/event granters.
//!
//! Concrete player scenario derived from the P11 matrix/re-audit row and the routed owner:
//! 1. Admit the provisioned character and send account-achievement page 0; expect an accepted,
//!    internally consistent page and an achievement-notice watermark snapshot.
//! 2. Refuse malformed query bytes at the local codec boundary without submission, then
//!    re-query unchanged state. The typed core API cannot submit a malformed wire query.
//! 3. Perform an ordinary player item-use action, the first reachable event/granter-shaped action
//!    available to the node-boot starter (its genuinely granted backpack); expect the command's
//!    real reply and, if it grants, an AchievementEarned delta plus a changed query result.
//! 4. Repeat the same player action; expect the refusal/idempotent path to add neither another
//!    fact nor another AchievementEarned delta.
//! 5. Log out, obtain a fresh admission, and query again; expect the exact earned facts, point
//!    total, and fact count from step 3 to survive. A server that does not connect this legitimate
//!    player action to a source granter is reported FAIL from the observed reply/missing delta;
//!    the remaining quest/event granters and malformed wire refusal remain unproven, so the
//!    full row stays FAIL even if this bounded lifecycle succeeds.
use super::*;
use oteryn_dev_client::client_runtime::account_achievements::{
    AccountAchievementsResult, Response,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::account_achievements as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
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

    let backpack = client
        .inventory()
        .and_then(|inventory| inventory.main_backpack)
        .ok_or("step=granter no legitimately granted starter item")?;
    client.take_events();
    let use_reply = client.session_mut().use_item(backpack.handle).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    let earned = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::AchievementEarned(delta) => Some(delta.value),
            _ => None,
        });
    let Some(earned) = earned else {
        // Still complete the durable lifecycle so this is runtime evidence, not a source probe.
        let unchanged = page(&mut client, 0).await?;
        logout(&mut client).await?;
        let mut fresh = connect_world(clients, 0xc2).await?;
        let persisted = page(&mut fresh, 0).await?;
        logout(&mut fresh).await?;
        return Err(format!(
            "step=granter observed_reply={use_reply:?},missing_delta=AchievementEarned,query_unchanged={},relogin_unchanged={};need=reachable_quest_or_event_source_granter",
            unchanged == before,
            persisted == before
        )
        .into());
    };
    let granted = page(&mut client, 0).await?;
    if granted.fact_count != before.fact_count.saturating_add(1)
        || !granted.rows.iter().any(|row| row.key == earned.key)
    {
        return Err(format!("step=granter delta/query mismatch: {earned:?} {granted:?}").into());
    }

    client.take_events();
    let duplicate_reply = client.session_mut().use_item(backpack.handle).await?;
    client.service_liveness(Duration::from_millis(250)).await?;
    if client
        .take_events()
        .iter()
        .any(|event| matches!(event, SessionEvent::AchievementEarned(_)))
        || page(&mut client, 0).await? != granted
    {
        return Err(
            format!("step=duplicate observed_reply={duplicate_reply:?},duplicate-grant").into(),
        );
    }
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xc2).await?;
    if page(&mut fresh, 0).await? != granted {
        return Err("step=relogin durable achievement page changed".into());
    }
    logout(&mut fresh).await?;
    Err("step=coverage observed=bounded-grant-relogin-validated;unproven=malformed-wire-query-refusal,remaining-quest-event-granters".into())
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
