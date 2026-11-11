//! W08 full-row journey: Animus Mastery completion and species XP multiplier.
//!
//! Concrete player scenario derived from the W08 audit/matrix row and the named
//! `kill_reward`/Animus owners:
//! 1. Admit the provisioned character and send command 232 for page zero; require
//!    an accepted, bounded, sorted mastery page and retain its durable species set.
//! 2. Send an invalid zero-limit command 232 and require the server to reject it
//!    without a page. This is the refusal path and must not change the mastery set.
//! 3. Re-query with the valid player command and require the same set. If no
//!    completion exists, report the observed empty reply at this exact acquisition
//!    step: a player cannot manufacture a completion through the query protocol.
//! 4. Log out, obtain a fresh admission, send command 232 again, and require the
//!    completed species set to survive with the same total and keys.
//! 5. The completed-species combat leg must kill that species and assert the XP
//!    progress delta equals the server's species-scoped Animus multiplier; absence
//!    of a player-earned completion is reported as the blocking observed state, not
//!    converted into a PASS.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::animus_mastery as wire;
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "initial-query",
    "invalid-query-refused",
    "completion-observed",
    "fresh-admission-persistence",
    "species-xp-multiplier",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    failure: Option<String>,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
            failure: None,
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!("W08_CHECK {action} PASS observed={observed:?}"));
        } else {
            self.failure = Some(format!("duplicate-or-unknown-action:{action}"));
        }
    }

    fn fail(&mut self, step: &'static str, observed: impl std::fmt::Display) {
        let message = format!("step={step};observed={observed}");
        evidence(&format!("W08_CHECK {step} FAIL {message}"));
        self.failure.get_or_insert(message);
    }

    fn finish(self) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("W08_GAP {action} assertion-not-completed"));
            }
        }
        let passed = self.failure.is_none() && ACTIONS.iter().all(|a| self.passed.contains(a));
        let suffix = self
            .failure
            .as_deref()
            .map(|failure| format!(" evidence={}", evidence_token(failure)))
            .unwrap_or_default();
        evidence(&format!(
            "e2e_all scenario=W08.full status={} seconds={:.3}{suffix}",
            if passed { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:=,".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = row_complete(journey(&mut row, clients)).await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => row.fail("journey", error),
        Err(_) => row.fail("journey", "timeout-after-90s"),
    }
    row.finish();
}

async fn command(
    client: &mut DevClientSession,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_ANIMUS_MASTERY_V1,
            wire::COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
            payload,
        )
        .await?)
}

async fn page(client: &mut DevClientSession) -> TestResult<wire::AnimusMasteries> {
    let payload = wire::AnimusPage {
        offset: 0,
        limit: wire::MAX_ANIMUS_PAGE_KEYS,
    }
    .encode()
    .ok_or("W08 valid page did not encode")?;
    let reply = command(client, &payload).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("valid command-232 reply={reply:?}").into());
    }
    wire::decode_page(&reply.payload)
        .ok_or_else(|| format!("accepted command-232 malformed-page={:?}", reply.payload).into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0xc8).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let before = page(&mut client).await?;
    row.pass("initial-query", (&before.total, &before.keys));

    let refused = command(&mut client, &[0, 0, 0, 0]).await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        row.fail("invalid-query-refused", format!("reply={refused:?}"));
        return Ok(());
    }
    row.pass("invalid-query-refused", refused.status);

    let after_refusal = page(&mut client).await?;
    if after_refusal != before {
        row.fail(
            "completion-observed",
            format!("mastery-set-mutated-by-refusal before={before:?},after={after_refusal:?}"),
        );
        return Ok(());
    }
    if before.total == 0 || before.keys.is_empty() {
        row.fail(
            "completion-observed",
            "accepted-command-232-page-total=0,keys=[];need-player-earned-Soulpit-survivor-completion-before-species-XP-command-path",
        );
        return Ok(());
    }
    row.pass("completion-observed", (&before.total, &before.keys));

    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        row.fail("fresh-admission-persistence", "logout-reply-not-accepted");
        return Ok(());
    }
    row_needs_logout_0 = false;
    let mut fresh = connect_world(clients, 0xc9).await?;
    let row_needs_logout_1 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let durable = page(&mut fresh).await?;
    if durable != before {
        row.fail(
            "fresh-admission-persistence",
            format!("before={before:?},fresh={durable:?}"),
        );
        return Ok(());
    }
    row.pass(
        "fresh-admission-persistence",
        (&durable.total, &durable.keys),
    );

    // Reaching this branch proves durable completion. The native client currently
    // exposes no species identity on visible actors, so selecting and proving the
    // matching kill without guessing would be false evidence.
    row.fail(
        "species-xp-multiplier",
        format!(
            "durable-species={:?};missing-native-visible-actor-species-key-needed-to-select-matching-combat-target",
            durable.keys.first()
        ),
    );
    Ok(())

    }).await.unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_1 { tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??; }
    row_result

    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(evidence_token("reply=[]; need x"), "reply=----need-x");
    }
}
