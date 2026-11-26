//! P09 client replay: drive the daily-reward panel with the current journey's wire replies.
use super::*;
use oteryn_dev_client::client_ui::daily_reward::{
    DailyRewardCommand, DailyRewardPanel, DailyRewardReply, DailyRewardResponse, Intent,
    apply_result, decode_reply, panel_text, request,
};
use oteryn_protocol_oteryn::daily_reward as wire;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::fs::File::create_new(Path::new(&root).join(name))?.write_all(bytes)?;
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2b).await?;
    let mut panel = DailyRewardPanel {
        selected: true,
        ..DailyRewardPanel::default()
    };
    if request(&mut panel, Intent::Refresh) != Some(DailyRewardCommand::Query) {
        return Err("daily panel did not request a wall refresh".into());
    }
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
    let DailyRewardReply::Wall(wall) = decode_reply(DailyRewardCommand::Query, &query.payload)?
    else {
        return Err("wall query did not decode as a wall".into());
    };
    if wall.claimed || wall.claim_enabled || !(1..=30).contains(&wall.lane_day) {
        return Err(format!("expected fresh unopened wall observed={wall:?}").into());
    }
    if !apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: query.status,
            reply: Some(DailyRewardReply::Wall(wall)),
        },
    ) || panel.wall != Some(wall)
    {
        return Err("daily panel did not apply the live wall".into());
    }
    let text = panel_text(&panel);
    for expected in [
        format!("Day {} of 7", (wall.lane_day - 1) % 7 + 1),
        format!("Streak: {}", wall.streak),
        format!("Jokers: {}", wall.jokers),
        "Status: not available".to_owned(),
    ] {
        if !text.lines().any(|line| line == expected) {
            return Err(format!("daily panel omitted {expected:?}: {text:?}").into());
        }
    }
    if request(&mut panel, Intent::Claim).is_some()
        || panel.feedback != Some("The daily reward cannot be claimed now")
    {
        return Err("daily panel did not disable the unopened claim".into());
    }
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
    let DailyRewardReply::Result(result) = decode_reply(DailyRewardCommand::Claim, &claim.payload)?
    else {
        return Err("claim did not decode as a result".into());
    };
    if claim.status != CommandStatus::Rejected
        || result.disposition != 4
        || result.transaction_id.is_some()
    {
        return Err(format!(
            "expected unopened shrine refusal status={:?} result={result:?}",
            claim.status
        )
        .into());
    }
    if apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: claim.status,
            reply: Some(DailyRewardReply::Result(result)),
        },
    ) || panel.wall != Some(wall)
        || panel.feedback != Some("Open the daily reward shrine within reach, then refresh")
    {
        return Err("daily panel did not retain the wall and display the shrine refusal".into());
    }
    let expected = format!(
        "lane_day={}\nstreak={}\njokers={}\nclaimed={}\nclaim_enabled={}\nclaim_status=refused\nclaim_disposition={}\n",
        wall.lane_day,
        wall.streak,
        wall.jokers,
        wall.claimed,
        wall.claim_enabled,
        result.disposition
    );
    capture("p09-wall.bin", &query.payload)?;
    capture("p09-claim.bin", &claim.payload)?;
    capture("p09-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P09.client status=PASS evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P09.client status=FAIL evidence=timeout"),
    }
}
