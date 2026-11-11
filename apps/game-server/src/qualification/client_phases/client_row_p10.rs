//! P10 client capture. The row's observable effect is resting-area health, mana, stamina and soul
//! regeneration that needs a seven-day daily-reward streak. The client consumers exist (daily wall
//! in `0182-daily-reward`, actor vitals in `crates/session`), but the streak needs seven committed
//! server-save epochs, which a bounded run cannot produce, so no replay test can assert the bonus.
use super::*;
use oteryn_dev_client::client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, decode_reply,
};
use oteryn_protocol_oteryn::daily_reward as wire;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x2c).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
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
    capture("p10-wall.bin", &query.payload)?;
    let DailyRewardReply::Wall(wall) = decode_reply(DailyRewardCommand::Query, &query.payload)?
    else {
        return Err("wall query did not decode as a wall".into());
    };
    let vitals = *client.actor_vitals().ok_or("actor vitals frame absent")?;
    Ok(format!(
        "streak={};lane_day={};health={}/{};mana={}/{};soul={};resting_bonus_unlocks_at_streak=2",
        wall.streak,
        wall.lane_day,
        vitals.health,
        vitals.max_health,
        vitals.mana,
        vitals.max_mana,
        vitals.soul
    ))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P10.client status=FAIL evidence=needed:resting-regeneration-journey;{observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P10.client status=FAIL evidence=timeout"),
    }
}
