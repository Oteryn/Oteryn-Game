//! P17: Exiva permission and player allow-list journey.
//!
//! Concrete player scenario derived from the P17 matrix/re-audit row and the
//! target-owned settings owner in `spell_privacy.rs`:
//! 1. The target sends Exiva SetModes(0); the server accepts the command and a
//!    List command reports no public/relationship permission.
//! 2. The caster says `exiva "<target>"`; the server rejects the spell and
//!    sends the protected-character reply, without changing caster vitals.
//! 3. The target enables player-list mode and sends AddName(`<caster>`); List
//!    returns mode 2 plus the caster's resolved character id and name.
//! 4. The caster repeats `exiva "<target>"`; the server accepts the spell,
//!    debits mana, and sends the directional Find Person reply.
//! 5. The target sends malformed SetModes(32); the server rejects it and a
//!    subsequent List proves that the allow-list state did not change.
//! 6. The target logs out and makes a fresh admission; List must return the
//!    same durable mode and player entry under a new game-session id.
//! 7. The target sends RemovePlayer(`<caster id>`); the server accepts it and
//!    List reports an empty player list while retaining mode 2.
//! 8. After the spell cooldown, the caster repeats the Exiva command; the
//!    server again rejects it with the protected-character reply and no
//!    vitals delta, proving the removal controls the live permission check.

use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::exiva::{
    CAPABILITY_EXIVA_SETTINGS_V1, COMMAND_TYPE_EXIVA_SETTINGS, PlayerList,
};
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "deny-modes",
    "denied-cast",
    "add-name",
    "allowed-cast",
    "malformed-refusal",
    "fresh-admission-state",
    "remove-player",
    "removed-cast-refusal",
];

const PROTECTED_REPLY: &str =
    "The character you are trying to find with Exiva is currently protected from your spell.";

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
    invalid: bool,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
            invalid: false,
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if !ACTIONS.contains(&action) || !self.passed.insert(action) {
            self.invalid = true;
            return;
        }
        evidence(&format!(
            "E2E_P17_CHECK {action} PASS observed={observed:?}"
        ));
    }

    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!(
                    "E2E_P17_CHECK {action} FAIL assertion-not-completed"
                ));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("E2E_P17_GAP step-failed observed={error}"));
        }
        let passed = result.is_ok()
            && !self.invalid
            && ACTIONS.iter().all(|action| self.passed.contains(action));
        evidence(&format!(
            "e2e_all scenario=P17.full status={} seconds={:.3} evidence={}",
            if passed { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if passed {
                "observed-settings-casts-refusal-and-durable-state"
            } else {
                "runtime-step-failure-see-E2E_P17_GAP"
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(120), journey(&mut row, clients))
        .await
        .unwrap_or_else(|_| Err("step deadline: P17 journey timed out".into()));
    row.finish(result);
}

async fn settings(client: &mut DevClientSession, payload: &[u8]) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_EXIVA_SETTINGS_V1,
            COMMAND_TYPE_EXIVA_SETTINGS,
            payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "settings command payload={payload:?}: status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into());
    }
    Ok(reply.payload)
}

async fn list(client: &mut DevClientSession) -> TestResult<PlayerList> {
    let payload = settings(client, &[5]).await?;
    PlayerList::decode(&payload)
        .ok_or_else(|| format!("List reply was not a PlayerList: {payload:?}").into())
}

async fn malformed_refused(client: &mut DevClientSession) -> TestResult {
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_EXIVA_SETTINGS_V1,
            COMMAND_TYPE_EXIVA_SETTINGS,
            &[0, 32],
        )
        .await?;
    if reply.status != CommandStatus::Rejected || reply.payload != [0] {
        return Err(format!(
            "step 5 malformed SetModes was not refused: status={:?} payload={:?}",
            reply.status, reply.payload
        )
        .into());
    }
    Ok(())
}

fn own_name(client: &DevClientSession) -> TestResult<String> {
    let entities = client.world_entities().ok_or("own-name entities absent")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .and_then(|entry| entry.display_name)
        .ok_or("own-name display name absent")?
        .as_str()
        .map(str::to_owned)
        .map_err(|error| format!("own-name invalid: {error:?}").into())
}

fn private_messages(client: &mut DevClientSession) -> Vec<String> {
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            oteryn_dev_client::SessionEvent::ChatLine(delta) => match delta.value {
                ChatLine::Private { speaker_name, text } if speaker_name == "Game" => Some(text),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

async fn cast(
    caster: &mut DevClientSession,
    target: &str,
    expected: ChatDisposition,
) -> TestResult<(u64, Vec<String>)> {
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("Exiva caster vitals absent")?;
    caster.take_events();
    let outcome = caster
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: format!("exiva \"{target}\""),
        })
        .await?;
    let expected_status = if expected == ChatDisposition::Rejected {
        CommandStatus::Rejected
    } else {
        CommandStatus::Accepted
    };
    if outcome.status != expected_status || outcome.disposition != expected {
        return Err(format!(
            "Exiva reply: status={:?} disposition={:?} expected={expected_status:?}/{expected:?}",
            outcome.status, outcome.disposition
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut messages = private_messages(caster);
    while messages.is_empty() && tokio::time::Instant::now() < deadline {
        caster.service_liveness(Duration::from_millis(50)).await?;
        messages.extend(private_messages(caster));
    }
    if messages.is_empty() {
        return Err(format!("Exiva {expected:?} reply had no Game feedback frame").into());
    }
    let after = caster
        .actor_vitals()
        .copied()
        .ok_or("Exiva caster post-cast vitals absent")?;
    if expected == ChatDisposition::Rejected && after != before {
        return Err(format!("refused Exiva changed vitals: {before:?} -> {after:?}").into());
    }
    if expected == ChatDisposition::Ok && before.mana.checked_sub(20) != Some(after.mana) {
        return Err(format!("allowed Exiva did not debit mana: {before:?} -> {after:?}").into());
    }
    Ok((outcome.result_server_sequence, messages))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P17 target logout refused".into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P17_ACCOUNT").await?;
    let caster_clients = identity.clients(clients);
    character_prep::declare_seeded("P17", "level-8-sorcerer", "Find Person requires level 8");
    let mut caster = connect_character(&caster_clients, 0xd5, 0).await?;
    let preparation_session = caster.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(preparation_session.as_bytes()),
        "--profile",
        "c05-exiva".into(),
    )
    .await?;
    logout(&mut caster).await?;
    drop(caster);
    let mut caster = connect_character(&caster_clients, 0xd7, 0).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        8,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("level-8 sorcerer vitals unavailable")?;
    if caster.join_snapshot().game_session_id == preparation_session
        || full_e2ef7::player_level(&mut caster).await? != 8
        || !caster.actor_vitals().is_some_and(|vitals| {
            vitals.max_health == max_health && vitals.max_mana == max_mana && vitals.mana >= 20
        })
    {
        return Err(format!(
            "P17 prepared caster readmission mismatch: {:?}",
            caster.actor_vitals()
        )
        .into());
    }
    evidence(&format!(
        "E2E_P17_CHECK caster-preparation PASS level=8 max_health={max_health} max_mana={max_mana}"
    ));
    let mut target = connect_character(clients, 0xd4, 0).await?;
    if !target
        .selected_capabilities()
        .contains(&CAPABILITY_EXIVA_SETTINGS_V1)
    {
        return Err("step 1 observed admission without Exiva settings capability 147; need server registration".into());
    }
    let target_name = own_name(&target)?;
    let caster_name = own_name(&caster)?;

    let initial = list(&mut target).await?;
    if matches!(initial.modes, 0 | 2) || !initial.players.is_empty() {
        return Err(format!(
            "step 1 expected a distinct enabled mode and no explicit entries: {initial:?}"
        )
        .into());
    }
    if settings(&mut target, &[0, 0]).await? != [1]
        || list(&mut target).await?
            != (PlayerList {
                modes: 0,
                players: Vec::new(),
            })
    {
        return Err("step 1 SetModes(0) lacked accepted receipt/list state delta".into());
    }
    row.pass("deny-modes", 0_u8);

    let (_, denied) = cast(&mut caster, &target_name, ChatDisposition::Rejected).await?;
    if !denied.iter().any(|line| line == PROTECTED_REPLY) {
        return Err(format!("step 2 denial lacked protected reply: {denied:?}").into());
    }
    row.pass("denied-cast", denied);

    if settings(&mut target, &[0, 2]).await? != [1]
        || list(&mut target).await?
            != (PlayerList {
                modes: 2,
                players: Vec::new(),
            })
    {
        return Err("step 3 SetModes(2) did not enable the empty explicit list".into());
    }
    let mut add = vec![6];
    add.extend(caster_name.as_bytes());
    if settings(&mut target, &add).await? != [1] {
        return Err("step 3 AddName lacked accepted receipt".into());
    }
    let allowed = list(&mut target).await?;
    let (caster_id, listed_name) = allowed
        .players
        .first()
        .ok_or("step 3 List lacked added player delta")?;
    if allowed.modes != 2
        || allowed.players.len() != 1
        || *caster_id != caster_clients.characters[0]
        || listed_name != &caster_name
    {
        return Err(format!("step 3 unexpected List delta: {allowed:?}").into());
    }
    row.pass("add-name", &allowed);

    let caster_tile = tile(caster.world_spatial().actor_position)?;
    let target_tile = tile(target.world_spatial().actor_position)?;
    if caster_tile.floor != target_tile.floor
        || caster_tile
            .x
            .abs_diff(target_tile.x)
            .max(caster_tile.y.abs_diff(target_tile.y))
            >= 5
    {
        return Err(format!(
            "Find Person near-target fixture mismatch: {caster_tile:?} -> {target_tile:?}"
        )
        .into());
    }
    let (_, located) = cast(&mut caster, &target_name, ChatDisposition::Ok).await?;
    if !located
        .iter()
        .any(|line| line == &format!("{target_name} is standing next to you."))
    {
        return Err(format!("step 4 allowed cast lacked Find Person reply: {located:?}").into());
    }
    row.pass("allowed-cast", located);

    malformed_refused(&mut target).await?;
    if list(&mut target).await? != allowed {
        return Err("step 5 malformed refusal changed the allow-list".into());
    }
    row.pass("malformed-refusal", CommandStatus::Rejected);

    let previous_session = target.join_snapshot().game_session_id;
    logout(&mut target).await?;
    drop(target);
    let mut target = connect_character(clients, 0xd6, 0).await?;
    if target.join_snapshot().game_session_id == previous_session
        || list(&mut target).await? != allowed
    {
        return Err("step 6 fresh admission did not retain the durable allow-list".into());
    }
    row.pass(
        "fresh-admission-state",
        target.join_snapshot().game_session_id,
    );

    let mut remove = vec![2];
    remove.extend(caster_id);
    if settings(&mut target, &remove).await? != [1] {
        return Err("step 7 RemovePlayer lacked accepted receipt".into());
    }
    let removed = list(&mut target).await?;
    if removed.modes != 2 || !removed.players.is_empty() {
        return Err(format!("step 7 removal delta mismatch: {removed:?}").into());
    }
    row.pass("remove-player", &removed);

    let removal_session = target.join_snapshot().game_session_id;
    logout(&mut target).await?;
    drop(target);
    let mut target = connect_character(clients, 0xd8, 0).await?;
    if target.join_snapshot().game_session_id == removal_session
        || list(&mut target).await? != removed
    {
        return Err("step 8 fresh admission did not retain the removed entry and mode".into());
    }
    let (_, denied) = cast(&mut caster, &target_name, ChatDisposition::Rejected).await?;
    if !denied.iter().any(|line| line == PROTECTED_REPLY) {
        return Err(format!("step 8 denial lacked protected reply: {denied:?}").into());
    }
    row.pass("removed-cast-refusal", denied);
    logout(&mut target).await?;
    logout(&mut caster).await?;
    release_row_characters(&caster_clients).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_requires_every_lifecycle_action() {
        let mut row = Row::new();
        for action in ACTIONS {
            row.pass(action, "unit");
        }
        assert!(!row.invalid);
        assert_eq!(row.passed.len(), ACTIONS.len());
    }
}
