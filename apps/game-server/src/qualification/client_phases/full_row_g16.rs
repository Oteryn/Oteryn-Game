//! G16: daily reward details and one-click shrine claim (commands 738 and 354).
//! Global parity: https://www.tibiawiki.com.br/wiki/Daily_Reward_System
//! The Winter Update 2026 rework uses one click and keeps the daily bounty
//! reroll token on the Bounty Task Board.

use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_reward::{self as daily, details};
use oteryn_protocol_oteryn::world_map::MapDefinition;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "daily-shrine-claim-delivered-and-durable".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G16.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    let position = client.world_spatial().actor_position;
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={outcome:?} position={position:?}").into());
    }
    Ok(())
}

const CAPABILITY: u32 = daily::CAPABILITY_DAILY_REWARD_V2;

async fn wall(client: &mut DevClientSession) -> TestResult<daily::DailyRewardWall> {
    use oteryn_dev_client::client_runtime::daily_reward::{
        DailyRewardCommand, DailyRewardReply, decode_reply,
    };
    let payload = accepted(
        client,
        CAPABILITY,
        daily::COMMAND_TYPE_DAILY_REWARD_QUERY,
        &[],
    )
    .await?;
    match decode_reply(DailyRewardCommand::Query, &payload)? {
        DailyRewardReply::Wall(value) => Ok(value),
        observed => Err(format!("step=wall observed={observed:?}").into()),
    }
}

type RewardLine = (String, i32, Option<i16>);

struct ClaimEvidence {
    transaction: [u8; 16],
    epoch: u64,
    day: u8,
}

const DAY: u8 = 1;
const FREE_TIER: details::Tier = details::Tier {
    cosmetic_tokens: 2,
    prey_wildcards: 0,
    temple_scrolls: 0,
    xp_percent: 0,
    xp_minutes: 0,
    bounty_tokens: 0,
};

fn expected_lines(tier: details::Tier) -> Vec<RewardLine> {
    [
        ("BOUNTY_REROLL", tier.bounty_tokens, None),
        ("COSMETIC_TOKEN", tier.cosmetic_tokens, None),
        ("PREY_WILDCARD", tier.prey_wildcards, None),
        ("TEMPLE_SCROLL", tier.temple_scrolls, None),
        (
            "XP_BOOST",
            tier.xp_minutes,
            Some(i16::from(tier.xp_percent)),
        ),
    ]
    .into_iter()
    .filter(|(_, quantity, _)| *quantity > 0)
    .map(|(kind, quantity, percent)| (kind.to_owned(), i32::from(quantity), percent))
    .collect()
}

async fn claim_count(clients: &SeamClients<'_>, epoch: u64) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_daily_reward_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND reset_epoch=$3::text::numeric",
    ).bind(clients.characters[0].as_slice()).bind(clients.world.as_slice())
        .bind(epoch.to_string()).fetch_one(&mut connection).await?)
}

async fn delivery(clients: &SeamClients<'_>, data: &ClaimEvidence) -> TestResult<Vec<RewardLine>> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT l.kind, l.quantity, CASE WHEN l.kind = 'XP_BOOST' THEN l.xp_boost_percent END
         FROM game_character_daily_reward_claims c
         JOIN game_character_daily_reward_lines l ON l.claim_id=c.transaction_id
         WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid
           AND c.reset_epoch=$3::text::numeric AND c.transaction_id=encode($4,'hex')::uuid
           AND c.lane_day=$5 AND l.status='DELIVERED' AND l.delivering_transaction_id=c.transaction_id
         ORDER BY l.kind LIMIT 6",
    ).bind(clients.characters[0].as_slice()).bind(clients.world.as_slice())
        .bind(data.epoch.to_string()).bind(data.transaction.as_slice()).bind(i16::from(data.day))
        .fetch_all(&mut connection).await?)
}

async fn balances(clients: &SeamClients<'_>) -> TestResult<(i64, i64)> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_as(
        "SELECT COALESCE((SELECT balance FROM game_character_cosmetic_tokens
                          WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid), 0),
                COALESCE((SELECT balance FROM game_character_bounty_reroll_tokens
                          WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid), 0)",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .fetch_one(&mut connection).await?)
}

async fn claim_tier(clients: &SeamClients<'_>, transaction: &[u8; 16]) -> TestResult<bool> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT premium FROM game_character_daily_reward_claims WHERE transaction_id=encode($1,'hex')::uuid",
    ).bind(transaction.as_slice()).fetch_one(&mut connection).await?)
}

fn claim_result(payload: &[u8]) -> TestResult<daily::DailyRewardResult> {
    use oteryn_dev_client::client_runtime::daily_reward::{
        DailyRewardCommand, DailyRewardReply, decode_reply,
    };
    match decode_reply(DailyRewardCommand::Claim, payload)? {
        DailyRewardReply::Result(value) => Ok(value),
        observed => Err(format!("step=claim observed={observed:?}").into()),
    }
}

fn shrine_tile(world: &BundleWorld, start: TilePos) -> TestResult<TilePos> {
    let content = crate::content::daily_reward::DailyRewardContent::catalogue()
        .ok_or("step=shrine observed=daily-catalogue-unavailable")?;
    let index = provisioning::item_index()?;
    let refs = content
        .shrine_source_ids
        .iter()
        .map(|id| provisioning::item_reference(&index, *id))
        .collect::<TestResult<Vec<_>>>()?;
    let facts = world.facts().pinned();
    for (floor, x, y, tile_data) in world.base().tiles() {
        let at = TilePos { floor, x, y };
        if start.x.abs_diff(x) > 160 || start.y.abs_diff(y) > 160 {
            continue;
        }
        let shrine = tile_data.ids().iter().zip(tile_data.depths())
            .filter(|(_, depth)| **depth == 0).enumerate().any(|(ordinal, (id, _))| {
                u8::try_from(ordinal).ok().and_then(|n| facts.base_entry(at, n, *id))
                    .is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(reference) if refs.contains(&reference)))
            });
        if shrine {
            return Ok(at);
        }
    }
    Err("step=shrine observed=no-daily-shrine-in-town-window".into())
}

async fn walk_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("G16 invalid route step")?;
        let reply = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if reply.disposition != StepDisposition::Moved
            || reply.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!(
                "step=walk observed={:?} from={from:?} expected={expected:?} actual={actual:?}",
                reply.disposition
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
        if reply.step_wait_ms > 0 {
            tokio::time::sleep(Duration::from_millis(u64::from(reply.step_wait_ms))).await;
        }
    }
    Ok(())
}

async fn body(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<(ClaimEvidence, (daily::DailyRewardWall, details::Details))> {
    let table = accepted(client, CAPABILITY, details::COMMAND_TYPE, &[]).await?;
    let table = details::Details::decode(&table)?;
    let advertised = table.days[usize::from(DAY - 1)].free;
    if advertised != FREE_TIER {
        return Err(format!(
            "step=catalogue day={DAY} tier=free observed={advertised:?} expected={FREE_TIER:?}"
        )
        .into());
    }
    let initial = wall(client).await?;
    if initial.claimed
        || initial.claim_enabled
        || initial.lane_day != DAY
        || initial.streak != 0
        || table.history.iter().any(Option::is_some)
    {
        return Err(format!(
            "step=wall observed={initial:?}-epoch:{} needed=unclaimed-unopened-day",
            table.epoch
        )
        .into());
    }
    let start = tile(client.world_spatial().actor_position)?;
    let shrine = shrine_tile(world, start)?;
    let path = route(world, start, |at| {
        at.floor == shrine.floor && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
    })
    .ok_or("step=shrine observed=no-route-to-selected-shrine")?;
    walk_once(client, world, &path).await?;
    let opened = client
        .session_mut()
        .use_object(b"oteryn:placement/daily-reward-shrine", 0)
        .await?;
    if opened.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=shrine observed={opened:?}").into());
    }
    let ready = wall(client).await?;
    if !ready.claim_enabled || ready.claimed || ready.lane_day != DAY {
        return Err(format!("step=shrine observed=wall:{ready:?}").into());
    }
    if claim_count(clients, table.epoch).await? != 0 || balances(clients).await? != (0, 0) {
        return Err("step=claim observed=preexisting-epoch-receipt".into());
    }
    let reply = accepted(
        client,
        CAPABILITY,
        daily::COMMAND_TYPE_DAILY_REWARD_CLAIM,
        &[],
    )
    .await?;
    let result = claim_result(&reply)?;
    if result.disposition != 1 {
        return Err(format!("step=claim observed={result:?} needed=new-disposition").into());
    }
    let transaction = result
        .transaction_id
        .ok_or("step=claim observed=no-transaction")?;
    let data = ClaimEvidence {
        transaction,
        epoch: table.epoch,
        day: DAY,
    };
    let claimed = verify(client, clients, &data).await?;
    let reply = accepted(
        client,
        CAPABILITY,
        daily::COMMAND_TYPE_DAILY_REWARD_CLAIM,
        &[],
    )
    .await?;
    let duplicate = claim_result(&reply)?;
    if duplicate.disposition != 2 || duplicate.transaction_id.is_some() {
        return Err(format!("step=duplicate observed={duplicate:?}").into());
    }
    let after_duplicate = verify(client, clients, &data).await?;
    if after_duplicate != claimed {
        return Err("step=duplicate observed=reward-state-changed".into());
    }
    Ok((data, claimed))
}

fn matches_free_delivery(lines: &[RewardLine], premium: bool, balances: (i64, i64)) -> bool {
    let expected = expected_lines(FREE_TIER);
    !expected.is_empty()
        && lines == expected
        && !premium
        && balances
            == (
                i64::from(FREE_TIER.cosmetic_tokens),
                i64::from(FREE_TIER.bounty_tokens),
            )
}

async fn verify(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    data: &ClaimEvidence,
) -> TestResult<(daily::DailyRewardWall, details::Details)> {
    let wall = wall(client).await?;
    let table =
        details::Details::decode(&accepted(client, CAPABILITY, details::COMMAND_TYPE, &[]).await?)?;
    let history_matches = table
        .history
        .iter()
        .flatten()
        .filter(|receipt| {
            receipt.epoch == data.epoch
                && receipt.lane_day == data.day
                && receipt.streak == 1
                && receipt.jokers_spent == 0
                && receipt.own_character
        })
        .count()
        == 1;
    let lines = delivery(clients, data).await?;
    let expected = expected_lines(FREE_TIER);
    let delivered = balances(clients).await?;
    let premium = claim_tier(clients, &data.transaction).await?;
    if claim_count(clients, data.epoch).await? != 1
        || !history_matches
        || table.epoch != data.epoch
        || wall.lane_day != DAY + 1
        || wall.streak != 1
        || !wall.claimed
        || wall.claim_enabled
        || !matches_free_delivery(&lines, premium, delivered)
    {
        return Err(format!("step=verify observed=wall:{wall:?},history:{history_matches},lines:{lines:?},premium:{premium},balances:{delivered:?} needed={expected:?}").into());
    }
    evidence("G16_CHECK pg=receipt-lines-and-delivered-balances status=PASS");
    Ok((wall, table))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_G16_ACCOUNT").await?;
    let clients = &fixture.clients(clients);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x7a, 0).await?;
    let session_id = client.join_snapshot().game_session_id;
    let outcome = verdict::observe(body(&mut client, clients, world)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let (data, claimed) = outcome?;
    closed?;
    let mut fresh = connect_character(clients, 0x7b, 0).await?;
    let checked = verdict::observe(verify(&mut fresh, clients, &data)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    let persisted = checked?;
    closed?;
    if fresh.join_snapshot().game_session_id == session_id || persisted != claimed {
        return Err("step=relog observed=session-not-fresh-or-reward-state-changed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_free_claim_requires_both_balances_and_the_free_receipt() {
        let lines = expected_lines(FREE_TIER);
        assert!(matches_free_delivery(&lines, false, (2, 0)));
        assert!(!matches_free_delivery(&lines, false, (2, 1)));
        assert!(!matches_free_delivery(&lines, false, (0, 0)));
        assert!(!matches_free_delivery(&lines, false, (2, 2)));
        assert!(!matches_free_delivery(&lines, true, (2, 0)));
        let premium = expected_lines(details::Tier {
            cosmetic_tokens: 4,
            ..FREE_TIER
        });
        assert!(!matches_free_delivery(&premium, true, (4, 0)));
        assert!(!matches_free_delivery(&[], false, (2, 0)));
    }

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn claim_result_uses_tagged_shared_payload_and_transaction_provenance() -> TestResult {
        let mut id = [1; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let claimed = daily::DailyRewardResult {
            disposition: 1,
            transaction_id: Some(id),
        };
        let payload = claimed
            .encode()
            .map_err(|error| format!("claim encode: {error:?}"))?;
        assert_eq!(payload.first(), Some(&0x08));
        assert_eq!(claim_result(&payload)?, claimed);
        let duplicate = daily::DailyRewardResult {
            disposition: 2,
            transaction_id: None,
        };
        let payload = duplicate
            .encode()
            .map_err(|error| format!("duplicate encode: {error:?}"))?;
        assert_eq!(claim_result(&payload)?, duplicate);
        assert!(claim_result(&[1]).is_err());
        assert!(claim_result(&[0x08, 1]).is_err());
        Ok(())
    }
}
