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
        evidence(&format!(
            "P09_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn shrine_claim_refused(
    disposition: u8,
    has_transaction: bool,
    wall_unchanged: bool,
    custody_unchanged: bool,
) -> bool {
    disposition == 4 && !has_transaction && wall_unchanged && custody_unchanged
}

#[test]
fn unopened_shrine_rejects_grants_and_other_refusals() {
    assert!(!shrine_claim_refused(1, true, true, true));
    assert!(!shrine_claim_refused(5, false, true, true));
    assert!(shrine_claim_refused(4, false, true, true));
    assert!(!shrine_claim_refused(4, false, false, true));
    assert!(!shrine_claim_refused(4, false, true, false));
}

async fn daily_claims(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_daily_reward_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(clients.characters[0].as_slice()).bind(clients.world.as_slice())
        .fetch_one(connection).await?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    // The full row claims today's reward on Character one.
    let mut client = connect_character(clients, 0x2b, 0).await?;
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
    if wall.claimed || wall.claim_enabled {
        return Err(format!("unopened shrine fixture requires an unclaimed day: {wall:?}").into());
    }
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let claims_before = daily_claims(&mut connection, clients).await?;
    let inventory_before = client
        .inventory()
        .ok_or("initial inventory missing")?
        .clone();
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
    let reread = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    let wall_unchanged =
        reread.status == CommandStatus::Accepted && reread.payload == query.payload;
    let custody_unchanged = client.inventory() == Some(&inventory_before)
        && daily_claims(&mut connection, clients).await? == claims_before;
    if claim.status != CommandStatus::Rejected
        || !shrine_claim_refused(
            result.disposition,
            result.transaction_id.is_some(),
            wall_unchanged,
            custody_unchanged,
        )
    {
        return Err(format!("unopened shrine claim result={result:?}").into());
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
    capture("p09-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P09.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P09.client status=FAIL evidence=timeout"),
    }
}
