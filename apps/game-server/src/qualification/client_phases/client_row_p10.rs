//! P10 client capture. The row's observable effect is resting-area health, mana, stamina and soul
//! regeneration that needs a seven-day daily-reward streak. The client consumers exist (daily wall
//! in `0182-daily-reward`, actor vitals in `crates/session`), but the streak needs seven committed
//! server-save epochs, which a bounded run cannot produce, so no replay test can assert the bonus.
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
        evidence(&format!("P10_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2c).await?;
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
    capture("p10-wall.bin", &query.payload);
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
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P10.client status=MISSING-CLIENT evidence={observed};searched=crates/session,crates/client-ui/src/features/0182-daily-reward,crates/client-runtime/src/features/*daily-reward*,tools/dev-client,apps/client/src/features/0182-daily-reward;gap=resting_regen_needs_seven_epoch_streak_not_reachable_in_run"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P10.client status=FAIL evidence=timeout"),
    }
}
