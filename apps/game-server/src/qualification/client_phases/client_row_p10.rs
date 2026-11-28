//! P10 claims each daily reward over the live session and replays the resting-status replies.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::daily_reward::{
    DailyRewardCommand, DailyRewardReply, decode_reply, send,
};
use oteryn_dev_client::client_ui::rest_status::RestStatusPanel;
use oteryn_protocol_oteryn::{daily_reward, rest_status, world_object::UseDisposition};
use sqlx::Connection;
use std::io::Write;

async fn wall(client: &mut DevClientSession) -> TestResult<daily_reward::DailyRewardWall> {
    let reply = client
        .session_mut()
        .gated_command(
            daily_reward::CAPABILITY_DAILY_REWARD_V2,
            daily_reward::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("P10 wall refused: {:?}", reply.status).into());
    }
    match decode_reply(DailyRewardCommand::Query, &reply.payload)? {
        DailyRewardReply::Wall(wall) => Ok(wall),
        _ => Err("P10 invalid wall".into()),
    }
}

fn shrine_route(world: &BundleWorld, start: TilePos) -> TestResult<Vec<StepDirection>> {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let content = crate::content::daily_reward::DailyRewardContent::catalogue()
        .ok_or("P10 daily catalogue missing")?;
    let index = provisioning::item_index()?;
    let references = content
        .shrine_source_ids
        .iter()
        .map(|id| provisioning::item_reference(&index, *id))
        .collect::<TestResult<Vec<_>>>()?;
    let facts = world.facts().pinned();
    let shrine = world.base().tiles()
        .filter(|(floor, x, y, _)| *floor == start.floor && start.x.abs_diff(*x) <= 160 && start.y.abs_diff(*y) <= 160)
        .filter_map(|(floor, x, y, data)| {
            let at = TilePos { floor, x, y };
            data.ids().iter().zip(data.depths()).filter(|(_, depth)| **depth == 0)
                .enumerate().any(|(ordinal, (id, _))| {
                    u8::try_from(ordinal).ok().and_then(|n| facts.base_entry(at, n, *id))
                        .is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(reference) if references.contains(&reference)))
                }).then_some(at)
        })
        .min_by_key(|at| (start.x.abs_diff(at.x).max(start.y.abs_diff(at.y)), *at))
        .ok_or("P10 no daily shrine on the starting floor")?;
    route(world, start, |at| {
        at.floor == shrine.floor
            && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|data| data.protection_zone())
    })
    .ok_or_else(|| format!("P10 no resting approach to shrine {shrine:?}").into())
}

async fn advance_save(database: &mut sqlx::PgConnection, world_id: &str, epoch: i64) -> TestResult {
    // As in the Dispatch daily reward PG fixture, provide only the World owner's
    // calendar input. The character's claim ledger and streak are written by wire claims.
    let committed: i64 = sqlx::query_scalar(
        "INSERT INTO game_world_reset_history
         (world_id, reset_epoch, reset_day, reset_at_unix_ms, committed_at_unix_ms)
         SELECT world_id, reset_epoch + 1, reset_day + 1, next_save,
                greatest(next_save, committed_at_unix_ms)
           FROM game_world_reset_history
          CROSS JOIN LATERAL (
             SELECT (extract(epoch FROM (((DATE '1970-01-01' + (reset_day + 1)::integer)
                      + TIME '06:00') AT TIME ZONE 'Europe/Berlin')) * 1000)::bigint AS next_save
          ) save
          WHERE world_id = $1::uuid AND reset_epoch = $2::bigint
          RETURNING reset_epoch::bigint",
    )
    .bind(world_id)
    .bind(epoch - 1)
    .fetch_one(database)
    .await?;
    if committed != epoch {
        return Err(format!("P10 expected committed epoch {epoch}, got {committed}").into());
    }
    evidence(&format!("P10_WORLD_FIXTURE committed_epoch={committed}"));
    Ok(())
}

async fn capture_status(
    client: &mut DevClientSession,
    root: &Path,
) -> TestResult<std::path::PathBuf> {
    let session = client.join_snapshot().game_session_id;
    let reply = client
        .session_mut()
        .gated_command(
            rest_status::CAPABILITY_REST_STATUS_V1,
            rest_status::COMMAND_TYPE_REST_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.server_sequence == 0 {
        return Err(format!("P10 rest status refused: {reply:?}").into());
    }
    let path = root.join(format!(
        "p10-{}-{}.bin",
        uuid_text(session.as_bytes()),
        reply.server_sequence
    ));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .write_all(&reply.payload)?;
    Ok(path)
}

fn replay(captures: &[(u32, std::path::PathBuf)]) -> TestResult {
    let mut panel = RestStatusPanel {
        selected: true,
        ..Default::default()
    };
    for (streak, path) in captures {
        if !panel.finish(true, &std::fs::read(path)?) {
            return Err(format!("P10 client reducer rejected streak {streak}").into());
        }
        let expected = rest_status::RestStatus {
            inside_resting_area: true,
            streak: *streak,
            food_remaining_ms: 0,
            health_multiplier: if *streak >= 5 {
                2
            } else {
                u8::from(*streak >= 2)
            },
            mana_multiplier: if *streak >= 6 {
                2
            } else {
                u8::from(*streak >= 3)
            },
            stamina: *streak >= 4,
            soul: *streak >= 7,
        };
        if panel.snapshot != Some(expected) {
            return Err(
                format!("P10 replay expected {expected:?}, got {:?}", panel.snapshot).into(),
            );
        }
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P10 fresh capture directory required")?;
    let root = Path::new(&root);
    let _ = required("NODE_BOOT_CONTROL_SOCKET")?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P10_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let clients = &scope;
    let world_id = uuid_text(&clients.world);
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut client = connect_character(clients, 0x2c, 0).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = shrine_route(world, start)?;
    walk(&mut client, world, &path).await?;
    let resting_tile = tile(client.world_spatial().actor_position)?;
    let mut captures = vec![(0, capture_status(&mut client, root).await?)];
    for streak in 1..=7_u32 {
        if streak > 1 {
            advance_save(&mut database, &world_id, i64::from(streak)).await?;
        }
        let before = wall(&mut client).await?;
        if before.claimed || before.streak != streak - 1 {
            return Err(format!("P10 epoch {streak} unexpected pre-claim wall: {before:?}").into());
        }
        let opened = client
            .session_mut()
            .use_object(b"oteryn:placement/daily-reward-shrine", 0)
            .await?;
        let ready = wall(&mut client).await?;
        if opened.disposition != UseDisposition::Committed
            || !ready.claim_enabled
            || ready.claimed
            || ready.streak != before.streak
            || ready.lane_day != before.lane_day
        {
            return Err(format!(
                "P10 epoch {streak} shrine {resting_tile:?}: {opened:?}, {ready:?}"
            )
            .into());
        }
        let reply = send(client.session_mut(), &DailyRewardCommand::Claim).await?;
        let capture = capture_status(&mut client, root).await?;
        let claimed = wall(&mut client).await?;
        if reply.status != CommandStatus::Accepted
            || !matches!(reply.reply, Some(DailyRewardReply::Result(r)) if r.disposition == 1 && r.transaction_id.is_some())
            || !claimed.claimed
            || claimed.claim_enabled
            || claimed.streak != streak
        {
            return Err(format!("P10 epoch {streak} claim: {reply:?}, {claimed:?}").into());
        }
        captures.push((streak, capture));
    }
    let persisted = wall(&mut client).await?;
    let previous = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P10 logout refused: {logout:?}").into());
    }
    drop(client);
    let mut client = connect_character(clients, 0x2d, 0).await?;
    if client.join_snapshot().game_session_id == previous
        || tile(client.world_spatial().actor_position)? != resting_tile
    {
        return Err("P10 streak or resting position did not survive relog".into());
    }
    captures.push((7, capture_status(&mut client, root).await?));
    if wall(&mut client).await? != persisted {
        return Err("P10 daily wall did not survive relog".into());
    }
    let claims: Vec<(i64, i32, bool)> = sqlx::query_as(
        "SELECT reset_epoch::bigint, streak_after, premium
           FROM game_character_daily_reward_claims
          WHERE world_id=$1::uuid AND character_id=encode($2,'hex')::uuid
          ORDER BY reset_epoch LIMIT 8",
    )
    .bind(&world_id)
    .bind(clients.characters[0].as_slice())
    .fetch_all(&mut database)
    .await?;
    let expected: Vec<_> = (1..=7)
        .map(|streak| (i64::from(streak), streak, true))
        .collect();
    if claims != expected {
        return Err(format!("P10 durable claim epochs/streak/Premium: {claims:?}").into());
    }
    replay(&captures)?;
    Ok(
        "seven_wire_claims;streak=7;hp=2;mana=2;stamina=true;soul=true;relog=true;client_replay=9"
            .into(),
    )
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(
        Duration::from_secs(120),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P10.client status=PASS evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P10.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P10.client status=FAIL evidence=timeout"),
    }
}
