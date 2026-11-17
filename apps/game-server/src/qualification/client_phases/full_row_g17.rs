//! G17 returner reward query and claim (capability 182, commands 6124 and 6125).
//! 1. Admit the node-boot character; query the returner state (6124).
//! 2. With no entitlement (state 0) a claim for an unknown identity must be refused and leave
//!    the inventory and the state unchanged; the row then fails naming the precondition,
//!    because an entitlement needs twelve months of inactivity evidence no player route makes.
//! 3. With an entitlement (state 1) claim its identity: the reply must be disposition 1 carrying
//!    that identity, the same session then reads state 2, a second claim is disposition 2, and
//!    the durable claim receipt joined to its minted item instance must exist (read-only SQL).
//!    The reward is delivered to the reward inbox, not the backpack, so the inventory is not the
//!    evidence. After logout and a fresh admission the receipt must still exist and the
//!    entitlement must not be offered again (state 2 is tied to the source session).

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_reward::{self as daily, returner};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "returner-reward-claimed-receipt-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G17.full status={status} seconds={:.3} evidence={detail}",
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
const BOGUS: [u8; 16] = [
    0x01, 0x93, 0x5f, 0x3b, 0x2c, 0x4d, 0x7a, 0x11, 0x8b, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
];

async fn state(client: &mut DevClientSession) -> TestResult<returner::Reply> {
    let payload = accepted(client, CAPABILITY, returner::QUERY, &[]).await?;
    Ok(returner::Reply::decode(&payload)?)
}

/// Claim reply: field 1 = disposition, field 2 (when claimed) = the 16-byte transaction id.
fn claim_reply_matches(payload: &[u8], disposition: u8, id: Option<[u8; 16]>) -> bool {
    let mut expected = vec![0x08, disposition];
    if let Some(id) = id {
        expected.extend_from_slice(&[0x12, 16]);
        expected.extend_from_slice(&id);
    }
    payload == expected.as_slice()
}

fn delivery_matches(bound: i64, minted: i64, inbox_delta: i64) -> bool {
    (bound, minted, inbox_delta) == (1, 1, 1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Delivery {
    item: Option<Vec<u8>>,
    minted: i64,
    inbox: i64,
}

async fn delivery(clients: &SeamClients<'_>, offer: &[u8; 16]) -> TestResult<Delivery> {
    use sqlx::Connection;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let (item, minted, inbox) = sqlx::query_as(
        "SELECT (SELECT uuid_send(i.item_instance_id)
                  FROM game_returner_reward_claims c
                  JOIN game_returner_reward_entitlements e USING(offer_id)
                  JOIN game_item_instances i ON (i.item_instance_id,i.world_id)=(c.item_instance_id,e.world_id)
                  JOIN game_item_character_inbox_locations l USING(item_instance_id,world_id)
                  WHERE c.offer_id=encode($3,'hex')::uuid AND e.character_id=encode($1,'hex')::uuid
                    AND e.world_id=encode($2,'hex')::uuid AND l.character_id=e.character_id
                    AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i266'
                    AND i.definition_revision_ref='definition-r1' AND i.quantity=1 AND i.lifecycle=1
                    AND i.minted_transaction_id=e.offer_id),
                (SELECT count(*) FROM game_item_instances i
                  WHERE i.world_id=encode($2,'hex')::uuid AND i.minted_transaction_id=encode($3,'hex')::uuid),
                (SELECT coalesce(sum(i.quantity),0)::bigint
                  FROM game_item_character_inbox_locations l JOIN game_item_instances i USING(item_instance_id,world_id)
                  WHERE l.character_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid
                    AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i266'
                    AND i.lifecycle=1)",
    ).bind(clients.characters[1].as_slice()).bind(clients.world.as_slice()).bind(offer.as_slice())
        .fetch_one(&mut database).await?;
    Ok(Delivery { item, minted, inbox })
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult<([u8; 16], Delivery)> {
    let initial = state(client).await?;
    let inventory = client.inventory().cloned();
    match (initial.state, initial.id) {
        (1, Some(id)) => {
            let before = delivery(clients, &id).await?;
            if before.item.is_some() || before.minted != 0 {
                return Err(format!("step=claim observed=preexisting-delivery:{before:?}").into());
            }
            let reply = accepted(client, CAPABILITY, returner::CLAIM, &id).await?;
            if !claim_reply_matches(&reply, 1, Some(id)) {
                return Err(format!("step=claim observed=reply:{reply:?}").into());
            }
            let after = state(client).await?;
            if after.state != 2 || after.id != Some(id) {
                return Err(format!("step=claim observed=state:{}", after.state).into());
            }
            let delivered = delivery(clients, &id).await?;
            if !delivery_matches(i64::from(delivered.item.is_some()), delivered.minted, delivered.inbox - before.inbox) {
                return Err(format!("step=claim observed=before:{before:?},after:{delivered:?};needed=one-exact-Inbox-mint").into());
            }
            let replay = accepted(client, CAPABILITY, returner::CLAIM, &id).await?;
            if !claim_reply_matches(&replay, 2, None) {
                return Err(format!("step=claim-replay observed=reply:{replay:?}").into());
            }
            let replayed = delivery(clients, &id).await?;
            if replayed != delivered || client.inventory().cloned() != inventory {
                return Err(format!("step=replay observed=changed-delivery:{replayed:?};needed={delivered:?}").into());
            }
            Ok((id, delivered))
        }
        (0, _) => {
            let (status, _) = send(client, CAPABILITY, returner::CLAIM, &BOGUS).await?;
            let after = state(client).await?;
            if status == CommandStatus::Accepted
                || after.state != 0
                || client.inventory().cloned() != inventory
            {
                return Err(format!(
                    "step=bogus-claim observed=status:{status:?}-state:{} needed=refusal-without-change",
                    after.state
                )
                .into());
            }
            Err("step=query observed=state-0-no-entitlement needed=returner-entitlement-after-12-month-inactivity".into())
        }
        (other, _) => Err(format!(
            "step=query observed=state-{other} needed=fresh-returner-entitlement"
        )
        .into()),
    }
}

async fn verify(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    data: &([u8; 16], Delivery),
) -> TestResult {
    let (id, delivered) = data;
    let after = state(client).await?;
    if after.state == 1 || (after.state == 2 && after.id != Some(*id)) {
        return Err("step=relog observed=claimed-entitlement-offered-again".into());
    }
    let fresh = delivery(clients, id).await?;
    if fresh != *delivered {
        return Err(format!("step=relog observed=changed-delivery:{fresh:?};needed={delivered:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x7c).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let data = outcome?;
    closed?;
    let mut fresh = connect_world(clients, 0x7d).await?;
    let checked = verdict::observe(verify(&mut fresh, clients, &data)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    checked?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returner_delivery_rejects_extra_mints_and_missing_inbox_quantity() {
        assert!(!delivery_matches(1, 1, 0));
        assert!(!delivery_matches(1, 2, 1));
        assert!(delivery_matches(1, 1, 1));
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
    fn claim_reply_is_matched_exactly() {
        let mut claimed = vec![0x08, 1, 0x12, 16];
        claimed.extend_from_slice(&BOGUS);
        assert!(claim_reply_matches(&claimed, 1, Some(BOGUS)));
        assert!(!claim_reply_matches(&claimed, 2, None));
        assert!(claim_reply_matches(&[0x08, 2], 2, None));
        assert!(!claim_reply_matches(&[0x08, 3], 1, Some(BOGUS)));
    }

    #[test]
    fn bogus_identity_is_a_uuid_v7() {
        assert_eq!(BOGUS[6] >> 4, 7);
        assert_eq!(BOGUS[8] >> 6, 2);
        assert_eq!(returner::decode_claim(&BOGUS), Ok(BOGUS));
    }
}
