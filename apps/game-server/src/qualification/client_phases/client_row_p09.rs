//! P09 client replay capture: the real daily-reward wall (Query without opening the shrine) and
//! the claim the server refuses while the shrine is unopened. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0182-daily-reward/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, decode_reply,
};
use oteryn_protocol_oteryn::daily_reward as wire;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P09_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2b).await?;
    let query = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if query.status != CommandStatus::Accepted {
        return Err(format!("wall query status={:?}", query.status).into());
    }
    capture("p09-wall.bin", &query.payload);
    let DailyRewardReply::Wall(wall) = decode_reply(DailyRewardCommand::Query, &query.payload)?
    else {
        return Err("wall query did not decode as a wall".into());
    };
    let claim = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_CLAIM,
            &[],
        )
        .await?;
    if claim.payload.is_empty() {
        return Err(format!("claim reply without payload status={:?}", claim.status).into());
    }
    capture("p09-claim.bin", &claim.payload);
    let DailyRewardReply::Result(result) = decode_reply(DailyRewardCommand::Claim, &claim.payload)?
    else {
        return Err("claim did not decode as a result".into());
    };
    let status = if matches!(claim.status, CommandStatus::Accepted) {
        "accepted"
    } else {
        "refused"
    };
    let expected = format!(
        "lane_day={}\nstreak={}\njokers={}\nclaimed={}\nclaim_enabled={}\nclaim_status={status}\nclaim_disposition={}\n",
        wall.lane_day, wall.streak, wall.jokers, wall.claimed, wall.claim_enabled, result.disposition
    );
    capture("p09-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P09.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P09.client status=FAIL evidence=timeout"),
    }
}
