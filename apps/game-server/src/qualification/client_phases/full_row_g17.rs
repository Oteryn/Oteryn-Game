//! G17 returner reward query and claim (capability 182, commands 6124 and 6125).
//! 1. Admit a dedicated character with native, qualifying inactivity history.
//! 2. With no entitlement (state 0) a claim for an unknown identity must be refused and leave
//!    the inventory and the state unchanged; the row then fails naming the precondition,
//!    because an entitlement needs twelve months of inactivity evidence no player route makes.
//! 3. With an entitlement (state 1) claim its identity: the reply must be disposition 1 carrying
//!    that identity, the same session then reads state 2, a second claim is disposition 2, and
//!    the durable claim receipt joined to its minted item instance must exist (read-only SQL).
//!    The reward is delivered to the reward inbox, not the backpack, so the inventory is not the
//!    evidence. After logout and a fresh admission, require state 0 and the same exact
//!    reward item instance in the recipient’s inbox (state 2 is tied to the source session).

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_reward::{self as daily, returner};
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
    let outcome = client.session_mut().logout().await?;
    evidence(&format!("G17_CHECK step=logout attempt=1 result={:?}", outcome.result));
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={:?}", outcome.result).into());
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

struct ClaimedReward {
    offer: [u8; 16],
    item_instance: String,
    inbox: Vec<InboxItem>,
}

/// Read the unique claim and verify the exact item and its recipient's inbox location.
async fn claimed_item(clients: &SeamClients<'_>, offer: &[u8; 16]) -> TestResult<Option<String>> {
    use sqlx::Connection;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let claim: Option<(String, bool)> = sqlx::query_as(
        "SELECT c.item_instance_id::text,
                EXISTS (
                    SELECT 1 FROM game_item_instances i
                    JOIN game_item_character_inbox_locations l USING (item_instance_id, world_id)
                    WHERE i.item_instance_id = c.item_instance_id
                      AND i.minted_transaction_id = c.offer_id
                      AND i.definition_family = 'Item'
                      AND i.definition_production_key = 'oteryn:item.tibia.i266'
                      AND i.definition_revision_ref = 'definition-r1'
                      AND i.quantity = 1
                      AND l.character_id = e.character_id
                      AND l.world_id = e.world_id
                )
         FROM game_returner_reward_claims c
         JOIN game_returner_reward_entitlements e USING (offer_id)
         WHERE c.offer_id = encode($1, 'hex')::uuid",
    )
    .bind(offer.as_slice())
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    match claim {
        None => Ok(None),
        Some((id, true)) => Ok(Some(id)),
        Some((_, false)) => Err("step=claim observed=wrong-inbox-reward".into()),
    }
}

async fn body(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<ClaimedReward> {
    let before = inbox(clients).await?;
    let initial = state(client).await?;
    let inventory = client.inventory().cloned();
    match (initial.state, initial.id) {
        (1, Some(id)) => {
            let mut database = sqlx::PgConnection::connect(clients.url).await?;
            let source: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT uuid_send(source_session_id) FROM game_returner_reward_entitlements
                 WHERE offer_id = encode($1, 'hex')::uuid",
            )
            .bind(id.as_slice())
            .fetch_optional(&mut database)
            .await?;
            validate_offer_source(source.as_deref(), client.join_snapshot().game_session_id.as_bytes())?;
            if claimed_item(clients, &id).await?.is_some() {
                return Err("step=pre-claim observed=existing-claim".into());
            }
            let (status, reply) = send(client, CAPABILITY, returner::CLAIM, &BOGUS).await?;
            let still_available = state(client).await?;
            if status != CommandStatus::Rejected
                || !claim_reply_matches(&reply, 3, None)
                || still_available.state != 1
                || still_available.id != Some(id)
                || claimed_item(clients, &id).await?.is_some()
                || client.inventory().cloned() != inventory
                || inbox(clients).await? != before
            {
                return Err("step=bogus-claim observed=entitlement-or-inventory-changed".into());
            }
            evidence("G17_CHECK step=history-entitlement status=PASS");
            let (status, reply) = send(client, CAPABILITY, returner::CLAIM, &id).await?;
            if status != CommandStatus::Accepted {
                return Err(format!(
                    "step=returner-claim observed=status:{status:?},payload:{reply:?}"
                )
                .into());
            }
            if !claim_reply_matches(&reply, 1, Some(id)) {
                return Err(format!("step=claim observed=reply:{reply:?}").into());
            }
            let after = state(client).await?;
            if after.state != 2 || after.id != Some(id) {
                return Err(format!("step=claim observed=state:{}", after.state).into());
            }
            let item_instance = claimed_item(clients, &id)
                .await?
                .ok_or("step=claim observed=no-claim-receipt-with-item")?;
            let delivered = inbox(clients).await?;
            if !added_only_reward(&before, &delivered, &item_instance) {
                return Err("step=claim observed=unexpected-inbox-delta".into());
            }
            let replay = accepted(client, CAPABILITY, returner::CLAIM, &id).await?;
            if !claim_reply_matches(&replay, 2, None) {
                return Err(format!("step=claim-replay observed=reply:{replay:?}").into());
            }
            if claimed_item(clients, &id).await?.as_ref() != Some(&item_instance)
                || inbox(clients).await? != delivered {
                return Err("step=claim-replay observed=inbox-reward-changed".into());
            }
            Ok(ClaimedReward {
                offer: id,
                item_instance,
                inbox: delivered,
            })
        }
        (0, _) => {
            let (status, reply) = send(client, CAPABILITY, returner::CLAIM, &BOGUS).await?;
            let after = state(client).await?;
            if status != CommandStatus::Rejected
                || !claim_reply_matches(&reply, 3, None)
                || after.state != 0
                || after.id.is_some()
                || client.inventory().cloned() != inventory
                || inbox(clients).await? != before
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
    data: &ClaimedReward,
) -> TestResult {
    let after = state(client).await?;
    if after.state != 0 || after.id.is_some() {
        return Err("step=relog observed=unexpected-returner-state".into());
    }
    if claimed_item(clients, &data.offer).await?.as_ref() != Some(&data.item_instance)
        || inbox(clients).await? != data.inbox {
        return Err("step=relog observed=inbox-reward-changed".into());
    }
    Ok(())
}

pub(crate) fn validate_offer_source(
    source: Option<&[u8]>,
    session: &[u8; 16],
) -> TestResult {
    if source != Some(session.as_slice()) {
        return Err("step=query observed=offer-not-issued-for-this-admission".into());
    }
    Ok(())
}

type InboxItem = (String, String, String, String, i64, String);

async fn inbox(clients: &SeamClients<'_>) -> TestResult<Vec<InboxItem>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let rows = sqlx::query_as(
        "SELECT i.item_instance_id::text, i.definition_family, i.definition_production_key,
                i.definition_revision_ref, i.quantity, l.ordinal::text
         FROM game_item_character_inbox_locations l
         JOIN game_item_instances i USING (item_instance_id, world_id)
         WHERE l.character_id = encode($1, 'hex')::uuid
         ORDER BY l.ordinal LIMIT 1025",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_all(&mut database)
    .await?;
    if rows.len() == 1025 {
        return Err("step=inbox observed=snapshot-limit-exceeded".into());
    }
    Ok(rows)
}

fn added_only_reward(before: &[InboxItem], after: &[InboxItem], reward: &str) -> bool {
    after.len() == before.len() + 1
        && after.iter().filter(|item| item.0 == reward).count() == 1
        && after.iter().filter(|item| item.0 != reward).eq(before)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let accounts = [required("NODE_BOOT_G17_ACCOUNT")?];
    let characters = [full_row_g01::fixture_character(clients, &accounts[0]).await?];
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    let mut client = connect_character(clients, 0x7c, 0).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let data = outcome?;
    closed?;
    let mut fresh = connect_character(clients, 0x7d, 0).await?;
    let checked = verdict::observe(verify(&mut fresh, clients, &data)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    checked?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_delta_rejects_extra_and_replaced_items() {
        let item = |id: &str| (id.to_owned(), "Item".into(), "i266".into(), "r1".into(), 1, id.into());
        let existing = item("existing");
        let reward = item("reward");
        let extra = item("extra");
        assert!(added_only_reward(std::slice::from_ref(&existing), &[existing.clone(), reward.clone()], "reward"));
        assert!(!added_only_reward(std::slice::from_ref(&existing), &[existing.clone(), reward.clone(), extra.clone()], "reward"));
        assert!(!added_only_reward(std::slice::from_ref(&existing), &[extra, reward], "reward"));
        assert!(!added_only_reward(std::slice::from_ref(&existing), std::slice::from_ref(&existing), "reward"));
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
        assert!(claim_reply_matches(&[0x08, 3], 3, None));
        assert!(!claim_reply_matches(&[0x08, 3], 1, Some(BOGUS)));
    }

    #[test]
    fn bogus_identity_is_a_uuid_v7() {
        assert_eq!(BOGUS[6] >> 4, 7);
        assert_eq!(BOGUS[8] >> 6, 2);
        assert_eq!(returner::decode_claim(&BOGUS), Ok(BOGUS));
    }
}
