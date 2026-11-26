//! W08 full-row journey: Animus Mastery completion and species XP multiplier.
//!
//! Concrete player scenario derived from the W08 audit/matrix row and the named
//! `kill_reward`/Animus owners:
//! The dedicated character starts with a Rat Soul Core, a seeded Knight build,
//! and an empty mastery page. Command 232 must reject a zero limit without
//! changing the page. The intended gameplay leg earns Rat mastery in Soulpit,
//! then checks the exact species after relog and measures its XP bonus.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::animus_mastery as wire;
use std::collections::BTreeSet;

const RAT: &str = "oteryn:creature.rat";

const ACTIONS: &[&str] = &[
    "seeded-prerequisites",
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
            verdict::record_assertion(action);
            self.failure = Some(format!("duplicate-or-unknown-action:{action}"));
        }
    }

    fn fail(&mut self, step: &'static str, observed: impl std::fmt::Display) {
        verdict::record_assertion(step);
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
    let result = verdict::indexed_timeout(
        Duration::from_secs(90),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
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
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W08_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    character_prep::declare_seeded(
        "W08",
        "knight-level=100,rat-soul-core=1",
        "ordinary leveling and core acquisition are outside the mastery scenario",
    );
    let mut setup = connect_character(clients, 0xc8, 0).await?;
    let setup_session = setup.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(setup_session.as_bytes()),
        "--profile",
        "w08-soulpit".into(),
    )
    .await?;
    if setup.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("W08 fixture setup logout refused".into());
    }
    drop(setup);
    let mut client = connect_character(clients, 0xc9, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("W08 fixture setup reused its admission".into());
    }
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "knight",
        100,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("W08 Knight vitals unavailable")?;
    let vitals = client.actor_vitals().ok_or("W08 admitted without vitals")?;
    if (vitals.max_health, vitals.max_mana) != (max_health, max_mana) {
        return Err(format!("W08 seeded Knight vitals differ: {vitals:?}").into());
    }
    let core = provisioning::item_reference(&provisioning::item_index()?, 47398)?;
    let core_count: u64 = client
        .inventory()
        .ok_or("W08 admitted without inventory")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == core)
        .map(|entry| u64::from(entry.count.get()))
        .sum();
    if core_count != 1 {
        return Err(format!("W08 expected one Rat Soul Core, found {core_count}").into());
    }
    row.pass("seeded-prerequisites", (max_health, max_mana, core_count));
    let before = page(&mut client).await?;
    if before.total != 0 || !before.keys.is_empty() {
        row.fail(
            "initial-query",
            format!("expected empty mastery page, got {before:?}"),
        );
        return Ok(());
    }
    row.pass("initial-query", (&before.total, &before.keys));

    let refused = command(&mut client, &[0, 0, 0, 0]).await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        row.fail("invalid-query-refused", format!("reply={refused:?}"));
        return Ok(());
    }
    let after_refusal = page(&mut client).await?;
    if after_refusal != before {
        row.fail(
            "invalid-query-refused",
            format!("mastery-set-mutated-by-refusal before={before:?},after={after_refusal:?}"),
        );
        return Ok(());
    }
    row.pass("invalid-query-refused", (refused.status, &after_refusal));

    // The acquisition driver must earn mastery before the completion query.
    let completed = page(&mut client).await?;
    if completed.total != 1 || completed.keys.as_slice() != [RAT] {
        row.fail(
            "completion-observed",
            format!("expected player-earned Rat mastery, got {completed:?}"),
        );
        return Ok(());
    }
    row.pass("completion-observed", (&completed.total, &completed.keys));

    let mastery_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        row.fail("fresh-admission-persistence", "logout-reply-not-accepted");
        return Ok(());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0xca, 0).await?;
    if fresh.join_snapshot().game_session_id == mastery_session {
        row.fail(
            "fresh-admission-persistence",
            "mastery admission was reused",
        );
        return Ok(());
    }
    let durable = page(&mut fresh).await?;
    if durable != completed {
        row.fail(
            "fresh-admission-persistence",
            format!("completed={completed:?},fresh={durable:?}"),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(evidence_token("reply=[]; need x"), "reply=----need-x");
    }
}
