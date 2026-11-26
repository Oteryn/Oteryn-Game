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
    capture(root, "w11-taints.bin", &taints.payload)?;
    capture(root, "w11-lever.bin", &lever.payload)?;
    let expected = format!(
        "now={}\nsoul_war={}\nrotten_blood={}\nrotten_blood_flags={}\nrotten_blood_final={}\nlever_status={:?}\nactivity=refused:Level\n",
        snapshot.now,
        taint.soul_war,
        taint.rotten_blood,
        taint.rotten_blood_flags,
        taint.rotten_blood_final,
        lever.status
    );
    capture(root, "w11-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
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
