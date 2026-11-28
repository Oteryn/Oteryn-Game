//! G16 daily reward details, picker and pick claim (capability 182, commands 738, 6122, 6123).
//! 1. Admit the node-boot character; read the 30-day details (738) and the daily wall.
//! 2. Walk to the production daily shrine and USE it so the claim is enabled.
//! 3. Query the picker (6122): on a pick day it offers items and a maximum count.
//! 4. Claim a pick (6123); the reply must be the "new" disposition and the character's durable
//!    claim-bound Inbox/pick quantity (read-only SQL) must equal the picked count.
//! 5. Claim again: duplicate disposition, no further delivery. Log out, freshly admit and
//!    re-check the same transaction, Inbox/pick proof and exhausted picker.
//!
//! The row fails when the day was already claimed (it shares the character with P09) or when the
//! current lane day is not a pick day; it never alters server state outside player commands.

use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_reward::{self as daily, details, picker};
use oteryn_protocol_oteryn::world_map::MapDefinition;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "daily-pick-claimed-delivered-and-durable".to_owned(),
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
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
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

/// Read-only evidence of the receipt and every selected output of this exact claim.
#[derive(Debug)]
struct ClaimEvidence {
    transaction: [u8; 16],
    epoch: u64,
    day: u8,
    item: u32,
    count: u8,
    proof: (i64, i64, i64, i64),
}

async fn claim_count(clients: &SeamClients<'_>, epoch: u64) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_daily_reward_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND reset_epoch=$3::text::numeric",
    ).bind(clients.characters[0].as_slice()).bind(clients.world.as_slice())
        .bind(epoch.to_string()).fetch_one(&mut connection).await?)
}

async fn delivery(
    clients: &SeamClients<'_>,
    data: &ClaimEvidence,
) -> TestResult<(i64, i64, i64, i64)> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    // The epoch receipt binds the returned transaction to this character/world/day.
    // Picks bind each selected instance; custody, lifecycle and mint provenance must all agree.
    Ok(sqlx::query_as(
        "SELECT COALESCE(sum(p.quantity),0)::bigint, \
         COALESCE(sum(i.quantity) FILTER (WHERE i.lifecycle=1 AND i.minted_transaction_id=c.transaction_id AND b.character_id=c.character_id AND l.character_id=c.character_id),0)::bigint, \
         (SELECT count(*) FROM game_daily_reward_picks outputs WHERE outputs.claim_id=encode($4,'hex')::uuid), \
         (SELECT COALESCE(sum(minted.quantity),0)::bigint FROM game_daily_reward_picks outputs JOIN game_item_instances minted USING(item_instance_id) WHERE outputs.claim_id=encode($4,'hex')::uuid) \
         FROM game_character_daily_reward_claims c \
         JOIN game_daily_reward_picks p ON p.claim_id=c.transaction_id AND p.source_id=$6 \
         LEFT JOIN game_item_instances i ON i.item_instance_id=p.item_instance_id AND i.world_id=c.world_id AND i.definition_production_key=$7 \
         LEFT JOIN game_item_bindings b ON b.item_instance_id=i.item_instance_id AND b.world_id=c.world_id \
         LEFT JOIN game_item_character_inbox_locations l ON l.item_instance_id=i.item_instance_id AND l.world_id=c.world_id \
         WHERE c.character_id=encode($1,'hex')::uuid AND c.world_id=encode($2,'hex')::uuid AND c.reset_epoch=$3::text::numeric AND c.transaction_id=encode($4,'hex')::uuid AND c.lane_day=$5",
    ).bind(clients.characters[0].as_slice()).bind(clients.world.as_slice())
        .bind(data.epoch.to_string()).bind(data.transaction.as_slice()).bind(i16::from(data.day))
        .bind(i64::from(data.item)).bind(format!("oteryn:item.tibia.i{}", data.item))
        .fetch_one(&mut connection).await?)
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
        if shrine
            && route(world, start, |candidate| {
                candidate.floor == at.floor
                    && candidate.x.abs_diff(at.x).max(candidate.y.abs_diff(at.y)) <= 1
            })
            .is_some()
        {
            return Ok(at);
        }
    }
    Err("step=shrine observed=no-reachable-daily-shrine-in-town-window".into())
}

async fn body(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<ClaimEvidence> {
    let table = accepted(client, CAPABILITY, details::COMMAND_TYPE, &[]).await?;
    let table = details::Details::decode(&table)?;
    let initial = wall(client).await?;
    if initial.claimed || initial.claim_enabled {
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
    .ok_or("step=shrine observed=walking-route-disappeared")?;
    walk(client, world, &path).await?;
    let opened = client
        .session_mut()
        .use_object(b"oteryn:placement/daily-reward-shrine", 0)
        .await?;
    if opened.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("step=shrine observed={opened:?}").into());
    }
    let ready = wall(client).await?;
    if !ready.claim_enabled || ready.claimed {
        return Err(format!("step=shrine observed=wall:{ready:?}").into());
    }
    let offer = accepted(client, CAPABILITY, picker::QUERY, &[]).await?;
    let offer = picker::Offer::decode(&offer)?;
    if offer.maximum == 0 {
        return Err(format!(
            "step=picker observed=maximum-0-day:{}-lane:{} needed=pick-day-reward",
            offer.day, ready.lane_day
        )
        .into());
    }
    let item = offer
        .items
        .iter()
        .copied()
        .find(|item| *item != 0)
        .ok_or("step=picker observed=no-offered-item")?;
    let count = offer.maximum;
    if claim_count(clients, offer.epoch).await? != 0 {
        return Err(
            "step=claim observed=preexisting-epoch-receipt needed=fresh-entitlement".into(),
        );
    }
    let claim = picker::Claim {
        epoch: offer.epoch,
        day: offer.day,
        picks: vec![picker::Pick { item, count }],
    }
    .encode()?;
    let reply = accepted(client, CAPABILITY, picker::CLAIM, &claim).await?;
    let result = claim_result(&reply)?;
    if result.disposition != 1 {
        return Err(format!("step=claim observed={result:?} needed=new-disposition").into());
    }
    let mut data = ClaimEvidence {
        transaction: result
            .transaction_id
            .ok_or("step=claim observed=no-transaction")?,
        epoch: offer.epoch,
        day: offer.day,
        item,
        count,
        proof: (0, 0, 0, 0),
    };
    data.proof = delivery(clients, &data).await?;
    if claim_count(clients, data.epoch).await? != 1
        || data.proof.0 != i64::from(count)
        || data.proof.1 != i64::from(count)
    {
        return Err(format!(
            "step=claim observed=proof:{:?} needed=claim-bound-pick-and-Inbox-quantity-{count}",
            data.proof
        )
        .into());
    }
    if !wall(client).await?.claimed {
        return Err("step=claim observed=wall-not-claimed".into());
    }
    let reply = accepted(client, CAPABILITY, picker::CLAIM, &claim).await?;
    let duplicate = claim_result(&reply)?;
    if duplicate.disposition != 2
        || duplicate.transaction_id.is_some()
        || claim_count(clients, data.epoch).await? != 1
        || delivery(clients, &data).await? != data.proof
    {
        return Err(
            format!("step=duplicate observed={duplicate:?}-or-claim-Inbox-proof-changed").into(),
        );
    }
    Ok(data)
}

async fn verify(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    data: &ClaimEvidence,
) -> TestResult {
    if claim_count(clients, data.epoch).await? != 1
        || delivery(clients, data).await? != data.proof
        || data.proof.0 != i64::from(data.count)
        || data.proof.1 != i64::from(data.count)
        || !wall(client).await?.claimed
    {
        return Err("step=relog observed=claim-Inbox-proof-or-wall-lost".into());
    }
    let offer = accepted(client, CAPABILITY, picker::QUERY, &[]).await?;
    if picker::Offer::decode(&offer)?.maximum != 0 {
        return Err("step=relog observed=picker-still-offers".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x7a, 0).await?;
    let outcome = verdict::observe(body(&mut client, clients, world)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let data = outcome?;
    closed?;
    let mut fresh = connect_character(clients, 0x7b, 0).await?;
    let checked = verdict::observe(verify(&mut fresh, clients, &data)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    checked?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

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
