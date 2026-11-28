//! W11 client replay capture: the real BOSS_COOLDOWNS reply with the Soul War / Rotten Blood taint
//! projection, the starter's typed Goshnar's Cruelty BOSS_LEVER level refusal and a malformed
//! lever refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0117-boss-encounter/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter as wire;

const BOSS: &str = "oteryn:creature.goshnar_s_cruelty";

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?
        .write_all(bytes)?;
    Ok(())
}

async fn read_taints(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<wire::Taints> {
    let query = wire::query_taints("").ok_or("taint query encoder")?;
    let reply = client
        .session_mut()
        .gated_command(query.capability, query.kind, &query.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("taint readback status={:?}", reply.status).into());
    }
    wire::decode(&reply.payload)
        .and_then(|snapshot| snapshot.taints)
        .ok_or_else(|| "taint readback missing".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("W11 capture directory missing")?;
    let root = Path::new(&root);
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
    if lever.status != CommandStatus::Rejected
        || reply != wire::BossActivityReply::Refused(wire::BossActivityRefusal::Level)
    {
        return Err(format!("starter lever status={:?} reply={reply:?}", lever.status).into());
    }
    if read_taints(&mut client).await? != taint {
        return Err("typed lever refusal changed taints".into());
    }
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
    if read_taints(&mut client).await? != taint {
        return Err("malformed lever refusal changed taints".into());
    }
    let original = client.join_snapshot().game_session_id;
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W11 client logout refused: {outcome:?}").into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xb5).await?;
    if fresh.join_snapshot().game_session_id == original || read_taints(&mut fresh).await? != taint
    {
        return Err("W11 refusal taints did not survive fresh admission".into());
    }
    let outcome = fresh.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W11 client cleanup logout refused: {outcome:?}").into());
    }

    let session = uuid_text(original.as_bytes());
    let frames = [taints, lever, malformed].map(|frame| {
        (
            frame.command_id,
            frame.server_sequence,
            format!("{:?}", frame.status),
            frame.payload,
        )
    });
    let expected = (
        taint.soul_war,
        taint.rotten_blood,
        taint.rotten_blood_flags,
        taint.rotten_blood_final,
    );
    capture(
        root,
        "w11-live.json",
        &serde_json::to_vec(&(&session, frames, expected))?,
    )?;
    Ok(format!("session={session}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_capture scenario=W11.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W11.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W11.client status=FAIL evidence=timeout"),
    }
}
