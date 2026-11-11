//! W02 client replay capture: the real BOSS_COOLDOWNS reply and the BOSS_LEVER reply for the
//! Faceless Bane lever sent from the admission position (the server's typed refusal or handoff).
//! The consumer is `apply_result` in `crates/client-ui/src/features/0117-boss-encounter/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::boss_encounter as wire;

const BOSS: &str = "oteryn:boss.faceless_bane";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x01).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let query = wire::query_taints("").ok_or("cooldown encoder")?;
        let cooldowns = client
            .session_mut()
            .gated_command(query.capability, query.kind, &query.payload)
            .await?;
        if cooldowns.status != CommandStatus::Accepted {
            return Err(format!("cooldown query status={:?}", cooldowns.status).into());
        }
        let snapshot = wire::decode(&cooldowns.payload).ok_or("malformed cooldown snapshot")?;
        let lever = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                wire::COMMAND_TYPE_BOSS_LEVER,
                &wire::encode_boss_key(BOSS).map_err(|error| format!("boss key: {error:?}"))?,
            )
            .await?;
        let reply = wire::decode_boss_activity_reply(&lever.payload).map_err(|error| {
            format!("lever reply malformed: {error:?} status={:?}", lever.status)
        })?;
        let activity = match &reply {
            wire::BossActivityReply::Accepted {
                deadline_unix_ms, ..
            } => format!("accepted:{deadline_unix_ms:?}"),
            wire::BossActivityReply::Refused(reason) => format!("refused:{reason:?}"),
        };
        capture("w02-cooldowns.bin", &cooldowns.payload)?;
        capture("w02-lever.bin", &lever.payload)?;
        let cooldown_text: Vec<String> = snapshot
            .cooldowns
            .iter()
            .map(|(key, at)| format!("{key}={at}"))
            .collect();
        let expected = format!(
            "now={}\npoints={}\ncooldowns={}\nlever_status={:?}\nactivity={activity}\n",
            snapshot.now,
            snapshot.points,
            cooldown_text.join(","),
            lever.status
        );
        capture("w02-expected.txt", expected.as_bytes())?;
        Ok(expected.replace('\n', ";"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W02.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W02.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W02.client status=FAIL evidence=timeout"),
    }
}
