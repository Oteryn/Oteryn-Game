//! W11 client replay capture: the real BOSS_COOLDOWNS reply with the Soul War / Rotten Blood taint
//! projection, the Goshnar's Cruelty BOSS_LEVER reply (typed refusal or handoff) and a malformed
//! lever refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0117-boss-encounter/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter as wire;

const BOSS: &str = "oteryn:creature.goshnar_s_cruelty";

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W11_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn remote_refusal(status: CommandStatus, reply: &wire::BossActivityReply) -> bool {
    status == CommandStatus::Rejected
        && matches!(
            reply,
            wire::BossActivityReply::Refused(
                wire::BossActivityRefusal::Position | wire::BossActivityRefusal::NoCapacity
            )
        )
}

#[test]
fn remote_soul_war_lever_is_checked_before_level() {
    use wire::{BossActivityRefusal as Refusal, BossActivityReply::Refused};
    assert!(!remote_refusal(
        CommandStatus::Accepted,
        &Refused(Refusal::Position)
    ));
    assert!(!remote_refusal(
        CommandStatus::Rejected,
        &Refused(Refusal::Level)
    ));
    assert!(remote_refusal(
        CommandStatus::Rejected,
        &Refused(Refusal::Position)
    ));
    assert!(remote_refusal(
        CommandStatus::Rejected,
        &Refused(Refusal::NoCapacity)
    ));
}

// A remote packet cannot reach the roster gate. Exercise the same prerequisite
// owner called by live dispatch with an otherwise qualified underlevel roster.
async fn verify_soul_war_level_gate() -> TestResult {
    use oteryn_gameplay_encounters::bosses::endgame_entry::{
        self, Encounter, EntryFacts, EntryRefusal,
    };
    let encounter = Encounter::from_boss_key(BOSS).ok_or("Soul War encounter not recognized")?;
    let mut participant = EntryFacts {
        character: [1; 16],
        character_revision: 1,
        level: 249,
        access: true,
        chain_ready: true,
    };
    let result = endgame_entry::enter(encounter, &[participant], |_| async { Ok(()) }).await;
    if result != Err(EntryRefusal::Level) {
        return Err(format!("step=roster-level expected Level at 249: {result:?}").into());
    }
    participant.level = 250;
    let result = endgame_entry::enter(encounter, &[participant], |_| async { Ok(()) }).await;
    if result != Ok(()) {
        return Err(format!(
            "step=roster-level expected admission to shared owner at 250: {result:?}"
        )
        .into());
    }
    Ok(())
}

#[test]
fn soul_war_roster_gate_enforces_level_before_shared_handoff() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(verify_soul_war_level_gate())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    verify_soul_war_level_gate().await?;
    let mut client = connect_world(clients, 0x02).await?;
    let query = wire::query_taints("").ok_or("taint query encoder")?;
    let taints = client
        .session_mut()
        .gated_command(query.capability, query.kind, &query.payload)
        .await?;
    if taints.status != CommandStatus::Accepted {
        return Err(format!("taint query status={:?}", taints.status).into());
    }
    let snapshot = wire::decode(&taints.payload).ok_or("malformed taint snapshot")?;
    let taint = snapshot.taints.ok_or("taint projection absent")?;
    let lever = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::COMMAND_TYPE_BOSS_LEVER,
            &wire::encode_boss_key(BOSS).map_err(|error| format!("boss key: {error:?}"))?,
        )
        .await?;
    let reply = wire::decode_boss_activity_reply(&lever.payload)
        .map_err(|error| format!("lever reply malformed: {error:?} status={:?}", lever.status))?;
    if !remote_refusal(lever.status, &reply) {
        return Err(format!(
            "lever did not return the expected remote Position or NoCapacity refusal: status={:?} reply={reply:?}",
            lever.status
        )
        .into());
    }
    let activity = match &reply {
        wire::BossActivityReply::Accepted {
            deadline_unix_ms, ..
        } => format!("accepted:{deadline_unix_ms:?}"),
        wire::BossActivityReply::Refused(reason) => format!("refused:{reason:?}"),
    };
    let malformed = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BOSS_ENCOUNTER_V1,
            wire::COMMAND_TYPE_BOSS_LEVER,
            &[10, 3, b'b'],
        )
        .await?;
    if malformed.status != CommandStatus::Rejected || !malformed.payload.is_empty() {
        return Err(format!("malformed lever was not refused: {malformed:?}").into());
    }
    capture("w11-taints.bin", &taints.payload);
    capture("w11-lever.bin", &lever.payload);
    let expected = format!(
        "now={}\nsoul_war={}\nrotten_blood={}\nrotten_blood_flags={}\nrotten_blood_final={}\nlever_status={:?}\nactivity={activity}\nroster_level_gate=249-refused-250-eligible\n",
        snapshot.now,
        taint.soul_war,
        taint.rotten_blood,
        taint.rotten_blood_flags,
        taint.rotten_blood_final,
        lever.status
    );
    capture("w11-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W11.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W11.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W11.client status=FAIL evidence=timeout"),
    }
}
