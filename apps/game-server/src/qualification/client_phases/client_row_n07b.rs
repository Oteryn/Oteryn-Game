//! N07b client replay capture: two real BOSS_COOLDOWNS replies, a moment apart, that feed the
//! independent Boss cooldown tracker. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0117-boss-encounter/protocol.rs`, projected by `cooldown_text`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("N07B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn cooldowns(client: &mut DevClientSession) -> TestResult<(Vec<u8>, wire::Snapshot)> {
    let query = wire::query_taints("").ok_or("cooldown encoder")?;
    let reply = client
        .session_mut()
        .gated_command(query.capability, query.kind, &query.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("cooldown query status={:?}", reply.status).into());
    }
    let snapshot = wire::decode(&reply.payload).ok_or("malformed cooldown snapshot")?;
    Ok((reply.payload, snapshot))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xfb).await?;
    let (first, snapshot) = cooldowns(&mut client).await?;
    capture("n07b-cooldowns.bin", &first);
    tokio::time::sleep(Duration::from_millis(1_300)).await;
    let (later, next) = cooldowns(&mut client).await?;
    if next.now <= snapshot.now {
        return Err(format!("server time did not advance: {} then {}", snapshot.now, next.now).into());
    }
    capture("n07b-cooldowns-later.bin", &later);
    let listed: Vec<String> = snapshot
        .cooldowns
        .iter()
        .map(|(key, at)| format!("{key}={at}"))
        .collect();
    let expected = format!(
        "now={}\npoints={}\ncooldowns={}\nmore={}\nnow_later={}\n",
        snapshot.now,
        snapshot.points,
        listed.join(","),
        snapshot.cooldown_next.is_some() || snapshot.progress_next.is_some(),
        next.now
    );
    capture("n07b-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N07b.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07b.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07b.client status=FAIL evidence=timeout"),
    }
}
