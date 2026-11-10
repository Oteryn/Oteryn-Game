//! G37: client parity package 3 (#3075): 0007-chat, 0009-highscores, 0042-party, 0062-stash,
//! 0063-market, 0068-imbue, 0072-task-board, 0182-daily-reward, 0461-party-list,
//! 0462-support-intake, 0463-action-bar-cooldowns, 0474-task-board-weekly-results and
//! 0479-store-cosmetic-tokens. The package is mostly client presentation; this row drives the
//! server-backed reads those windows render, through the dev client:
//! 1. highscores page 1 (retrying only an explicit empty refusal): identity and row bound.
//! 2. daily-reward wall QUERY: accepted and decodable as a wall.
//! 3. task-board QUERY (the weekly-results window's source): a decodable reply.
//! 4. walk to the depot locker and stash BROWSE: `Opened` with the page echoed.
//! 5. log out and require Accepted.
//!
//! A failed probe makes the row FAIL. When every probe passes the remaining windows (party list,
//! support intake, action-bar cooldowns, store cosmetic tokens, imbue, market, chat dialog) have
//! no dev-client driver, so the row ends MISSING-CLIENT naming the package's client sources.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{
    daily_reward,
    highscores::{self, wire as hs},
    task_board,
};
use oteryn_protocol_oteryn::stash::{StashAction, StashIntent, StashOutcome};
use oteryn_protocol_oteryn::task_board::full::Request;

const SOURCE: &str = "source:apps/client/src/features/0462-support-intake/input.rs:1";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(150), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(failures)) if failures.is_empty() => ("MISSING-CLIENT", SOURCE.to_owned()),
        Ok(Ok(failures)) => ("FAIL", token(&failures.join(";"))),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G37.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Record a probe result: `Ok` keeps going, an error becomes a `probe=observed` failure entry.
fn record(failures: &mut Vec<String>, probe: &str, result: TestResult) {
    if let Err(error) = result {
        failures.push(format!("probe={probe};observed={error}"));
    }
}

async fn highscores(client: &mut DevClientSession) -> TestResult {
    let query = hs::HighscoresQuery {
        world: None,
        details: false,
        category: 1,
        vocation: 0,
        page: 1,
        snapshot_id: 0,
    };
    let command = highscores::command(query).map_err(|error| format!("input:{error:?}"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let reply = loop {
        let reply = client
            .session_mut()
            .gated_command(command.capability, command.command_type, &command.payload)
            .await?;
        if reply.status == CommandStatus::Accepted {
            break reply;
        }
        if reply.status != CommandStatus::Rejected
            || !reply.payload.is_empty()
            || tokio::time::Instant::now() >= deadline
        {
            return Err(format!("status:{:?}", reply.status).into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    };
    let page = hs::decode_highscores_result(&reply.payload)
        .map_err(|error| format!("decode:{error:?}"))?;
    if page.category != 1
        || page.page != 1
        || page.snapshot_id == 0
        || page.rows.len() > hs::MAX_HIGHSCORES_PAGE_ROWS
    {
        return Err("identity-mismatch".into());
    }
    Ok(())
}

async fn daily_wall(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::daily_reward as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("status:{:?}", reply.status).into());
    }
    match daily_reward::decode_reply(daily_reward::DailyRewardCommand::Query, &reply.payload)? {
        daily_reward::DailyRewardReply::Wall(_) => Ok(()),
        daily_reward::DailyRewardReply::Result(_) => Err("not-a-wall".into()),
    }
}

async fn task_board_query(client: &mut DevClientSession) -> TestResult {
    task_board::send_full(
        client.session_mut(),
        &Request {
            action: 4,
            selection: 0,
            race: String::new(),
        },
    )
    .await
    .map(|_| ())
    .map_err(|error| format!("{error:?}").into())
}

async fn stash_browse(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use oteryn_dev_client::client_runtime::stash;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        near_locker(world, at)
    })
    .ok_or("needed=reachable-depot-locker")?;
    walk(client, world, &path).await?;
    let command = stash::StashCommand::new(StashIntent {
        page: 0,
        action: StashAction::Browse,
    })
    .ok_or("input-refused")?;
    let response = stash::send(client.session_mut(), &command).await?;
    if response.status != CommandStatus::Accepted || response.reply.outcome != StashOutcome::Opened
    {
        return Err(format!(
            "status:{:?},outcome:{:?}",
            response.status, response.reply.outcome
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Vec<String>> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xe8).await?;
    let mut failures = Vec::new();
    let result = highscores(&mut client).await;
    record(&mut failures, "highscores", result);
    let result = daily_wall(&mut client).await;
    record(&mut failures, "daily-reward", result);
    let result = task_board_query(&mut client).await;
    record(&mut failures, "task-board", result);
    let result = stash_browse(&mut client, world).await;
    record(&mut failures, "stash", result);
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        failures.push("probe=logout;observed=refused".to_owned());
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_keeps_only_failed_probes() {
        let mut failures = Vec::new();
        record(&mut failures, "a", Ok(()));
        record(&mut failures, "b", Err("boom".into()));
        assert_eq!(failures, vec!["probe=b;observed=boom".to_owned()]);
        assert_eq!(token("x  y"), "x-y");
    }
}
